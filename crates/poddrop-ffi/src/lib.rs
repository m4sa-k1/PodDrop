#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;

use poddrop_core::backend::{
    IpodBackend, TransferTrackRequest, UpdateAlbumRequest, UpdateTrackRequest,
};
use poddrop_core::backup::default_backup_root;
use poddrop_core::gpod::{discover_or_mount_ipods, eject_ipod, LibgpodBackend};
use poddrop_core::media::{validate_audio_file, TranscodeFormat};
use poddrop_core::system::collect_system_diagnostics;
use poddrop_core::APP_VERSION;

#[repr(C)]
pub struct PodDropDeviceInfo {
    pub mount_path: *mut c_char,
    pub model_name: *mut c_char,
    pub model_number: *mut c_char,
    pub capacity_gb: f64,
    pub filesystem: *mut c_char,
    pub is_read_only: i32,
    pub has_sysinfo_extended: i32,
    pub firewire_guid: *mut c_char,
    pub supports_artwork: i32,
    pub track_count: u32,
    pub playlist_count: u32,
    pub storage_total_bytes: u64,
    pub storage_free_bytes: u64,
    pub has_storage_stats: i32,
}

#[repr(C)]
pub struct PodDropTrack {
    pub id: u64,
    pub title: *mut c_char,
    pub artist: *mut c_char,
    pub album_artist: *mut c_char,
    pub album: *mut c_char,
    pub genre: *mut c_char,
    pub track_number: u32,
    pub disc_number: u32,
    pub year: u32,
    pub duration_ms: u32,
    pub bitrate_kbps: u32,
    pub size_bytes: u32,
    pub has_artwork: i32,
}

#[repr(C)]
pub struct PodDropTransferRequest {
    pub audio_path: *const c_char,
    pub artwork_path: *const c_char,
    pub transcode_format: i32,
    pub title_override: *const c_char,
    pub artist_override: *const c_char,
    pub album_override: *const c_char,
}

#[repr(C)]
pub struct PodDropUpdateRequest {
    pub title: *const c_char,
    pub artist: *const c_char,
    pub album: *const c_char,
    pub genre: *const c_char,
    pub track_number: i32,
    pub disc_number: i32,
    pub year: i32,
    pub artwork_path: *const c_char,
    pub clear_artwork: i32,
}

#[repr(C)]
pub struct PodDropUpdateAlbumRequest {
    pub old_album: *const c_char,
    pub new_album: *const c_char,
    pub new_artist: *const c_char,
    pub new_genre: *const c_char,
    pub year: i32,
    pub artwork_path: *const c_char,
    pub clear_artwork: i32,
    pub apply_artist_to_tracks: i32,
    pub apply_genre_to_tracks: i32,
    pub apply_year_to_tracks: i32,
    pub apply_artwork_to_tracks: i32,
}

#[repr(C)]
pub struct PodDropSystemInfo {
    pub app_name: *mut c_char,
    pub app_version: *mut c_char,
    pub ffmpeg_version: *mut c_char,
    pub ffprobe_version: *mut c_char,
    pub udisksctl_status: *mut c_char,
    pub backup_dir: *mut c_char,
    pub last_backup_time: *mut c_char,
    pub last_backup_size_bytes: u64,
}

static VERSION_CSTR: std::sync::LazyLock<CString> =
    std::sync::LazyLock::new(|| CString::new(APP_VERSION).unwrap_or_default());

#[no_mangle]
pub extern "C" fn poddrop_app_version() -> *const c_char {
    VERSION_CSTR.as_ptr()
}

