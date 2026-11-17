use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{PodDropError, Result};

/// Summary of a stored iPod metadata backup.
#[derive(Debug, Clone)]
pub struct BackupSummary {
    pub path: PathBuf,
    pub device_key: String,
    pub created_at: String,
    pub size_bytes: u64,
}

/// Derives a filesystem-safe identifier for the iPod at `mount_path`
/// (`FirewireGuid` from `SysInfo` / `SysInfoExtended`, falling back to mount name).
pub fn detect_device_key(mount_path: &Path) -> String {
    let device_dir = mount_path.join("iPod_Control").join("Device");

    if let Ok(sysinfo) = fs::read_to_string(device_dir.join("SysInfo")) {
        for line in sysinfo.lines() {
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("FirewireGuid") {
                    let guid = sanitize_key(v.trim().trim_start_matches("0x"));
                    if !guid.is_empty() {
                        return format!("ipod_{guid}");
                    }
                }
            }
        }
    }

    if let Ok(ext) = fs::read_to_string(device_dir.join("SysInfoExtended")) {
        if let Some(idx) = ext.find("FireWireGUID") {
            let tail = &ext[idx..];
            if let (Some(s), Some(e)) = (tail.find("<string>"), tail.find("</string>")) {
                if s + 8 < e {
                    let guid = sanitize_key(tail[s + 8..e].trim().trim_start_matches("0x"));
                    if !guid.is_empty() {
                        return format!("ipod_{guid}");
                    }
                }
            }
        }
    }

    let fallback = mount_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(sanitize_key)
        .unwrap_or_default();
    if fallback.is_empty() {
        "default_ipod".to_string()
    } else {
        format!("ipod_{fallback}")
    }
}

/// Creates a backup of `iPod_Control/{Device,iTunes,Artwork}` under
/// `<backup_root>/<device_key>/backup_YYYY-MM-DD_HH-MM-SS`.
///
/// After the new backup succeeds, any older `backup_*` directories for the
/// **same device** are automatically removed to save disk space, while backups
/// for other devices remain untouched.
pub fn create_prewrite_backup(mount_path: &Path, backup_root: &Path) -> Result<PathBuf> {
    let ipod_control = mount_path.join("iPod_Control");
    if !ipod_control.is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }

    let device_key = detect_device_key(mount_path);
    let device_backup_root = backup_root.join(&device_key);
    fs::create_dir_all(&device_backup_root).map_err(|e| PodDropError::Io(e.to_string()))?;

    let stamp = current_local_stamp();
    let mut dest_root = device_backup_root.join(format!("backup_{stamp}"));
    let mut suffix = 1u32;
    while dest_root.exists() {
        dest_root = device_backup_root.join(format!("backup_{stamp}_{suffix}"));
        suffix += 1;
    }

    let dest_control = dest_root.join("iPod_Control");
    fs::create_dir_all(&dest_control).map_err(|e| PodDropError::Io(e.to_string()))?;

    for sub in ["Device", "iTunes", "Artwork"] {
        let src = ipod_control.join(sub);
        if src.is_dir() {
            copy_dir_recursive(&src, &dest_control.join(sub))?;
        }
    }

    // Remove older backups for THIS device only after the new backup succeeds.
    if let Ok(entries) = fs::read_dir(&device_backup_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p == dest_root || !p.is_dir() {
                continue;
            }
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("backup_") {
                let _ = fs::remove_dir_all(&p);
            }
        }
    }

    Ok(dest_root)
}

/// Returns the most recent valid backup for a specific device (or legacy root).
pub fn latest_backup_for_device(mount_path: &Path, backup_root: &Path) -> Option<BackupSummary> {
    let device_key = detect_device_key(mount_path);
    let device_dir = backup_root.join(&device_key);
    latest_backup_in_dir(&device_dir, &device_key)
        .or_else(|| latest_backup_in_dir(backup_root, "legacy"))
}

/// Returns the most recent valid backup across all backed-up devices under `backup_root`.
pub fn latest_backup(backup_root: &Path) -> Option<BackupSummary> {
    let entries = fs::read_dir(backup_root).ok()?;
    let mut best: Option<(SystemTime, BackupSummary)> = None;

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let candidate = if name.starts_with("backup_") && p.join("iPod_Control").is_dir() {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            Some((
                modified,
                BackupSummary {
                    size_bytes: dir_size_bytes(&p),
                    path: p,
                    device_key: "legacy".to_string(),
                    created_at: format_backup_name(&name),
                },
            ))
        } else if let Some(summary) = latest_backup_in_dir(&p, &name) {
            let modified = fs::metadata(&summary.path)
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            Some((modified, summary))
        } else {
            None
        };

        if let Some((mod_time, summary)) = candidate {
            let replace = match &best {
                None => true,
                Some((best_mod, best_sum)) => {
                    (mod_time, &summary.path) > (*best_mod, &best_sum.path)
                }
            };
            if replace {
                best = Some((mod_time, summary));
            }
        }
    }

    best.map(|(_, s)| s)
}

/// Counts how many distinct devices currently have a valid backup under `backup_root`.
pub fn backed_up_device_count(backup_root: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(backup_root) else {
        return 0;
    };
    let mut count = 0u32;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("backup_") && latest_backup_in_dir(&p, &name).is_some() {
            count += 1;
        }
    }
    count
}

