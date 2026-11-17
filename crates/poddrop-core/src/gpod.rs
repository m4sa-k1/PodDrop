use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::ptr;

use crate::backend::{
    IpodBackend, TrackMetadata, TransferTrackRequest, TransferTrackResult, UpdateAlbumRequest,
    UpdateTrackRequest,
};
use crate::backup::create_prewrite_backup;
use crate::device::{DeviceInfo, DeviceModel, FilesystemType};
use crate::error::{PodDropError, Result};
use crate::media::{extract_embedded_artwork, probe_audio, transcode_for_ipod};

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
    genre: *mut c_char,
    track_number: i32,
    disc_number: i32,
    year: i32,
    duration_ms: i32,
    bitrate_kbps: i32,
    size_bytes: u32,
    has_artwork: i32,
}

#[repr(C)]
struct PodDropGpodAddTrackRequest {
    audio_path: *const c_char,
    artwork_path: *const c_char,
    title: *const c_char,
    artist: *const c_char,
    album: *const c_char,
    track_number: i32,
    disc_number: i32,
    duration_ms: i32,
    bitrate_kbps: i32,
    samplerate_hz: u16,
}

#[repr(C)]
struct PodDropGpodUpdateTrackRequest {
    title: *const c_char,
    artist: *const c_char,
    album: *const c_char,
    genre: *const c_char,
    track_number: i32,
    disc_number: i32,
    year: i32,
    artwork_path: *const c_char,
    clear_artwork: i32,
}

