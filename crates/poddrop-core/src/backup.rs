use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{PodDropError, Result};

/// Summary of a stored iPod metadata backup.
#[derive(Debug, Clone)]
pub struct BackupSummary {
    pub path: PathBuf,
    pub created_at: String,
    pub size_bytes: u64,
}

/// Creates a backup of `iPod_Control/{Device,iTunes,Artwork}` using a local
/// date-time directory name (`backup_YYYY-MM-DD_HH-MM-SS`).
///
/// After the new backup succeeds, any older `backup_*` directories under
/// `backup_root` are automatically removed to save disk space.
pub fn create_prewrite_backup(mount_path: &Path, backup_root: &Path) -> Result<PathBuf> {
    let ipod_control = mount_path.join("iPod_Control");
    if !ipod_control.is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }

    fs::create_dir_all(backup_root).map_err(|e| PodDropError::Io(e.to_string()))?;

    let stamp = current_local_stamp();
    let mut dest_root = backup_root.join(format!("backup_{stamp}"));
    let mut suffix = 1u32;
    while dest_root.exists() {
        dest_root = backup_root.join(format!("backup_{stamp}_{suffix}"));
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

    // Remove older backups only after the new one has completed cleanly.
    if let Ok(entries) = fs::read_dir(backup_root) {
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

/// Returns the most recent valid backup under `backup_root`, if any.
pub fn latest_backup(backup_root: &Path) -> Option<BackupSummary> {
    let entries = fs::read_dir(backup_root).ok()?;
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
        created_at,
        size_bytes,
    })
}

/// Restores `iPod_Control/{Device,iTunes,Artwork}` from the latest backup under `backup_root`.
pub fn restore_latest_backup(mount_path: &Path, backup_root: &Path) -> Result<BackupSummary> {
    let ipod_control = mount_path.join("iPod_Control");
    if !ipod_control.is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }

    let summary = latest_backup(backup_root)
        .ok_or_else(|| PodDropError::Io("No backup found to restore.".to_string()))?;
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
    // Expected: YYYY-MM-DD_HH-MM-SS (optionally with _N suffix)
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
    fn backup_copies_metadata_dirs_and_rotates_previous() {
        let tmp = std::env::temp_dir().join("poddrop_test_backup");
        let _ = fs::remove_dir_all(&tmp);
        let mock_ipod = tmp.join("ipod");
        let backup_root = tmp.join("backups");

        fs::create_dir_all(mock_ipod.join("iPod_Control/iTunes")).unwrap();
        fs::create_dir_all(mock_ipod.join("iPod_Control/Device")).unwrap();
        fs::write(mock_ipod.join("iPod_Control/iTunes/iTunesDB"), b"db_v1").unwrap();
        fs::write(mock_ipod.join("iPod_Control/Device/SysInfo"), b"sys").unwrap();

        let first = create_prewrite_backup(&mock_ipod, &backup_root).unwrap();
        assert!(first.join("iPod_Control/iTunes/iTunesDB").is_file());
        assert!(first.join("iPod_Control/Device/SysInfo").is_file());

        // Second backup should delete the first backup directory.
        fs::write(mock_ipod.join("iPod_Control/iTunes/iTunesDB"), b"db_v2").unwrap();
        let second = create_prewrite_backup(&mock_ipod, &backup_root).unwrap();
        assert!(second.is_dir());
        assert!(!first.exists());

        // Corrupt current DB and restore from latest backup.
        fs::write(mock_ipod.join("iPod_Control/iTunes/iTunesDB"), b"corrupt").unwrap();
        let restored = restore_latest_backup(&mock_ipod, &backup_root).unwrap();
        assert_eq!(restored.path, second);
        assert_eq!(
            fs::read(mock_ipod.join("iPod_Control/iTunes/iTunesDB")).unwrap(),
            b"db_v2"
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