/// Restores `iPod_Control/{Device,iTunes,Artwork}` from the latest backup for the connected device.
pub fn restore_latest_backup(mount_path: &Path, backup_root: &Path) -> Result<BackupSummary> {
    let ipod_control = mount_path.join("iPod_Control");
    if !ipod_control.is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }

    let summary = latest_backup_for_device(mount_path, backup_root)
        .ok_or_else(|| PodDropError::Io("No backup found for this device.".to_string()))?;
    let src_control = summary.path.join("iPod_Control");

    for sub in ["Device", "iTunes", "Artwork"] {
        let src_sub = src_control.join(sub);
        let dst_sub = ipod_control.join(sub);
        if src_sub.is_dir() {
            if dst_sub.exists() {
                fs::remove_dir_all(&dst_sub).map_err(|e| PodDropError::Io(e.to_string()))?;
            }
            copy_dir_recursive(&src_sub, &dst_sub)?;
        }
    }

    let _ = Command::new("sync").status();
    Ok(summary)
}

/// Returns the default backup directory under the user's XDG data directory.
pub fn default_backup_root() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("poddrop").join("backups");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("poddrop")
                .join("backups");
        }
    }
    PathBuf::from("backups")
}

fn latest_backup_in_dir(dir: &Path, device_key: &str) -> Option<BackupSummary> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(SystemTime, PathBuf, String)> = None;

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() || !p.join("iPod_Control").is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("backup_") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        let replace = match &best {
            None => true,
            Some((best_mod, best_path, _)) => (modified, &p) > (*best_mod, best_path),
        };
        if replace {
            best = Some((modified, p, format_backup_name(&name)));
        }
    }

    let (_, path, created_at) = best?;
    let size_bytes = dir_size_bytes(&path);
    Some(BackupSummary {
        path,
        device_key: device_key.to_string(),
        created_at,
        size_bytes,
    })
}

fn sanitize_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

fn current_local_stamp() -> String {
    if let Ok(out) = Command::new("date").arg("+%Y-%m-%d_%H-%M-%S").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ts.to_string()
}

fn format_backup_name(dir_name: &str) -> String {
    let raw = dir_name.strip_prefix("backup_").unwrap_or(dir_name);
    if raw.len() >= 19 && raw.as_bytes().get(10) == Some(&b'_') {
        let date = &raw[0..10];
        let time = raw[11..19].replace('-', ":");
        return format!("{date} {time}");
    }
    raw.to_string()
}

fn dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    total += dir_size_bytes(&p);
                } else if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| PodDropError::Io(e.to_string()))?;
    for entry in fs::read_dir(src).map_err(|e| PodDropError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| PodDropError::Io(e.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| PodDropError::Io(e.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).map_err(|e| PodDropError::Io(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_isolates_multiple_devices_and_rotates_per_device() {
        let tmp = std::env::temp_dir().join("poddrop_test_multi_backup");
        let _ = fs::remove_dir_all(&tmp);
        let ipod_a = tmp.join("ipod_a");
        let ipod_b = tmp.join("ipod_b");
        let backup_root = tmp.join("backups");

        fs::create_dir_all(ipod_a.join("iPod_Control/iTunes")).unwrap();
        fs::create_dir_all(ipod_a.join("iPod_Control/Device")).unwrap();
        fs::write(
            ipod_a.join("iPod_Control/Device/SysInfo"),
            b"FirewireGuid: 0xAAAA1111\n",
        )
        .unwrap();
        fs::write(ipod_a.join("iPod_Control/iTunes/iTunesDB"), b"db_a1").unwrap();

        fs::create_dir_all(ipod_b.join("iPod_Control/iTunes")).unwrap();
        fs::create_dir_all(ipod_b.join("iPod_Control/Device")).unwrap();
        fs::write(
            ipod_b.join("iPod_Control/Device/SysInfo"),
            b"FirewireGuid: 0xBBBB2222\n",
        )
        .unwrap();
        fs::write(ipod_b.join("iPod_Control/iTunes/iTunesDB"), b"db_b1").unwrap();

        let bak_a1 = create_prewrite_backup(&ipod_a, &backup_root).unwrap();
        let bak_b1 = create_prewrite_backup(&ipod_b, &backup_root).unwrap();
        assert!(bak_a1.is_dir());
        assert!(bak_b1.is_dir());
        assert_eq!(backed_up_device_count(&backup_root), 2);

        // Second backup of A rotates A's first backup but leaves B untouched.
        fs::write(ipod_a.join("iPod_Control/iTunes/iTunesDB"), b"db_a2").unwrap();
        let bak_a2 = create_prewrite_backup(&ipod_a, &backup_root).unwrap();
        assert!(bak_a2.is_dir());
        assert!(!bak_a1.exists());
        assert!(bak_b1.is_dir());

        // Corrupt A and restore; A gets db_a2, B still has db_b1.
        fs::write(ipod_a.join("iPod_Control/iTunes/iTunesDB"), b"corrupt").unwrap();
        let restored = restore_latest_backup(&ipod_a, &backup_root).unwrap();
        assert_eq!(restored.path, bak_a2);
        assert_eq!(
            fs::read(ipod_a.join("iPod_Control/iTunes/iTunesDB")).unwrap(),
            b"db_a2"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn formats_date_backup_name() {
        assert_eq!(
            format_backup_name("backup_2026-11-17_22-17-25"),
            "2026-11-17 22:17:25"
        );
        assert_eq!(
            format_backup_name("backup_2026-11-17_22-17-25_1"),
            "2026-11-17 22:17:25"
        );
    }
}
