# PodDrop Roadmap

We follow a strict, milestone-driven workflow. Each phase must be verified before moving to the next.

## Phase 0: Foundation & Project Setup (`v0.1.0-dev`) — Completed
- [x] Repository initialization, GPL-3.0-or-later license, `.gitignore` safety rules
- [x] Cargo workspace (`poddrop-core`, `poddrop-cli`, `poddrop-ffi`)
- [x] Bilingual (`en` / `ja`) localization foundation
- [x] GitHub Actions CI (`fmt`, `clippy`, `test`, `qt-gui` build)

## Phase 1: Device Detection & Read-Only Verification (`v0.1.0-alpha.1`) — Completed
- [x] Install and verify `libgpod` FFI bindings (`poddrop-core`)
- [x] Implement safe device mount detection & filesystem check (FAT32 / HFS+ warning)
- [x] Read-only inspection of `SysInfo` / `SysInfoExtended` (FireWire GUID), `iTunesDB`, and `ArtworkDB`
- [x] Built-in SCSI VPD (`SG_IO`) `SysInfoExtended` reader (independent of broken `ipod-read-sysinfo-extended`)
- [x] **Hardware Verification**: Inspected real iPod Nano 4G (`B598`, 8.0 GB) via `poddrop-cli` without modifying any files on device

## Phase 2: Safe Transfer & Artwork Registration (`v0.1.0-alpha.2`) — Completed
- [x] Pre-write automatic backup of `/iPod_Control/iTunes/` and `/iPod_Control/Artwork/`
- [x] Audio metadata extraction and optional transcoding (FLAC/OGG -> ALAC/AAC/MP3 via `ffmpeg`)
- [x] `iTunesDB` + `Hash58`/`Hash72` write support
- [x] `ArtworkDB` + `.ithmb` (RGB565) generation and registration
- [x] **Sandbox Verification**: Verified FLAC->AAC transcode, 6 `.ithmb` artwork files generation, and track removal on local Nano 4G snapshot
- [x] **Hardware Verification**: Transferred tracks with cover art to real iPod Nano 4G (`B598`), verified playback, cover display on device, and safe eject (`unmount` + `power-off`)

## Phase 3: Lightweight Classic GUI (`v0.1.0-beta.1`) — Completed
- [x] Classic Qt6 Widgets `QMainWindow` (menu bar, toolbar, device/album sidebar, sortable `QTableWidget`, status bar)
- [x] Read-only mount safety notice and write-lock guard
- [x] Drag & drop / file-picker transfer, album creation dialog, track & album metadata batch editors, live cover artwork preview
- [x] Safe eject button (`sync` + `udisksctl unmount` + `udisksctl power-off`)
- [x] Live language (`en` / `ja`) and classic monochrome Light / Dark theme switching

## Phase 4: Release Preparation (`v0.1.0`) — Completed
- [x] Desktop entry (`io.github.m4sa_k1.PodDrop.desktop`), SVG icon, and CMake install targets
- [x] Documentation (`README.md`, `CHANGELOG.md`, `docs/architecture.md`) and version bump to `0.1.0`

## Future Ideas (`v0.2.0+`)
- [ ] Playlist creation and management
- [ ] iPod Shuffle (`iTunesSD`) and iPod Nano 6G/7G (`HASHAB`) research & support
- [ ] Optional AUR `PKGBUILD` and Flatpak manifest
