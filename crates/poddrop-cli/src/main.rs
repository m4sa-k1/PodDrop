use std::path::PathBuf;
use std::process::ExitCode;

use poddrop_core::backend::{IpodBackend, TransferTrackRequest};
use poddrop_core::backup::default_backup_root;
use poddrop_core::gpod::{read_sysinfo_extended_xml, LibgpodBackend};
use poddrop_core::i18n::{Language, Translator};
use poddrop_core::media::TranscodeFormat;
use poddrop_core::{APP_NAME, APP_VERSION};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut lang = Language::from_env();
    let mut show_version = false;
    let mut show_help = false;
    let mut positional: Vec<String> = Vec::new();
    let mut show_tracks = false;
    let mut artwork_path: Option<PathBuf> = None;
    let mut format = TranscodeFormat::Aac;
    let mut no_backup = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-V" => show_version = true,
            "--help" | "-h" => show_help = true,
            "--tracks" => show_tracks = true,
            "--no-backup" => no_backup = true,
            "--lang" => {
                if i + 1 < args.len() {
                    lang = Language::from_locale_str(&args[i + 1]);
                    i += 1;
                }
            }
            "--artwork" => {
                if i + 1 < args.len() {
                    artwork_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--format" => {
                if i + 1 < args.len() {
                    format = match args[i + 1].to_ascii_lowercase().as_str() {
                        "alac" => TranscodeFormat::Alac,
                        "mp3" => TranscodeFormat::Mp3,
                        _ => TranscodeFormat::Aac,
                    };
                    i += 1;
                }
            }
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    if show_version {
        println!("{APP_NAME} {APP_VERSION}");
        return ExitCode::SUCCESS;
    }

    let tr = Translator::new(lang);
    if show_help || positional.is_empty() {
        println!("{APP_NAME} {APP_VERSION} - {}", tr.t("app.tagline"));
        println!("{}", tr.t("cli.usage"));
        return ExitCode::SUCCESS;
    }

    match positional[0].as_str() {
        "inspect" => {
            let Some(path_str) = positional.get(1) else {
                eprintln!("{}", tr.t("cli.usage"));
                return ExitCode::FAILURE;
            };
            run_inspect(&PathBuf::from(path_str), show_tracks, &tr)
        }
        "read-sysinfo" => {
            let (Some(dev_str), Some(out_str)) = (positional.get(1), positional.get(2)) else {
                eprintln!("Usage: poddrop-cli read-sysinfo <block_device> <output_xml_path>");
                return ExitCode::FAILURE;
            };
            match read_sysinfo_extended_xml(&PathBuf::from(dev_str)) {
                Ok(xml) => match std::fs::write(out_str, &xml) {
                    Ok(()) => {
                        println!("Wrote SysInfoExtended ({} bytes) to {out_str}", xml.len());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("Failed to write {out_str}: {e}");
                        ExitCode::FAILURE
                    }
                },
                Err(e) => {
                    eprintln!("Error reading SysInfoExtended: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "add" => {
            let (Some(mount_str), Some(audio_str)) = (positional.get(1), positional.get(2)) else {
                eprintln!("Usage: poddrop-cli add <mount_path> <audio_file> [--artwork <img_path>] [--format aac|alac|mp3] [--no-backup]");
                return ExitCode::FAILURE;
            };
            let req = TransferTrackRequest {
                audio_path: PathBuf::from(audio_str),
                artwork_path,
                transcode_format: format,
                title_override: None,
                artist_override: None,
                album_override: None,
            };
            let backup_root = if no_backup {
                None
            } else {
                Some(default_backup_root())
            };
            let backend = LibgpodBackend::new();
            match backend.transfer_track(&PathBuf::from(mount_str), &req, backup_root.as_deref()) {
                Ok(res) => {
                    println!(
                        "Transferred track [{}]: {} - {} ({}) [transcoded: {}, artwork: {}]",
                        res.track_id,
                        res.artist,
                        res.title,
                        res.album,
                        res.transcoded,
                        res.artwork_registered
                    );
                    if let Some(bp) = res.backup_path {
                        println!("Pre-write backup saved to: {}", bp.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Transfer failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "remove" => {
            let (Some(mount_str), Some(id_str)) = (positional.get(1), positional.get(2)) else {
                eprintln!("Usage: poddrop-cli remove <mount_path> <track_id>");
                return ExitCode::FAILURE;
            };
            let Ok(track_id) = id_str.parse::<u64>() else {
                eprintln!("Invalid track_id: {id_str}");
                return ExitCode::FAILURE;
            };
            let backend = LibgpodBackend::new();
            match backend.remove_track(&PathBuf::from(mount_str), track_id) {
                Ok(()) => {
                    println!("Removed track [{track_id}]");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Remove failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("{}", tr.t("cli.usage"));
            ExitCode::FAILURE
        }
    }
}

fn run_inspect(path: &std::path::Path, show_tracks: bool, tr: &Translator) -> ExitCode {
    let backend = LibgpodBackend::new();
    match backend.inspect_device(path) {
        Ok(info) => {
            println!("=== {} ===", tr.t("inspect.header"));
            println!("{}: {}", tr.t("inspect.mount"), info.mount_path.display());
            println!("{}: {}", tr.t("inspect.model"), info.model.display_name());
            if let Some(num) = &info.model_number {
                println!("{}: {num}", tr.t("inspect.model_number"));
            }
            if let Some(cap) = info.capacity_gb {
                println!("{}: {cap:.1} GB", tr.t("inspect.capacity"));
            }
            println!(
                "{}: {}",
                tr.t("inspect.filesystem"),
                info.filesystem.as_str()
            );
            println!("{}: {}", tr.t("inspect.readonly"), info.is_read_only);
            println!(
                "{}: {}",
                tr.t("inspect.sysinfo_ext"),
                info.has_sysinfo_extended
            );
            println!(
                "{}: {}",
                tr.t("inspect.guid"),
                info.firewire_guid.as_deref().unwrap_or("(none)")
            );
            println!("{}: {}", tr.t("inspect.artwork"), info.supports_artwork);
            println!("{}: {}", tr.t("inspect.tracks"), info.track_count);
            println!("{}: {}", tr.t("inspect.playlists"), info.playlist_count);

            if show_tracks {
                match backend.list_tracks(path) {
                    Ok(tracks) => {
                        println!("\n--- Tracks ({}) ---", tracks.len());
                        for t in tracks.iter().take(20) {
                            println!(
                                "[{}] {} - {} ({}) [art: {}]",
                                t.id, t.artist, t.title, t.album, t.has_artwork
                            );
                        }
                        if tracks.len() > 20 {
                            println!("... and {} more", tracks.len() - 20);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to list tracks: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
