# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-11-17

### Added
- Optional checkbox in the Playlist Editor (`Also permanently delete removed tracks from iPod`, unchecked by default) so users can choose whether removing a track from a playlist only removes it from the playlist or also deletes it from the iPod.

## [0.1.5] - 2026-11-17

### Added
- Automatic backup on iPod connection/open with date-time folder naming (`backup_YYYY-MM-DD_HH-MM-SS`) and automatic rotation (removes older backups after the new one completes to save disk space).
- In-app restore (`File -> Restore from Backup...`) to restore `iPod_Control/{Device,iTunes,Artwork}` from the latest backup.
- Latest backup timestamp and size display in `Help -> About & System Info...`.

### Changed
- Split `qt-gui/src/mainwindow.cpp` into modular implementation files (`mainwindow_device.cpp`, `mainwindow_albums.cpp`, `mainwindow_playlists.cpp`, `mainwindow_transfer.cpp`) for maintainability.

## [0.1.4] - 2026-11-17

### Added
- Dedicated [TERMS.md](TERMS.md) detailing Terms of Use & Disclaimer (retroactively applicable to all past versions `v0.1.0`–`v0.1.3`, with Japanese as the sole legally valid text).
- Built-in Terms & Disclaimer viewer (`Help -> Terms of Use & Disclaimer...`) embedded into `poddrop-gui`.
- Included `TERMS.md` in release archives (`release.yml`).

## [0.1.3] - 2026-11-17

### Added
- Playlist management (`Playlists` tab in sidebar, `New Playlist`, `Edit Playlist`, `Delete Playlist`) with track filtering, adding/removing tracks, and reordering playback order.
- Quick `Add to Playlist...` action from track and album context menus (supports adding to an existing playlist or creating a new one).

## [0.1.2] - 2026-11-17

### Added
- Album editor scope checkboxes to choose per field (`Artist`, `Genre`, `Year`, `Cover`) whether edits apply only to the album (`albumartist` / album grouping) or also overwrite the tracks inside.
- Album removal (`Remove Album`) with a choice to either delete the album only (releasing tracks by clearing their album metadata) or delete the album along with all tracks inside.

## [0.1.1] - 2026-11-17

### Added
- System & version diagnostics dialog (`Help -> About & System Info...`) showing app version, Qt version, `ffmpeg`/`ffprobe`/`udisksctl` status, and backup path.
- Folder drag & drop album creation with strict pre-flight validation (file readability, `ffprobe` audio stream & duration checks, free space verification, and automatic cover image detection) before writing anything to the iPod.

## [0.1.0] - 2026-11-17

### Added
- Initial public release of PodDrop (`GPL-3.0-or-later`).
- Rust core (`poddrop-core`), diagnostic/transfer CLI (`poddrop-cli`), and C ABI bridge (`poddrop-ffi`).
- `libgpod` FFI integration for `iTunesDB` / `ArtworkDB` read/write and RGB565 `.ithmb` thumbnail extraction.
- Built-in SCSI VPD (`0xC0`+) `SysInfoExtended` reader via Linux `SG_IO`.
- Pre-write automatic backups and `ffprobe` / `ffmpeg` audio inspection and transcoding (`AAC`, `ALAC`, `MP3`).
- Classic Qt6 Widgets GUI (`qt-gui`) featuring album browser, sortable track table, track & album metadata editors, live cover image preview, drag & drop transfer, safe eject (`unmount` + `power-off`), bilingual (`en`/`ja`) UI, and monochrome Light/Dark themes.
- Desktop entry and SVG icon (`packaging/`), GitHub Actions CI workflow.
