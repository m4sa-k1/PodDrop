# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial repository structure, GPL-3.0-or-later license, and Cargo workspace (`poddrop-core`, `poddrop-cli`, `poddrop-gui`).
- Architecture and phased roadmap documentation (`docs/architecture.md`, `docs/roadmap.md`).
- GitHub Actions CI workflow (`fmt`, `clippy`, `test`).
- Minimal core models, error types, and bilingual (`en`/`ja`) localization scaffolding.
- Read-only `libgpod` FFI backend and filesystem/mount detection (`poddrop-core`).
- Built-in SCSI VPD (`0xC0`+) `SysInfoExtended` reader via `SG_IO` (`poddrop-core`, `poddrop-cli`).
- `poddrop-cli inspect` and `poddrop-cli read-sysinfo` commands.
- Pre-write backup, `ffprobe`/`ffmpeg` metadata & transcode pipeline, `iTunesDB`/`ArtworkDB` track transfer & removal (`poddrop-core`, `poddrop-cli`).
- Lightweight GTK4 + `libadwaita` GUI (`poddrop-gui`) with auto-detection, drag & drop transfer, artwork selection, read-only safety guard, and live `en`/`ja` switching.
