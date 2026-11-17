# PodDrop

A lightweight, open-source GUI application for Linux to transfer music and artwork to classic iPod devices.

[日本語 README はこちら](#日本語)

## Overview

PodDrop is designed for Linux users who want a clean, fast, and reliable way to manage classic iPod music libraries and album artwork (`ArtworkDB` / `.ithmb`) without heavy all-in-one media players.

### Target Devices (Initial v0.1.x Scope)

- **iPod Classic** (1G – 7G)
- **iPod Mini** (1G – 2G)
- **iPod Nano** (1G – 5G)

*(iPod Shuffle, Nano 6G/7G, and newer architectures are planned for evaluation in future releases via our abstracted backend.)*

### Planned Features

- Lightweight native Linux GUI (C++ / Qt6 Widgets + Rust core)
- High-compatibility database sync (`iTunesDB` / `Hash58` / `Hash72`) via `libgpod`
- Album artwork registration (`ArtworkDB` / RGB565 `.ithmb` conversion)
- Safe pre-write backup of device databases
- Bilingual UI: English & Japanese (`en` / `ja`)

## Project Status

**Phase 0 — Foundation & Architecture Setup (`v0.1.0-dev`)**

We are building PodDrop step by step with hardware verification at each milestone. See [docs/roadmap.md](docs/roadmap.md) and [docs/architecture.md](docs/architecture.md).

## Build & Test

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cmake -S qt-gui -B build && cmake --build build -j
```

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).

---

## 日本語

**PodDrop** は、Linux 上でクラシック系 iPod へ音楽転送および iPod 表示形式でのジャケット画像（`ArtworkDB` / `.ithmb`）登録を行うための、軽量・オープンソース GUI アプリケーションです。

### 初期バージョンの対象機種

- **iPod Classic** (第1世代〜第7世代)
- **iPod Mini** (第1世代〜第2世代)
- **iPod Nano** (第1世代〜第5世代)

### 現在のステータス

現在 **Phase 0（基盤・設計構築）** です。一気に実装せず、実機（iPod Nano 第5世代）を用いた段階的な読み取り・書き込み検証を経て開発を進めています。詳細は [docs/roadmap.md](docs/roadmap.md) を参照してください。
