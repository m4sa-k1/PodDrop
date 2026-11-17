# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
