use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{PodDropError, Result};

/// Creates a backup of `iPod_Control/{Device,iTunes,Artwork}` before any write operation.
///
/// Returns the created backup directory path.
pub fn create_prewrite_backup(mount_path: &Path, backup_root: &Path) -> Result<PathBuf> {
    let ipod_control = mount_path.join("iPod_Control");
    if !ipod_control.is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest_root = backup_root.join(format!("backup_{ts}"));
    let dest_control = dest_root.join("iPod_Control");
    fs::create_dir_all(&dest_control).map_err(|e| PodDropError::Io(e.to_string()))?;

    for sub in ["Device", "iTunes", "Artwork"] {
        let src = ipod_control.join(sub);
        if src.is_dir() {
            copy_dir_recursive(&src, &dest_control.join(sub))?;
        }
    }

    Ok(dest_root)
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
    fn backup_copies_metadata_dirs() {
        let tmp = std::env::temp_dir().join("poddrop_test_backup");
        let _ = fs::remove_dir_all(&tmp);
        let mock_ipod = tmp.join("ipod");
        let backup_root = tmp.join("backups");

        fs::create_dir_all(mock_ipod.join("iPod_Control/iTunes")).unwrap();
        fs::create_dir_all(mock_ipod.join("iPod_Control/Device")).unwrap();
        fs::write(mock_ipod.join("iPod_Control/iTunes/iTunesDB"), b"db").unwrap();
        fs::write(mock_ipod.join("iPod_Control/Device/SysInfo"), b"sys").unwrap();

        let created = create_prewrite_backup(&mock_ipod, &backup_root).unwrap();
        assert!(created.join("iPod_Control/iTunes/iTunesDB").is_file());
        assert!(created.join("iPod_Control/Device/SysInfo").is_file());

        let _ = fs::remove_dir_all(&tmp);
    }
}
