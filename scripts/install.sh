#!/usr/bin/env bash
set -euo pipefail

PREFIX="${HOME}/.local"
BUILD_DIR="build"

usage() {
    cat <<'EOF'
Usage: ./scripts/install.sh [--system | --prefix <path>]

Options:
  --system         Install system-wide to /usr/local (requires sudo/root)
  --prefix <path>  Install to a custom prefix (default: ~/.local)
  -h, --help       Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --system)
            PREFIX="/usr/local"
            shift
            ;;
        --prefix)
            PREFIX="${2:?Missing path after --prefix}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

missing=()
for cmd in cargo cmake g++ pkg-config ffmpeg; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
done
pkg-config --exists libgpod-1.0 gdk-pixbuf-2.0 Qt6Widgets 2>/dev/null || missing+=("libgpod / gdk-pixbuf2 / Qt6Widgets dev packages")

if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Missing required dependencies:" >&2
    for m in "${missing[@]}"; do
        echo "  - $m" >&2
    done
    echo "" >&2
    echo "Install dependencies first:" >&2
    echo "  Arch Linux:    sudo pacman -S rust cmake gcc pkgconf libgpod qt6-base ffmpeg udisks2" >&2
    echo "  Ubuntu/Debian: sudo apt install cargo cmake g++ pkg-config libgpod-dev libgdk-pixbuf-2.0-dev qt6-base-dev ffmpeg udisks2" >&2
    exit 1
fi

echo "==> Building poddrop-cli (Release)..."
cargo build --release -p poddrop-cli

echo "==> Building poddrop-gui (Release)..."
cmake -S qt-gui -B "${BUILD_DIR}" -DCMAKE_BUILD_TYPE=Release
cmake --build "${BUILD_DIR}" -j"$(nproc)"

echo "==> Installing to ${PREFIX}..."
cmake --install "${BUILD_DIR}" --prefix "${PREFIX}"
install -Dm755 target/release/poddrop-cli "${PREFIX}/bin/poddrop-cli"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${PREFIX}/share/applications" >/dev/null 2>&1 || true
fi

echo ""
echo "PodDrop installed to ${PREFIX}."
echo "Run 'poddrop-gui' (ensure ${PREFIX}/bin is in your PATH) or launch from your app menu."
