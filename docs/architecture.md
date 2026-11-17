# PodDrop Architecture

## Design Goals

1. **Lightweight & Native**: Minimal memory footprint and fast startup on Linux using a Rust core (`poddrop-core` + `poddrop-ffi`) paired with a classic Qt6 Widgets GUI (`qt-gui`).
2. **Backend Abstraction**: Decouple UI and business logic from `libgpod` via an internal `IpodBackend` trait, allowing future support for newer devices (e.g., `HASHAB` / pure-Rust DB engine) without rewriting the UI.
3. **Safety First**: Always validate mount points, filesystem type (FAT32 vs read-only HFS+ warnings), and create pre-write backups of `iTunesDB` / `ArtworkDB` before mutating device state.

## Workspace Layout

```text
PodDrop/
├── crates/
│   ├── poddrop-core/   # Core domain types, backend traits, libgpod FFI, media & backup
│   ├── poddrop-cli/    # Diagnostic & verification CLI
│   └── poddrop-ffi/    # C ABI staticlib bridge for native GUIs
├── qt-gui/             # Classic Qt6 Widgets GUI (C++20 / CMake)
├── locales/            # Bilingual localization resources (en, ja)
└── docs/               # Architecture, roadmap, hardware verification logs
```

## Core Abstractions (`poddrop-core`)

- `DeviceModel`: Enum covering `Classic1G`–`Classic7G`, `Mini1G`–`Mini2G`, `Nano1G`–`Nano5G`.
- `DeviceInfo`: Mount path, model, capacity, FireWire GUID / `SysInfoExtended` status, filesystem type.
- `TrackMetadata`: Title, artist, album, track/disc number, duration, artwork reference.
- `IpodBackend` (trait):
  - `inspect_device(mount_path) -> Result<DeviceInfo>`
  - `list_tracks(mount_path) -> Result<Vec<TrackMetadata>>`
  - `transfer_tracks(...)` (Phase 2)
  - `set_artwork(...)` (Phase 2)
