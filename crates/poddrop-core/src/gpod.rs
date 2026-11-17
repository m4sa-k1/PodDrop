use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::ptr;

use crate::backend::{IpodBackend, TrackMetadata};
use crate::device::{DeviceInfo, DeviceModel, FilesystemType};
use crate::error::{PodDropError, Result};

#[repr(C)]
struct PodDropGpodDeviceInfo {
    model_number: *mut c_char,
    capacity_gb: f64,
    generation: i32,
    supports_artwork: i32,
    track_count: u32,
    playlist_count: u32,
    firewire_guid: *mut c_char,
}

#[repr(C)]
struct PodDropGpodTrack {
    id: u64,
    title: *mut c_char,
    artist: *mut c_char,
    album: *mut c_char,
    track_number: i32,
    disc_number: i32,
    duration_ms: i32,
    has_artwork: i32,
}

extern "C" {
    fn poddrop_gpod_inspect(
        mount_path: *const c_char,
        out_info: *mut PodDropGpodDeviceInfo,
        out_err: *mut *mut c_char,
    ) -> c_int;

    fn poddrop_gpod_list_tracks(
        mount_path: *const c_char,
        out_tracks: *mut *mut PodDropGpodTrack,
        out_count: *mut usize,
        out_err: *mut *mut c_char,
    ) -> c_int;

    fn poddrop_gpod_free_device_info(info: *mut PodDropGpodDeviceInfo);
    fn poddrop_gpod_free_tracks(tracks: *mut PodDropGpodTrack, count: usize);
    fn poddrop_gpod_free_string(s: *mut c_char);
}

/// Read-only and sync backend powered by `libgpod`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LibgpodBackend;

impl LibgpodBackend {
    pub const fn new() -> Self {
        Self
    }
}

impl IpodBackend for LibgpodBackend {
    fn inspect_device(&self, mount_path: &Path) -> Result<DeviceInfo> {
        validate_mount_path(mount_path)?;
        let c_path = path_to_cstring(mount_path)?;

        let mut raw = PodDropGpodDeviceInfo {
            model_number: ptr::null_mut(),
            capacity_gb: 0.0,
            generation: 0,
            supports_artwork: 0,
            track_count: 0,
            playlist_count: 0,
            firewire_guid: ptr::null_mut(),
        };
        let mut err_ptr: *mut c_char = ptr::null_mut();

        let rc = unsafe { poddrop_gpod_inspect(c_path.as_ptr(), &mut raw, &mut err_ptr) };
        if rc != 0 {
            let msg = take_c_error(err_ptr);
            return Err(PodDropError::Backend(msg));
        }

        let model_number = unsafe { cstr_to_opt_string(raw.model_number) };
        let firewire_guid = unsafe { cstr_to_opt_string(raw.firewire_guid) };
        let model = map_generation(raw.generation);
        let capacity_gb = if raw.capacity_gb > 0.0 {
            Some(raw.capacity_gb)
        } else {
            None
        };
        let supports_artwork = raw.supports_artwork != 0 || model.supports_artwork();
        let track_count = raw.track_count;
        let playlist_count = raw.playlist_count;

        unsafe { poddrop_gpod_free_device_info(&mut raw) };

        let (filesystem, is_read_only) = detect_mount_fs(mount_path);
        let has_sysinfo_extended = mount_path
            .join("iPod_Control/Device/SysInfoExtended")
            .is_file();

        Ok(DeviceInfo {
            mount_path: mount_path.to_path_buf(),
            model,
            model_number,
            capacity_gb,
            filesystem,
            is_read_only,
            has_sysinfo_extended,
            firewire_guid,
            supports_artwork,
            track_count,
            playlist_count,
        })
    }