#[no_mangle]
pub extern "C" fn poddrop_discover_first_ipod() -> *mut c_char {
    match discover_or_mount_ipods().into_iter().next() {
        Some(p) => to_c_char(&p.to_string_lossy()),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_inspect_device(
    mount_path: *const c_char,
    out_info: *mut PodDropDeviceInfo,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(path) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    if out_info.is_null() {
        set_err(out_err, "Null out_info");
        return -1;
    }

    match LibgpodBackend::new().inspect_device(Path::new(&path)) {
        Ok(info) => {
            let (total, free, has_stats) = match (info.storage_total_bytes, info.storage_free_bytes)
            {
                (Some(t), Some(f)) => (t, f, 1),
                _ => (0, 0, 0),
            };
            unsafe {
                *out_info = PodDropDeviceInfo {
                    mount_path: to_c_char(&info.mount_path.to_string_lossy()),
                    model_name: to_c_char(info.model.display_name()),
                    model_number: to_c_char(info.model_number.as_deref().unwrap_or("")),
                    capacity_gb: info.capacity_gb.unwrap_or(0.0),
                    filesystem: to_c_char(info.filesystem.as_str()),
                    is_read_only: i32::from(info.is_read_only),
                    has_sysinfo_extended: i32::from(info.has_sysinfo_extended),
                    firewire_guid: to_c_char(info.firewire_guid.as_deref().unwrap_or("")),
                    supports_artwork: i32::from(info.supports_artwork),
                    track_count: info.track_count,
                    playlist_count: info.playlist_count,
                    storage_total_bytes: total,
                    storage_free_bytes: free,
                    has_storage_stats: has_stats,
                };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_list_tracks(
    mount_path: *const c_char,
    out_tracks: *mut *mut PodDropTrack,
    out_count: *mut usize,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if out_tracks.is_null() || out_count.is_null() {
        set_err(out_err, "Null output pointer");
        return -1;
    }
    unsafe {
        *out_tracks = ptr::null_mut();
        *out_count = 0;
    }
    let Some(path) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };

    match LibgpodBackend::new().list_tracks(Path::new(&path)) {
        Ok(tracks) => {
            let boxed: Vec<PodDropTrack> = tracks
                .into_iter()
                .map(|t| PodDropTrack {
                    id: t.id,
                    title: to_c_char(&t.title),
                    artist: to_c_char(&t.artist),
                    album_artist: to_c_char(&t.album_artist),
                    album: to_c_char(&t.album),
                    genre: to_c_char(&t.genre),
                    track_number: t.track_number.unwrap_or(0),
                    disc_number: t.disc_number.unwrap_or(0),
                    year: t.year.unwrap_or(0),
                    duration_ms: t.duration_ms,
                    bitrate_kbps: t.bitrate_kbps,
                    size_bytes: t.size_bytes,
                    has_artwork: i32::from(t.has_artwork),
                })
                .collect();
            let count = boxed.len();
            let ptr = if count == 0 {
                ptr::null_mut()
            } else {
                let mut slice = boxed.into_boxed_slice();
                let p = slice.as_mut_ptr();
                std::mem::forget(slice);
                p
            };
            unsafe {
                *out_tracks = ptr;
                *out_count = count;
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_transfer_track(
    mount_path: *const c_char,
    req: *const PodDropTransferRequest,
    create_backup: i32,
    out_track_id: *mut u64,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    if req.is_null() {
        set_err(out_err, "Null transfer request");
        return -1;
    }
    let r = unsafe { &*req };
    let Some(audio) = (unsafe { cstr_to_opt(r.audio_path) }) else {
        set_err(out_err, "Invalid audio_path");
        return -1;
    };
    let fmt = match r.transcode_format {
        1 => TranscodeFormat::Alac,
        2 => TranscodeFormat::Mp3,
        _ => TranscodeFormat::Aac,
    };
    let transfer_req = TransferTrackRequest {
        audio_path: PathBuf::from(audio),
        artwork_path: unsafe { cstr_to_opt(r.artwork_path) }.map(PathBuf::from),
        transcode_format: fmt,
        title_override: unsafe { cstr_to_opt(r.title_override) },
        artist_override: unsafe { cstr_to_opt(r.artist_override) },
        album_override: unsafe { cstr_to_opt(r.album_override) },
    };

    let backup_root = (create_backup != 0).then(default_backup_root);
    match LibgpodBackend::new().transfer_track(
        Path::new(&mp),
        &transfer_req,
        backup_root.as_deref(),
    ) {
        Ok(res) => {
            if !out_track_id.is_null() {
                unsafe { *out_track_id = res.track_id };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_update_track(
    mount_path: *const c_char,
    track_id: u64,
    req: *const PodDropUpdateRequest,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    if req.is_null() {
        set_err(out_err, "Null update request");
        return -1;
    }
    let r = unsafe { &*req };
    let update_req = UpdateTrackRequest {
        title: unsafe { cstr_to_opt_allow_empty(r.title) },
        artist: unsafe { cstr_to_opt_allow_empty(r.artist) },
        album: unsafe { cstr_to_opt_allow_empty(r.album) },
        genre: unsafe { cstr_to_opt_allow_empty(r.genre) },
        track_number: u32::try_from(r.track_number).ok(),
        disc_number: u32::try_from(r.disc_number).ok(),
        year: u32::try_from(r.year).ok(),
        artwork_path: unsafe { cstr_to_opt(r.artwork_path) }.map(PathBuf::from),
        clear_artwork: r.clear_artwork != 0,
    };

    match LibgpodBackend::new().update_track(Path::new(&mp), track_id, &update_req) {
        Ok(()) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_update_album(
    mount_path: *const c_char,
    req: *const PodDropUpdateAlbumRequest,
    out_updated_count: *mut u32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    if req.is_null() {
        set_err(out_err, "Null update album request");
        return -1;
    }
    let r = unsafe { &*req };
    let Some(old_album) = (unsafe { cstr_to_opt(r.old_album) }) else {
        set_err(out_err, "Invalid old_album");
        return -1;
    };

    let update_req = UpdateAlbumRequest {
        old_album,
        new_album: unsafe { cstr_to_opt_allow_empty(r.new_album) },
        new_artist: unsafe { cstr_to_opt_allow_empty(r.new_artist) },
        new_genre: unsafe { cstr_to_opt_allow_empty(r.new_genre) },
        year: u32::try_from(r.year).ok(),
        artwork_path: unsafe { cstr_to_opt(r.artwork_path) }.map(PathBuf::from),
        clear_artwork: r.clear_artwork != 0,
        apply_artist_to_tracks: r.apply_artist_to_tracks != 0,
        apply_genre_to_tracks: r.apply_genre_to_tracks != 0,
        apply_year_to_tracks: r.apply_year_to_tracks != 0,
        apply_artwork_to_tracks: r.apply_artwork_to_tracks != 0,
    };

    match LibgpodBackend::new().update_album(Path::new(&mp), &update_req) {
        Ok(cnt) => {
            if !out_updated_count.is_null() {
                unsafe { *out_updated_count = cnt };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_remove_album(
    mount_path: *const c_char,
    album_name: *const c_char,
    delete_tracks: i32,
    out_affected_count: *mut u32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    let Some(alb) = (unsafe { cstr_to_opt(album_name) }) else {
        set_err(out_err, "Invalid album_name");
        return -1;
    };

    match LibgpodBackend::new().remove_album(Path::new(&mp), &alb, delete_tracks != 0) {
        Ok(cnt) => {
            if !out_affected_count.is_null() {
                unsafe { *out_affected_count = cnt };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_remove_track(
    mount_path: *const c_char,
    track_id: u64,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    match LibgpodBackend::new().remove_track(Path::new(&mp), track_id) {
        Ok(()) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_get_track_artwork_png(
    mount_path: *const c_char,
    track_id: u64,
    out_data: *mut *mut u8,
    out_len: *mut usize,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if out_data.is_null() || out_len.is_null() {
        set_err(out_err, "Null output pointer");
        return -1;
    }
    unsafe {
        *out_data = ptr::null_mut();
        *out_len = 0;
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };

    match LibgpodBackend::new().get_track_artwork_png(Path::new(&mp), track_id) {
        Ok(Some(bytes)) if !bytes.is_empty() => {
            let len = bytes.len();
            let mut boxed = bytes.into_boxed_slice();
            let ptr = boxed.as_mut_ptr();
            std::mem::forget(boxed);
            unsafe {
                *out_data = ptr;
                *out_len = len;
            }
            0
        }
        Ok(_) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_eject_device(
    mount_path: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Invalid mount_path");
        return -1;
    };
    match eject_ipod(Path::new(&mp)) {
        Ok(()) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_bytes(data: *mut u8, len: usize) {
    if !data.is_null() && len > 0 {
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, len)) });
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_validate_audio(
    audio_path: *const c_char,
    out_size_bytes: *mut u64,
    out_duration_ms: *mut u32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(path) = (unsafe { cstr_to_opt(audio_path) }) else {
        set_err(out_err, "Invalid audio_path");
        return -1;
    };
    match validate_audio_file(Path::new(&path)) {
        Ok((info, size)) => {
            if !out_size_bytes.is_null() {
                unsafe { *out_size_bytes = size };
            }
            if !out_duration_ms.is_null() {
                unsafe { *out_duration_ms = info.duration_ms };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_get_system_info(out_info: *mut PodDropSystemInfo) {
    if out_info.is_null() {
        return;
    }
    let diag = collect_system_diagnostics();
    unsafe {
        *out_info = PodDropSystemInfo {
            app_name: to_c_char(&diag.app_name),
            app_version: to_c_char(&diag.app_version),
            ffmpeg_version: to_c_char(&diag.ffmpeg_version),
            ffprobe_version: to_c_char(&diag.ffprobe_version),
            udisksctl_status: to_c_char(&diag.udisksctl_status),
            backup_dir: to_c_char(&diag.backup_dir),
            last_backup_time: to_c_char(diag.last_backup_time.as_deref().unwrap_or("")),
            last_backup_size_bytes: diag.last_backup_size_bytes,
        };
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_system_info(info: *mut PodDropSystemInfo) {
    if info.is_null() {
        return;
    }
    let i = unsafe { &mut *info };
    unsafe {
        poddrop_free_string(i.app_name);
        poddrop_free_string(i.app_version);
        poddrop_free_string(i.ffmpeg_version);
        poddrop_free_string(i.ffprobe_version);
        poddrop_free_string(i.udisksctl_status);
        poddrop_free_string(i.backup_dir);
        poddrop_free_string(i.last_backup_time);
    }
    i.app_name = ptr::null_mut();
    i.app_version = ptr::null_mut();
    i.ffmpeg_version = ptr::null_mut();
    i.ffprobe_version = ptr::null_mut();
    i.udisksctl_status = ptr::null_mut();
    i.backup_dir = ptr::null_mut();
    i.last_backup_time = ptr::null_mut();
    i.last_backup_size_bytes = 0;
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_create_backup(
    mount_path: *const c_char,
    out_backup_time: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if !out_backup_time.is_null() {
        unsafe { *out_backup_time = ptr::null_mut() };
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let root = poddrop_core::backup::default_backup_root();
    match poddrop_core::backup::create_prewrite_backup(Path::new(&mp), &root) {
        Ok(_) => {
            if !out_backup_time.is_null() {
                let ts = poddrop_core::backup::latest_backup_for_device(Path::new(&mp), &root)
                    .map(|b| b.created_at)
                    .unwrap_or_default();
                unsafe { *out_backup_time = to_c_char(&ts) };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_get_device_backup_info(
    mount_path: *const c_char,
    out_backup_time: *mut *mut c_char,
    out_size_bytes: *mut u64,
    out_device_count: *mut u32,
) -> i32 {
    if !out_backup_time.is_null() {
        unsafe { *out_backup_time = ptr::null_mut() };
    }
    if !out_size_bytes.is_null() {
        unsafe { *out_size_bytes = 0 };
    }
    let root = poddrop_core::backup::default_backup_root();
    if !out_device_count.is_null() {
        unsafe { *out_device_count = poddrop_core::backup::backed_up_device_count(&root) };
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        return -1;
    };
    if let Some(summary) = poddrop_core::backup::latest_backup_for_device(Path::new(&mp), &root) {
        if !out_backup_time.is_null() {
            unsafe { *out_backup_time = to_c_char(&summary.created_at) };
        }
        if !out_size_bytes.is_null() {
            unsafe { *out_size_bytes = summary.size_bytes };
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_restore_latest_backup(
    mount_path: *const c_char,
    out_restored_time: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if !out_restored_time.is_null() {
        unsafe { *out_restored_time = ptr::null_mut() };
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let root = poddrop_core::backup::default_backup_root();
    match poddrop_core::backup::restore_latest_backup(Path::new(&mp), &root) {
        Ok(summary) => {
            if !out_restored_time.is_null() {
                unsafe { *out_restored_time = to_c_char(&summary.created_at) };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[repr(C)]
pub struct PodDropPlaylist {
    pub id: u64,
    pub name: *mut c_char,
    pub is_smart: i32,
    pub track_ids: *mut u32,
    pub track_count: usize,
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_list_playlists(
    mount_path: *const c_char,
    out_playlists: *mut *mut PodDropPlaylist,
    out_count: *mut usize,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if out_playlists.is_null() || out_count.is_null() {
        set_err(out_err, "Null output pointer");
        return -1;
    }
    unsafe {
        *out_playlists = ptr::null_mut();
        *out_count = 0;
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };

    match LibgpodBackend::new().list_playlists(Path::new(&mp)) {
        Ok(playlists) => {
            let count = playlists.len();
            let vec: Vec<PodDropPlaylist> = playlists
                .into_iter()
                .map(|p| {
                    let track_count = p.track_ids.len();
                    let track_ids = if track_count == 0 {
                        ptr::null_mut()
                    } else {
                        let boxed = p.track_ids.into_boxed_slice();
                        Box::into_raw(boxed) as *mut u32
                    };
                    PodDropPlaylist {
                        id: p.id,
                        name: to_c_char(&p.name),
                        is_smart: i32::from(p.is_smart),
                        track_ids,
                        track_count,
                    }
                })
                .collect();
            let ptr = if vec.is_empty() {
                ptr::null_mut()
            } else {
                let boxed = vec.into_boxed_slice();
                Box::into_raw(boxed) as *mut PodDropPlaylist
            };
            unsafe {
                *out_playlists = ptr;
                *out_count = count;
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_playlists(playlists: *mut PodDropPlaylist, count: usize) {
    if playlists.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(playlists, count)) };
    for p in slice.iter() {
        unsafe {
            poddrop_free_string(p.name);
        }
        if !p.track_ids.is_null() && p.track_count > 0 {
            drop(unsafe {
                Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    p.track_ids,
                    p.track_count,
                ))
            });
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_create_playlist(
    mount_path: *const c_char,
    name: *const c_char,
    track_ids: *const u32,
    track_count: usize,
    out_playlist_id: *mut u64,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let Some(pl_name) = (unsafe { cstr_to_opt(name) }) else {
        set_err(out_err, "Playlist name cannot be empty");
        return -1;
    };
    let ids: &[u32] = if track_ids.is_null() || track_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(track_ids, track_count) }
    };

    match LibgpodBackend::new().create_playlist(Path::new(&mp), &pl_name, ids) {
        Ok(id) => {
            if !out_playlist_id.is_null() {
                unsafe { *out_playlist_id = id };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_update_playlist(
    mount_path: *const c_char,
    playlist_id: u64,
    new_name: *const c_char,
    track_ids: *const u32,
    track_count: usize,
    replace_tracks: i32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let name_opt = unsafe { cstr_to_opt(new_name) };
    let ids_opt: Option<&[u32]> = if replace_tracks != 0 {
        if track_ids.is_null() || track_count == 0 {
            Some(&[])
        } else {
            Some(unsafe { std::slice::from_raw_parts(track_ids, track_count) })
        }
    } else {
        None
    };

    match LibgpodBackend::new().update_playlist(
        Path::new(&mp),
        playlist_id,
        name_opt.as_deref(),
        ids_opt,
    ) {
        Ok(()) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_add_tracks_to_playlist(
    mount_path: *const c_char,
    playlist_id: u64,
    track_ids: *const u32,
    track_count: usize,
    out_added_count: *mut u32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let ids: &[u32] = if track_ids.is_null() || track_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(track_ids, track_count) }
    };

    match LibgpodBackend::new().add_tracks_to_playlist(Path::new(&mp), playlist_id, ids) {
        Ok(added) => {
            if !out_added_count.is_null() {
                unsafe { *out_added_count = added };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_remove_playlist(
    mount_path: *const c_char,
    playlist_id: u64,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };

    match LibgpodBackend::new().remove_playlist(Path::new(&mp), playlist_id) {
        Ok(()) => 0,
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_export_tracks(
    mount_path: *const c_char,
    track_ids: *const u32,
    track_count: usize,
    dest_dir: *const c_char,
    out_exported_count: *mut u32,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    let Some(dest) = (unsafe { cstr_to_opt(dest_dir) }) else {
        set_err(out_err, "Null or empty dest_dir");
        return -1;
    };
    let ids: &[u32] = if track_ids.is_null() || track_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(track_ids, track_count) }
    };

    match LibgpodBackend::new().export_tracks(Path::new(&mp), ids, Path::new(&dest)) {
        Ok(exported) => {
            if !out_exported_count.is_null() {
                unsafe { *out_exported_count = exported };
            }
            0
        }
        Err(e) => {
            set_err(out_err, &e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_check_duplicates(
    mount_path: *const c_char,
    file_paths: *const *const c_char,
    file_count: usize,
    album_override: *const c_char,
    artist_override: *const c_char,
    out_duplicates_text: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    clear_err(out_err);
    if !out_duplicates_text.is_null() {
        unsafe { *out_duplicates_text = ptr::null_mut() };
    }
    let Some(mp) = (unsafe { cstr_to_opt(mount_path) }) else {
        set_err(out_err, "Null or empty mount_path");
        return -1;
    };
    if file_paths.is_null() || file_count == 0 {
        return 0;
    }

    let existing = match LibgpodBackend::new().list_tracks(Path::new(&mp)) {
        Ok(tracks) => tracks,
        Err(e) => {
            set_err(out_err, &e.to_string());
            return -1;
        }
    };

    let ov_album = unsafe { cstr_to_opt(album_override) };
    let ov_artist = unsafe { cstr_to_opt(artist_override) };
    let paths_slice = unsafe { std::slice::from_raw_parts(file_paths, file_count) };
    let mut dup_lines: Vec<String> = Vec::new();

    for &p_ptr in paths_slice {
        let Some(path_str) = (unsafe { cstr_to_opt(p_ptr) }) else {
            continue;
        };
        let path = Path::new(&path_str);
        let fallback_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let (title, artist, album) = match poddrop_core::media::probe_audio(path) {
            Ok(info) => (
                if info.title.is_empty() {
                    fallback_title
                } else {
                    info.title
                },
                ov_artist.clone().unwrap_or(info.artist),
                ov_album.clone().unwrap_or(info.album),
            ),
            Err(_) => (
                fallback_title,
                ov_artist.clone().unwrap_or_default(),
                ov_album.clone().unwrap_or_default(),
            ),
        };

        let t_low = title.trim().to_lowercase();
        let a_low = artist.trim().to_lowercase();
        let alb_low = album.trim().to_lowercase();
        if t_low.is_empty() {
            continue;
        }

        let is_dup = existing.iter().any(|ex| {
            ex.title.trim().to_lowercase() == t_low
                && (a_low.is_empty() || ex.artist.trim().to_lowercase() == a_low)
                && (alb_low.is_empty() || ex.album.trim().to_lowercase() == alb_low)
        });
        if is_dup {
            let label = if artist.is_empty() {
                title
            } else {
                format!("{title} — {artist}")
            };
            dup_lines.push(label);
        }
    }

    if !dup_lines.is_empty() && !out_duplicates_text.is_null() {
        unsafe { *out_duplicates_text = to_c_char(&dup_lines.join("\n")) };
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_device_info(info: *mut PodDropDeviceInfo) {
    if info.is_null() {
        return;
    }
    let i = unsafe { &mut *info };
    unsafe {
        poddrop_free_string(i.mount_path);
        poddrop_free_string(i.model_name);
        poddrop_free_string(i.model_number);
        poddrop_free_string(i.filesystem);
        poddrop_free_string(i.firewire_guid);
    }
    i.mount_path = ptr::null_mut();
    i.model_name = ptr::null_mut();
    i.model_number = ptr::null_mut();
    i.filesystem = ptr::null_mut();
    i.firewire_guid = ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn poddrop_free_tracks(tracks: *mut PodDropTrack, count: usize) {
    if tracks.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(tracks, count)) };
    for t in slice.iter() {
        unsafe {
            poddrop_free_string(t.title);
            poddrop_free_string(t.artist);
            poddrop_free_string(t.album_artist);
            poddrop_free_string(t.album);
            poddrop_free_string(t.genre);
        }
    }
}

fn to_c_char(s: &str) -> *mut c_char {
    let cleaned = s.replace('\0', "");
    CString::new(cleaned).unwrap_or_default().into_raw()
}

unsafe fn cstr_to_opt_allow_empty(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

unsafe fn cstr_to_opt(ptr: *const c_char) -> Option<String> {
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

fn clear_err(out_err: *mut *mut c_char) {
    if !out_err.is_null() {
        unsafe { *out_err = ptr::null_mut() };
    }
}

fn set_err(out_err: *mut *mut c_char, msg: &str) {
    if !out_err.is_null() {
        unsafe { *out_err = to_c_char(msg) };
    }
}
