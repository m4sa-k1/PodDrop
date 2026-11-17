# 🎵 PodDrop

Linux でクラシック系 iPod へ音楽転送とカバー画像登録ができる、軽量・オープンソースのデスクトップアプリです！🐧✨

---

## ✨ 特徴・できること

- 🖥️ **シンプルで軽快なクラシック UI**（C++20 / Qt6 Widgets + Rust コア、単一バイナリ約 1.7 MB）
- 🔌 **挿すだけで自動認識＆自動マウント**（`udisks2` 連携・未マウント状態でも自動検出）
- 🛡️ **大切な iPod を守る安全設計**
  - 書き込み前に `/iPod_Control/{Device,iTunes,Artwork}` を自動バックアップ（`~/.local/share/poddrop/backups/`）
  - 読み取り専用マウント（HFS+ ジャーナリング等）を検知すると書き込みを自動ロック
  - `sync` ＋ `udisksctl unmount` ＋ `power-off` で iPod 実機画面までしっかり切り替わる **安全取り出し**
- 🎶 **ドラッグ＆ドロップ転送 ＆ 自動変換**
  - FLAC / OGG / WAV なども `ffmpeg` で iPod 互換の **AAC / ALAC / MP3** に自動変換して転送
- 🖼️ **カバー画像（アートワーク）登録 ＆ ライブプレビュー**
  - `ArtworkDB` / RGB565 `.ithmb` をモデルに合わせて生成・登録
  - メイン画面・曲編集・アルバム編集・アルバム作成画面のすべてでカバー画像をプレビュー表示
- 💿 **アルバム作成 ＆ 曲・アルバム一括編集**
  - アルバム単位の絞り込み、まとめて追加、タイトル・アーティスト・ジャンル・トラック番号・年・カバー画像の編集に対応
- 🌐 **日本語 / English 切り替え** ＆ **ライト（白）/ ダーク（黒）テーマ切り替え**
- 🔧 **自前 SCSI VPD リーダー内蔵**（`poddrop-cli read-sysinfo` により外部バイナリに頼らず `SysInfoExtended` を取得可能）

---

## 📱 対応機種（`v0.1.0`）

- **iPod Classic**（第1世代 〜 第7世代）
- **iPod Mini**（第1世代 〜 第2世代）
- **iPod Nano**（第1世代 〜 第5世代）
  - *iPod Nano 第4世代（モデル番号 `B598`）実機にて、転送・カバー画像表示・再生・安全取り出しを検証済みです！*

*(iPod Shuffle や Nano 6G/7G などは今後のアップデートで検討予定です)*

---

## 🚀 簡単インストール

### 方法 1：リリース版アーカイブから導入（おすすめ・ビルド不要）

[Releases](https://github.com/m4sa-k1/PodDrop/releases) から `poddrop-v0.1.0-linux-x86_64.tar.gz` をダウンロードして展開し、`./install.sh` を実行するだけです！

```bash
tar -xzf poddrop-v0.1.0-linux-x86_64.tar.gz
cd poddrop-v0.1.0-linux-x86_64
./install.sh          # ユーザー環境 (~/.local) にインストール
# システム全体に入れる場合: sudo ./install.sh --system
```

### 方法 2：Arch Linux（`PKGBUILD`）

```bash
git clone https://github.com/m4sa-k1/PodDrop.git
cd PodDrop/packaging/arch
makepkg -si
```

### 方法 3：ソースから 1 コマンドでビルド＆インストール

```bash
git clone https://github.com/m4sa-k1/PodDrop.git
cd PodDrop
./scripts/install.sh  # アンインストールは ./scripts/uninstall.sh
```

---

## 📦 必要なパッケージ

- `libgpod`, `gdk-pixbuf2`, `qt6-base`, `ffmpeg`, `udisks2`
- （ソースからビルドする場合）`rust` / `cargo`（1.75 以上）, `cmake`, `g++`（C++20）, `pkg-config`

**Arch Linux:**
```bash
sudo pacman -S libgpod qt6-base cmake ffmpeg udisks2 pkgconf
```

**Ubuntu / Debian:**
```bash
sudo apt install pkg-config libgpod-dev libgdk-pixbuf-2.0-dev cmake g++ qt6-base-dev ffmpeg udisks2
```

---

## 🛠️ 開発・手動ビルド

```bash
# テスト & Lint
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Qt6 GUI ビルド（poddrop-ffi も自動でビルドされます）
cmake -S qt-gui -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j

# 起動
./build/poddrop-gui
```

---

## ⚠️ 免責事項

- **非公式ソフトウェア**: PodDrop は個人・コミュニティによる非公式のオープンソースツールであり、Apple Inc. とは一切関係ありません（iPod・iTunes は Apple Inc. の登録商標または商標です）。
- **無保証・自己責任**: 本ソフトウェアは現状有姿（AS-IS）で提供され、明示または黙示を問わずいかなる保証も行いません。本ソフトウェアの使用または使用不能により生じた、音楽データやデータベースの破損・消失、機器の不具合、その他一切の損害について、開発者および貢献者は責任を負いかねます。
- **バックアップの推奨**: PodDrop は書き込み前に自動バックアップを作成しますが、大切な実機を保護するため、**初回利用前や重要な操作の前にはご自身でも `/iPod_Control/` や元データのバックアップを別途取得する**ことを強く推奨します。

---

## 📄 ライセンス

[GNU General Public License v3.0 or later](LICENSE)（`GPL-3.0-or-later`）のもとで公開されています。
