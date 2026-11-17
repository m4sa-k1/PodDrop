use std::path::{Path, PathBuf};

use crate::device::DeviceInfo;
use crate::error::Result;
use crate::media::TranscodeFormat;

/// Normalized track metadata read from or written to an iPod database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadata {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub duration_ms: u32,
    pub bitrate_kbps: u32,
    pub size_bytes: u32,
    pub has_artwork: bool,
}

/// Request payload for transferring a track and optional cover art to an iPod.
#[derive(Debug, Clone)]
pub struct TransferTrackRequest {
    pub audio_path: PathBuf,
    pub artwork_path: Option<PathBuf>,
    pub transcode_format: TranscodeFormat,
    pub title_override: Option<String>,
    pub artist_override: Option<String>,
    pub album_override: Option<String>,
}

/// Request payload for updating metadata of an existing track on an iPod.
#[derive(Debug, Clone, Default)]
pub struct UpdateTrackRequest {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub artwork_path: Option<PathBuf>,
    pub clear_artwork: bool,
}

/// Result of transferring a track to an iPod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferTrackResult {
    pub track_id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub transcoded: bool,
    pub artwork_registered: bool,
    pub backup_path: Option<PathBuf>,
}

/// Abstract backend interface for interacting with iPod databases.
///
/// Decoupling the UI and core workflows behind this trait allows supporting
/// `libgpod` initially while leaving room for a pure-Rust or newer backend later.
pub trait IpodBackend {
    fn inspect_device(&self, mount_path: &Path) -> Result<DeviceInfo>;
    fn list_tracks(&self, mount_path: &Path) -> Result<Vec<TrackMetadata>>;
    fn transfer_track(
        &self,
        mount_path: &Path,
        request: &TransferTrackRequest,
        backup_root: Option<&Path>,
    ) -> Result<TransferTrackResult>;
    fn update_track(
        &self,
        mount_path: &Path,
        track_id: u64,
        request: &UpdateTrackRequest,
    ) -> Result<()>;
    fn remove_track(&self, mount_path: &Path, track_id: u64) -> Result<()>;
}
