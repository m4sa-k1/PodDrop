# PodDrop

A lightweight, open-source GUI application for Linux to transfer music and cover artwork to classic iPod devices.

[日本語 README はこちら](#日本語)

## Overview

PodDrop is designed for Linux users who want a fast, reliable, classic desktop utility to manage iPod libraries (`iTunesDB` / `ArtworkDB` / `.ithmb`) without heavy media players.

### Target Devices (`v0.1.0`)

- **iPod Classic** (1G – 7G)
- **iPod Mini** (1G – 2G)
- **iPod Nano** (1G – 5G) — *Hardware verified on iPod Nano 4th Gen (`B598`)*

*(iPod Shuffle, Nano 6G/7G, and newer devices are planned for future evaluation via our abstracted backend.)*

### Features

- Classic, lightweight native desktop UI (C++20 / Qt6 Widgets + Rust core)
- Automatic iPod mount detection and FAT32 / read-only HFS+ safety guard
- Pre-write automatic backup of `/iPod_Control/{Device,iTunes,Artwork}`
- Built-in SCSI VPD (`SG_IO`) `SysInfoExtended` reader (`poddrop-cli read-sysinfo`)
- Drag & drop and batch album transfer with automatic `ffmpeg` transcoding (FLAC/OGG/WAV → AAC / ALAC / MP3)
- Cover artwork registration (`ArtworkDB` / RGB565 `.ithmb`) and live thumbnail preview
- Track and album metadata viewer & batch editor
- Safe eject (`sync` + `udisksctl unmount` + `udisksctl power-off`)
- Bilingual UI: English & Japanese (`en` / `ja`) + Light / Dark monochrome themes

## Quick Install

### Option 1: From GitHub Releases (Prebuilt x86_64 Archive)

Download `poddrop-v0.1.0-linux-x86_64.tar.gz` from [Releases](https://github.com/m4sa-k1/PodDrop/releases), then run:

```bash
tar -xzf poddrop-v0.1.0-linux-x86_64.tar.gz
cd poddrop-v0.1.0-linux-x86_64
./install.sh          # User install (~/.local)
# or: sudo ./install.sh --system
```

### Option 2: Arch Linux (`PKGBUILD`)

```bash
git clone https://github.com/m4sa-k1/PodDrop.git
cd PodDrop/packaging/arch
makepkg -si
```

### Option 3: One-Step Source Install

```bash
git clone https://github.com/m4sa-k1/PodDrop.git
cd PodDrop
./scripts/install.sh  # Use --system for /usr/local
```

## Dependencies

- Rust toolchain (`cargo`, `rustc` >= 1.75)
- `cmake`, `g++` (C++20), `pkg-config`
- `libgpod`, `glib2`, `gdk-pixbuf2`
- `qt6-base` (Qt6 Widgets)
- `ffmpeg` (for `ffprobe` tag inspection and transcoding)
- `udisks2` (for safe unmount / eject)

On Arch Linux:

```bash
sudo pacman -S libgpod qt6-base cmake ffmpeg udisks2 pkgconf
```

On Ubuntu / Debian:

```bash
sudo apt install pkg-config libgpod-dev libgdk-pixbuf-2.0-dev cmake g++ qt6-base-dev ffmpeg udisks2
```

## Build & Run

```bash
# Run unit tests & lints
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Build Qt6 GUI (automatically builds poddrop-ffi)
cmake -S qt-gui -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j

# Launch GUI
./build/poddrop-gui
```

Optional system installation:

```bash
sudo cmake --install build
```

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).

---

## 日本語

**PodDrop** は、Linux 上でクラシック系 iPod へ音楽転送および iPod 表示形式でのカバー画像（`ArtworkDB` / `.ithmb`）登録を行うための、軽量・オープンソース GUI アプリケーションです。

### 初期バージョン（`v0.1.0`）の対象機種

- **iPod Classic** (第1世代〜第7世代)
- **iPod Mini** (第1世代〜第2世代)
- **iPod Nano** (第1世代〜第5世代) — *iPod Nano 第4世代（`B598`）実機にて転送・カバー表示・再生・取り出しを検証済み*

### 主な機能

- シンプルで軽量なクラシック・デスクトップ UI（C++ / Qt6 Widgets + Rust コア）
- マウント自動検出、読み取り専用（`ro` / ジャーナリング付き HFS+）保護ガード
- 書き込み前の自動バックアップ作成（`~/.local/share/poddrop/backups/`）
- 自前 SCSI VPD (`SG_IO`) リーダーによる `SysInfoExtended` 取得（`poddrop-cli read-sysinfo`）
- ドラッグ＆ドロップおよびアルバム単位のまとめて転送、`ffmpeg` による自動形式変換（AAC / ALAC / MP3）
- カバー画像の自動登録（`ArtworkDB` / `.ithmb`）とプレビュー表示
- 曲情報およびアルバム単位での一括閲覧・編集
- 実機画面を切り替える安全な取り出し（`sync` + `udisksctl unmount` + `udisksctl power-off`）
- 日本語 / 英語切り替え、ライト / ダーク（白黒基調）テーマ切り替え

### 簡単インストール

- **リリース版アーカイブ（ビルド不要）**: [Releases](https://github.com/m4sa-k1/PodDrop/releases) から `.tar.gz` を展開し、`./install.sh` を実行（システム全体に入れる場合は `sudo ./install.sh --system`）。
- **Arch Linux (`makepkg`)**: `packaging/arch/PKGBUILD` のディレクトリで `makepkg -si`。
- **ソースから 1 コマンド導入**: リポジトリ直下で `./scripts/install.sh`（アンインストールは `./scripts/uninstall.sh`）。
