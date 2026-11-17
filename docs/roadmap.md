# PodDrop Roadmap

We follow a strict, milestone-driven workflow. Each phase must be verified before moving to the next.

## Phase 0: Foundation & Project Setup (`v0.1.0-dev`) — Current
- [x] Repository initialization, GPL-3.0-or-later license, `.gitignore` safety rules
- [x] Cargo workspace (`poddrop-core`, `poddrop-cli`, `poddrop-gui`)
- [x] Bilingual (`en` / `ja`) localization foundation
- [x] GitHub Actions CI (`fmt`, `clippy`, `test`)

## Phase 1: Device Detection & Read-Only Verification (`v0.1.0-alpha.1`) — Completed
- [x] Install and verify `libgpod` FFI bindings (`poddrop-core`)
- [x] Implement safe device mount detection & filesystem check (FAT32 / HFS+ warning)
- [x] Read-only inspection of `SysInfo` / `SysInfoExtended` (FireWire GUID), `iTunesDB`, and `ArtworkDB`
- [x] Built-in SCSI VPD (`SG_IO`) `SysInfoExtended` reader (independent of broken `ipod-read-sysinfo-extended`)
- [x] **Hardware Verification**: Inspected real iPod Nano 4G (`B598`, 8.0 GB) via `poddrop-cli` without modifying any files on device

## Phase 2: Safe Transfer & Artwork Registration (`v0.1.0-alpha.2`)
- [ ] Pre-write automatic backup of `/iPod_Control/iTunes/` and `/iPod_Control/Artwork/`
- [ ] Audio metadata extraction and optional transcoding (FLAC/OGG -> ALAC/AAC/MP3 via `ffmpeg`)
- [ ] `iTunesDB` + `Hash58`/`Hash72` write support
- [ ] `ArtworkDB` + `.ithmb` (RGB565) generation and registration
- [ ] **Hardware Verification**: Transfer test tracks with album art to **iPod Nano 5G** and verify playback + artwork display on device

## Phase 3: Lightweight GTK4 GUI (`v0.1.0-beta.1`)
- [ ] GTK4 + libadwaita main window, device sidebar, track list
- [ ] Drag & drop music transfer and artwork preview/assignment
- [ ] Language switcher (`en` / `ja`) and progress/error dialogs

## Phase 4: Release Preparation (`v0.1.0`)
- [ ] Packaging (`PKGBUILD` / `.desktop` / AppStream metadata)
- [ ] Full documentation & release notes