    fn list_tracks(&self, mount_path: &Path) -> Result<Vec<TrackMetadata>> {
        validate_mount_path(mount_path)?;
        let c_path = path_to_cstring(mount_path)?;

        let mut tracks_ptr: *mut PodDropGpodTrack = ptr::null_mut();
        let mut count: usize = 0;
        let mut err_ptr: *mut c_char = ptr::null_mut();

        let rc = unsafe {
            poddrop_gpod_list_tracks(c_path.as_ptr(), &mut tracks_ptr, &mut count, &mut err_ptr)
        };
        if rc != 0 {
            let msg = take_c_error(err_ptr);
            return Err(PodDropError::Backend(msg));
        }

        let mut result = Vec::with_capacity(count);
        if !tracks_ptr.is_null() && count > 0 {
            let slice = unsafe { std::slice::from_raw_parts(tracks_ptr, count) };
            for item in slice {
                result.push(TrackMetadata {
                    id: item.id,
                    title: unsafe { cstr_to_string(item.title) },
                    artist: unsafe { cstr_to_string(item.artist) },
                    album: unsafe { cstr_to_string(item.album) },
                    track_number: u32::try_from(item.track_number).ok().filter(|&n| n > 0),
                    disc_number: u32::try_from(item.disc_number).ok().filter(|&n| n > 0),
                    duration_ms: u32::try_from(item.duration_ms).unwrap_or(0),
                    has_artwork: item.has_artwork != 0,
                });
            }
        }

        unsafe { poddrop_gpod_free_tracks(tracks_ptr, count) };
        Ok(result)
    }
}

fn validate_mount_path(mount_path: &Path) -> Result<()> {
    if !mount_path.is_dir() || !mount_path.join("iPod_Control").is_dir() {
        return Err(PodDropError::DeviceNotFound(
            mount_path.display().to_string(),
        ));
    }
    Ok(())
}

fn path_to_cstring(path: &Path) -> Result<CString> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| PodDropError::Io(format!("Invalid path: {e}")))
}

fn take_c_error(err_ptr: *mut c_char) -> String {
    if err_ptr.is_null() {
        return "Unknown libgpod error".to_string();
    }
    let msg = unsafe { CStr::from_ptr(err_ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { poddrop_gpod_free_string(err_ptr) };
    msg
}

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

unsafe fn cstr_to_opt_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn map_generation(gen_id: i32) -> DeviceModel {
    match gen_id {
        1 => DeviceModel::Classic1G,
        2 => DeviceModel::Classic2G,
        3 => DeviceModel::Classic3G,
        4 | 5 => DeviceModel::Classic4G,
        7 => DeviceModel::Mini1G,
        8 => DeviceModel::Mini2G,
        12 => DeviceModel::Nano1G,
        13 => DeviceModel::Nano2G,
        14 => DeviceModel::Nano3G,
        15 => DeviceModel::Nano4G,
        16 | 17 => DeviceModel::Classic5G,
        18 | 19 => DeviceModel::Classic6G,
        26 => DeviceModel::Classic7G,
        27 => DeviceModel::Nano5G,
        _ => DeviceModel::Unknown,
    }
}

/// Reads `/proc/mounts` to identify filesystem type and read-only mount status.
pub fn detect_mount_fs(mount_path: &Path) -> (FilesystemType, bool) {
    let canonical = fs::canonicalize(mount_path).unwrap_or_else(|_| mount_path.to_path_buf());
    let target = canonical.to_string_lossy();
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return (FilesystemType::Unknown, false);
    };

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let mnt = parts[1].replace("\\040", " ");
        if mnt == target {
            let fs_type = match parts[2] {
                "vfat" | "msdos" => FilesystemType::Fat32,
                "hfsplus" => FilesystemType::HfsPlus,
                _ => FilesystemType::Unknown,
            };
            let is_ro = parts[3].split(',').any(|opt| opt == "ro");
            return (fs_type, is_ro);
        }
    }

    (FilesystemType::Unknown, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_mapping_covers_targets() {
        assert_eq!(map_generation(18), DeviceModel::Classic6G);
        assert_eq!(map_generation(26), DeviceModel::Classic7G);
        assert_eq!(map_generation(27), DeviceModel::Nano5G);
        assert_eq!(map_generation(7), DeviceModel::Mini1G);
        assert_eq!(map_generation(0), DeviceModel::Unknown);
    }

    #[test]
    fn inspect_missing_path_returns_device_not_found() {
        let backend = LibgpodBackend::new();
        let err = backend
            .inspect_device(Path::new("/nonexistent/ipod/mount"))
            .unwrap_err();
        assert!(matches!(err, PodDropError::DeviceNotFound(_)));
    }
}
