use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{PodDropError, Result};

/// Target audio format when transcoding unsupported formats for iPod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TranscodeFormat {
    /// AAC in M4A container (small, high compatibility).
    #[default]
    Aac,
    /// Apple Lossless in M4A container (lossless, larger).
    Alac,
    /// MP3 (universal compatibility).
    Mp3,
}

/// Extracted metadata from a local audio file via `ffprobe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAudioInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_ms: u32,
    pub bitrate_kbps: u32,
    pub samplerate_hz: u32,
    pub codec_name: String,
    pub has_embedded_art: bool,
}

impl LocalAudioInfo {
    /// Returns whether this audio file can be copied to an iPod without transcoding.
    pub fn is_ipod_native(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        matches!(
            (self.codec_name.as_str(), ext.as_str()),
            ("mp3", "mp3") | ("aac", "m4a" | "mp4") | ("alac", "m4a") | ("pcm_s16le", "wav")
        )
    }
}

/// Probes audio tags, stream info, and embedded artwork presence using `ffprobe`.
pub fn probe_audio(path: &Path) -> Result<LocalAudioInfo> {
    if !path.is_file() {
        return Err(PodDropError::Io(format!(
            "Audio file not found: {}",
            path.display()
        )));
    }

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration,bit_rate:format_tags=title,artist,album,track,disc:stream=codec_name,sample_rate,codec_type",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(path)
        .output()
        .map_err(|e| PodDropError::Io(format!("Failed to run ffprobe: {e}")))?;

    if !output.status.success() {
        return Err(PodDropError::Backend(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ffprobe_output(&text, path))
}

/// Strictly validates that an audio file is non-empty, readable, and contains a valid audio stream.
pub fn validate_audio_file(path: &Path) -> Result<(LocalAudioInfo, u64)> {
    let meta = fs::metadata(path)
        .map_err(|e| PodDropError::Io(format!("Cannot access {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(PodDropError::Io(format!(
            "Not a regular file: {}",
            path.display()
        )));
    }
    if meta.len() == 0 {
        return Err(PodDropError::Backend(format!(
            "Empty file (0 bytes): {}",
            path.display()
        )));
    }

    let info = probe_audio(path)?;
    if info.codec_name.is_empty() {
        return Err(PodDropError::Backend(format!(
            "No valid audio stream found: {}",
            path.display()
        )));
    }
    if info.duration_ms == 0 || info.samplerate_hz == 0 {
        return Err(PodDropError::Backend(format!(
            "Invalid audio duration or sample rate: {}",
            path.display()
        )));
    }

    Ok((info, meta.len()))
}

/// Extracts embedded cover art from an audio file into a JPEG/PNG image file.
pub fn extract_embedded_artwork(audio_path: &Path, out_image_path: &Path) -> Result<bool> {
    if let Some(parent) = out_image_path.parent() {
        fs::create_dir_all(parent).map_err(|e| PodDropError::Io(e.to_string()))?;
    }

    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(audio_path)
        .args(["-an", "-vcodec", "png", "-frames:v", "1"])
        .arg(out_image_path)
        .status()
        .map_err(|e| PodDropError::Io(format!("Failed to run ffmpeg: {e}")))?;

    if status.success() && out_image_path.is_file() {
        let len = fs::metadata(out_image_path).map(|m| m.len()).unwrap_or(0);
        if len > 0 {
            return Ok(true);
        }
    }
    let _ = fs::remove_file(out_image_path);
    Ok(false)
}

/// Transcodes an unsupported audio file (e.g., FLAC/OGG) into an iPod-compatible file.
pub fn transcode_for_ipod(
    src_path: &Path,
    out_dir: &Path,
    format: TranscodeFormat,
) -> Result<PathBuf> {
    fs::create_dir_all(out_dir).map_err(|e| PodDropError::Io(e.to_string()))?;
    let stem = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track");

    let (ext, codec_args): (&str, &[&str]) = match format {
        TranscodeFormat::Aac => ("m4a", &["-c:a", "aac", "-b:a", "256k"]),
        TranscodeFormat::Alac => ("m4a", &["-c:a", "alac"]),
        TranscodeFormat::Mp3 => ("mp3", &["-c:a", "libmp3lame", "-b:a", "256k"]),
    };

    let dest_path = out_dir.join(format!("{stem}.{ext}"));
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-y", "-i"])
        .arg(src_path)
        .args(["-vn"])
        .args(codec_args)
        .args(["-map_metadata", "0"])
        .arg(&dest_path);

    let output = cmd
        .output()
        .map_err(|e| PodDropError::Io(format!("Failed to run ffmpeg: {e}")))?;
    if !output.status.success() {
        return Err(PodDropError::Backend(format!(
            "ffmpeg transcode failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(dest_path)
}

fn parse_ffprobe_output(text: &str, path: &Path) -> LocalAudioInfo {
    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut track_number = None;
    let mut disc_number = None;
    let mut duration_ms = 0u32;
    let mut bitrate_kbps = 0u32;
    let mut samplerate_hz = 44100u32;
    let mut codec_name = String::new();
    let mut has_embedded_art = false;

    let mut current_codec_name = String::new();
    let mut current_sample_rate = None;

    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        let val = v.trim();

        match key.as_str() {
            "duration" => {
                if let Ok(sec) = val.parse::<f64>() {
                    if sec.is_finite() && sec >= 0.0 {
                        duration_ms = (sec * 1000.0).round() as u32;
                    }
                }
            }
            "bit_rate" => {
                if let Ok(bps) = val.parse::<u64>() {
                    bitrate_kbps = (bps / 1000) as u32;
                }
            }
            "tag:title" => title = val.to_string(),
            "tag:artist" => artist = val.to_string(),
            "tag:album" => album = val.to_string(),
            "tag:track" => track_number = parse_slash_number(val),
            "tag:disc" => disc_number = parse_slash_number(val),
            "codec_name" => current_codec_name = val.to_ascii_lowercase(),
            "sample_rate" => current_sample_rate = val.parse::<u32>().ok(),
            "codec_type" => {
                if val == "audio" && codec_name.is_empty() {
                    codec_name = current_codec_name.clone();
                    if let Some(sr) = current_sample_rate {
                        samplerate_hz = sr;
                    }
                } else if val == "video" {
                    has_embedded_art = true;
                }
            }
            _ => {}
        }
    }

    if title.is_empty() {
        title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Title")
            .to_string();
    }
    if artist.is_empty() {
        artist = "Unknown Artist".to_string();
    }
    if album.is_empty() {
        album = "Unknown Album".to_string();
    }

    LocalAudioInfo {
        title,
        artist,
        album,
        track_number,
        disc_number,
        duration_ms,
        bitrate_kbps,
        samplerate_hz,
        codec_name,
        has_embedded_art,
    }
}

fn parse_slash_number(val: &str) -> Option<u32> {
    val.split('/').next()?.trim().parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffprobe_kv_output() {
        let sample = "\
codec_name=mp3
codec_type=audio
sample_rate=44100
codec_name=mjpeg
codec_type=video
duration=183.456
bit_rate=320000
TAG:title=Sample Song
TAG:artist=Sample Artist
TAG:album=Sample Album
TAG:track=3/12
TAG:disc=1/2
";
        let info = parse_ffprobe_output(sample, Path::new("/tmp/fallback.mp3"));
        assert_eq!(info.title, "Sample Song");
        assert_eq!(info.artist, "Sample Artist");
        assert_eq!(info.album, "Sample Album");
        assert_eq!(info.track_number, Some(3));
        assert_eq!(info.disc_number, Some(1));
        assert_eq!(info.duration_ms, 183_456);
        assert_eq!(info.bitrate_kbps, 320);
        assert_eq!(info.samplerate_hz, 44100);
        assert_eq!(info.codec_name, "mp3");
        assert!(info.has_embedded_art);
        assert!(info.is_ipod_native(Path::new("song.mp3")));
    }

    #[test]
    fn validate_audio_rejects_empty_and_missing_files() {
        let missing = validate_audio_file(Path::new("/nonexistent/poddrop_missing.mp3"));
        assert!(missing.is_err());

        let tmp = std::env::temp_dir().join("poddrop_empty_test.mp3");
        let _ = fs::write(&tmp, b"");
        let empty_res = validate_audio_file(&tmp);
        let _ = fs::remove_file(&tmp);
        assert!(empty_res.is_err());
    }
}
