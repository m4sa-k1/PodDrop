#!/usr/bin/env bash
set -euo pipefail

PREFIX="${HOME}/.local"

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
        *)
            echo "Usage: ./scripts/uninstall.sh [--system | --prefix <path>]" >&2
            exit 1
            ;;
    esac
done

rm -f "${PREFIX}/bin/poddrop-gui"
rm -f "${PREFIX}/bin/poddrop-cli"
rm -f "${PREFIX}/share/applications/io.github.m4sa_k1.PodDrop.desktop"
rm -f "${PREFIX}/share/icons/hicolor/scalable/apps/io.github.m4sa_k1.PodDrop.svg"
rm -f "${PREFIX}/share/icons/hicolor/128x128/apps/io.github.m4sa_k1.PodDrop.png"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${PREFIX}/share/applications" >/dev/null 2>&1 || true
fi

echo "PodDrop removed from ${PREFIX}."