#[repr(C)]
struct PodDropGpodUpdateAlbumRequest {
    old_album: *const c_char,
    new_album: *const c_char,
    new_artist: *const c_char,
    new_genre: *const c_char,
    year: i32,
    artwork_path: *const c_char,
    clear_artwork: i32,
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
    fn poddrop_read_sysinfo_vpd(
        dev_path: *const c_char,
        out_xml: *mut *mut c_char,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_gpod_add_track(
        mount_path: *const c_char,
        req: *const PodDropGpodAddTrackRequest,
        out_track_id: *mut u64,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_gpod_remove_track(
        mount_path: *const c_char,
        track_id: u64,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_gpod_update_track(
        mount_path: *const c_char,
        track_id: u64,
        req: *const PodDropGpodUpdateTrackRequest,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_gpod_update_album(
        mount_path: *const c_char,
        req: *const PodDropGpodUpdateAlbumRequest,
        out_updated_count: *mut u32,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_statvfs(
        path: *const c_char,
        out_total_bytes: *mut u64,
        out_free_bytes: *mut u64,
    ) -> c_int;
    fn poddrop_gpod_get_track_artwork_png(
        mount_path: *const c_char,
        track_id: u64,
        out_data: *mut *mut u8,
        out_len: *mut usize,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn poddrop_gpod_free_bytes(data: *mut u8);
}

/// Reads `SysInfoExtended` plist XML directly from an iPod block device via SCSI VPD (read-only).
pub fn read_sysinfo_extended_xml(dev_path: &Path) -> Result<String> {
    let c_dev = path_to_cstring(dev_path)?;
    let mut xml_ptr: *mut c_char = ptr::null_mut();
    let mut err_ptr: *mut c_char = ptr::null_mut();

    let rc = unsafe { poddrop_read_sysinfo_vpd(c_dev.as_ptr(), &mut xml_ptr, &mut err_ptr) };
    if rc != 0 {
        return Err(PodDropError::Backend(take_c_error(err_ptr)));
    }
    if xml_ptr.is_null() {
        return Err(PodDropError::Backend("Null XML pointer returned".into()));
    }
    let xml = unsafe { cstr_to_string(xml_ptr) };
    unsafe { poddrop_gpod_free_string(xml_ptr) };
    Ok(xml)
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

        let mut total_bytes: u64 = 0;
        let mut free_bytes: u64 = 0;
        let stat_ok =
            unsafe { poddrop_statvfs(c_path.as_ptr(), &mut total_bytes, &mut free_bytes) } == 0;

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
            storage_total_bytes: stat_ok.then_some(total_bytes),
            storage_free_bytes: stat_ok.then_some(free_bytes),
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
                    genre: unsafe { cstr_to_string(item.genre) },
                    track_number: u32::try_from(item.track_number).ok().filter(|&n| n > 0),
                    disc_number: u32::try_from(item.disc_number).ok().filter(|&n| n > 0),
                    year: u32::try_from(item.year).ok().filter(|&y| y > 0),
                    duration_ms: u32::try_from(item.duration_ms).unwrap_or(0),
                    bitrate_kbps: u32::try_from(item.bitrate_kbps).unwrap_or(0),
                    size_bytes: item.size_bytes,
                    has_artwork: item.has_artwork != 0,
                });
            }
        }

        unsafe { poddrop_gpod_free_tracks(tracks_ptr, count) };
        Ok(result)
    }

    fn transfer_track(
        &self,
        mount_path: &Path,
        request: &TransferTrackRequest,
        backup_root: Option<&Path>,
    ) -> Result<TransferTrackResult> {
        let info = self.inspect_device(mount_path)?;
        if info.is_read_only {
            return Err(PodDropError::ReadOnlyFilesystem(
                info.filesystem.as_str().to_string(),
            ));
        }

        let backup_path = if let Some(root) = backup_root {
            Some(create_prewrite_backup(mount_path, root)?)
        } else {
            None
        };

        let audio_info = probe_audio(&request.audio_path)?;
        let title = request
            .title_override
            .clone()
            .unwrap_or_else(|| audio_info.title.clone());
        let artist = request
            .artist_override
            .clone()
            .unwrap_or_else(|| audio_info.artist.clone());
        let album = request
            .album_override
            .clone()
            .unwrap_or_else(|| audio_info.album.clone());

        let work_dir = create_temp_work_dir()?;
        let _cleanup = TempDirGuard(work_dir.clone());

        let (final_audio_path, transcoded) = if audio_info.is_ipod_native(&request.audio_path) {
            (request.audio_path.clone(), false)
        } else {
            let out = transcode_for_ipod(&request.audio_path, &work_dir, request.transcode_format)?;
            (out, true)
        };

        let resolved_artwork: Option<PathBuf> = if let Some(art_path) = &request.artwork_path {
            if art_path.is_file() {
                Some(art_path.clone())
            } else {
                return Err(PodDropError::Io(format!(
                    "Artwork file not found: {}",
                    art_path.display()
                )));
            }
        } else if audio_info.has_embedded_art {
            let extracted = work_dir.join("cover.png");
            if extract_embedded_artwork(&request.audio_path, &extracted)? {
                Some(extracted)
            } else {
                None
            }
        } else {
            None
        };

        let c_mount = path_to_cstring(mount_path)?;
        let c_audio = path_to_cstring(&final_audio_path)?;
        let c_art = resolved_artwork
            .as_deref()
            .map(path_to_cstring)
            .transpose()?;
        let c_title = str_to_cstring(&title)?;
        let c_artist = str_to_cstring(&artist)?;
        let c_album = str_to_cstring(&album)?;

        let raw_req = PodDropGpodAddTrackRequest {
            audio_path: c_audio.as_ptr(),
            artwork_path: c_art.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            title: c_title.as_ptr(),
            artist: c_artist.as_ptr(),
            album: c_album.as_ptr(),
            track_number: audio_info.track_number.unwrap_or(0) as i32,
            disc_number: audio_info.disc_number.unwrap_or(0) as i32,
            duration_ms: audio_info.duration_ms as i32,
            bitrate_kbps: audio_info.bitrate_kbps as i32,
            samplerate_hz: u16::try_from(audio_info.samplerate_hz).unwrap_or(44100),
        };

        let mut track_id: u64 = 0;
        let mut err_ptr: *mut c_char = ptr::null_mut();
        let rc = unsafe {
            poddrop_gpod_add_track(c_mount.as_ptr(), &raw_req, &mut track_id, &mut err_ptr)
        };
        if rc != 0 {
            return Err(PodDropError::Backend(take_c_error(err_ptr)));
        }

        Ok(TransferTrackResult {
            track_id,
            title,
            artist,
            album,
            transcoded,
            artwork_registered: info.supports_artwork && resolved_artwork.is_some(),
            backup_path,
        })
    }

    fn remove_track(&self, mount_path: &Path, track_id: u64) -> Result<()> {
        let info = self.inspect_device(mount_path)?;
        if info.is_read_only {
            return Err(PodDropError::ReadOnlyFilesystem(
                info.filesystem.as_str().to_string(),
            ));
        }

        let c_mount = path_to_cstring(mount_path)?;
        let mut err_ptr: *mut c_char = ptr::null_mut();
        let rc = unsafe { poddrop_gpod_remove_track(c_mount.as_ptr(), track_id, &mut err_ptr) };
        if rc != 0 {
            return Err(PodDropError::Backend(take_c_error(err_ptr)));
        }
        Ok(())
    }

    fn update_track(
        &self,
        mount_path: &Path,
        track_id: u64,
        request: &UpdateTrackRequest,
    ) -> Result<()> {
        let info = self.inspect_device(mount_path)?;
        if info.is_read_only {
            return Err(PodDropError::ReadOnlyFilesystem(
                info.filesystem.as_str().to_string(),
            ));
        }

        let c_mount = path_to_cstring(mount_path)?;
        let c_title = request.title.as_deref().map(str_to_cstring).transpose()?;
        let c_artist = request.artist.as_deref().map(str_to_cstring).transpose()?;
        let c_album = request.album.as_deref().map(str_to_cstring).transpose()?;
        let c_genre = request.genre.as_deref().map(str_to_cstring).transpose()?;
        let c_art = request
            .artwork_path
            .as_deref()
            .map(path_to_cstring)
            .transpose()?;

        let raw = PodDropGpodUpdateTrackRequest {
            title: c_title.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            artist: c_artist.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            album: c_album.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            genre: c_genre.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            track_number: request.track_number.map_or(-1, |v| v as i32),
            disc_number: request.disc_number.map_or(-1, |v| v as i32),
            year: request.year.map_or(-1, |v| v as i32),
            artwork_path: c_art.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            clear_artwork: i32::from(request.clear_artwork),
        };

        let mut err_ptr: *mut c_char = ptr::null_mut();
        let rc =
            unsafe { poddrop_gpod_update_track(c_mount.as_ptr(), track_id, &raw, &mut err_ptr) };
        if rc != 0 {
            return Err(PodDropError::Backend(take_c_error(err_ptr)));
        }
        Ok(())
    }

    fn update_album(&self, mount_path: &Path, request: &UpdateAlbumRequest) -> Result<u32> {
        let info = self.inspect_device(mount_path)?;
        if info.is_read_only {
            return Err(PodDropError::ReadOnlyFilesystem(
                info.filesystem.as_str().to_string(),
            ));
        }

        let c_mount = path_to_cstring(mount_path)?;
        let c_old = str_to_cstring(&request.old_album)?;
        let c_album = request
            .new_album
            .as_deref()
            .map(str_to_cstring)
            .transpose()?;
        let c_artist = request
            .new_artist
            .as_deref()
            .map(str_to_cstring)
            .transpose()?;
        let c_genre = request
            .new_genre
            .as_deref()
            .map(str_to_cstring)
            .transpose()?;
        let c_art = request
            .artwork_path
            .as_deref()
            .map(path_to_cstring)
            .transpose()?;

        let raw = PodDropGpodUpdateAlbumRequest {
            old_album: c_old.as_ptr(),
            new_album: c_album.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            new_artist: c_artist.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            new_genre: c_genre.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            year: request.year.map_or(-1, |v| v as i32),
            artwork_path: c_art.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
            clear_artwork: i32::from(request.clear_artwork),
        };

        let mut updated: u32 = 0;
        let mut err_ptr: *mut c_char = ptr::null_mut();
        let rc = unsafe {
            poddrop_gpod_update_album(c_mount.as_ptr(), &raw, &mut updated, &mut err_ptr)
        };
        if rc != 0 {
            return Err(PodDropError::Backend(take_c_error(err_ptr)));
        }
        Ok(updated)
    }

    fn get_track_artwork_png(&self, mount_path: &Path, track_id: u64) -> Result<Option<Vec<u8>>> {
        validate_mount_path(mount_path)?;
        let c_mount = path_to_cstring(mount_path)?;
        let mut data_ptr: *mut u8 = ptr::null_mut();
        let mut data_len: usize = 0;
        let mut err_ptr: *mut c_char = ptr::null_mut();

        let rc = unsafe {
            poddrop_gpod_get_track_artwork_png(
                c_mount.as_ptr(),
                track_id,
                &mut data_ptr,
                &mut data_len,
                &mut err_ptr,
            )
        };
        if rc != 0 {
            return Err(PodDropError::Backend(take_c_error(err_ptr)));
        }
        if data_ptr.is_null() || data_len == 0 {
            if !data_ptr.is_null() {
                unsafe { poddrop_gpod_free_bytes(data_ptr) };
            }
            return Ok(None);
        }

        let bytes = unsafe { std::slice::from_raw_parts(data_ptr, data_len) }.to_vec();
        unsafe { poddrop_gpod_free_bytes(data_ptr) };
        Ok(Some(bytes))
    }
}

/// Flushes buffers (`sync`) and safely unmounts the mounted iPod via `udisksctl` or `umount`.
pub fn eject_ipod(mount_path: &Path) -> Result<()> {
    let canonical = fs::canonicalize(mount_path).unwrap_or_else(|_| mount_path.to_path_buf());
    let target = canonical.to_string_lossy();

    let block_dev = fs::read_to_string("/proc/mounts").ok().and_then(|mounts| {
        mounts.lines().find_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].replace("\\040", " ") == target {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
    });

    let _ = std::process::Command::new("sync").status();

    if let Some(dev) = block_dev {
        if let Ok(out) = std::process::Command::new("udisksctl")
            .args(["unmount", "-b", &dev])
            .output()
        {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    let out = std::process::Command::new("umount")
        .arg(mount_path)
        .output()
        .map_err(|e| PodDropError::Io(format!("Failed to unmount iPod: {e}")))?;

    if out.status.success() {
        Ok(())
    } else {
        Err(PodDropError::Backend(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
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

fn str_to_cstring(s: &str) -> Result<CString> {
    CString::new(s.as_bytes()).map_err(|e| PodDropError::Io(format!("Invalid string: {e}")))
}

fn create_temp_work_dir() -> Result<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("poddrop_work_{}_{}", std::process::id(), ts));
    fs::create_dir_all(&dir).map_err(|e| PodDropError::Io(e.to_string()))?;
    Ok(dir)
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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

/// Scans `/proc/mounts` for mounted filesystems containing an `iPod_Control` directory.
pub fn discover_mounted_ipods() -> Vec<PathBuf> {
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mnt_str = parts[1].replace("\\040", " ");
        let path = PathBuf::from(mnt_str);
        if path.join("iPod_Control").is_dir() && !found.contains(&path) {
            found.push(path);
        }
    }
    found
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
