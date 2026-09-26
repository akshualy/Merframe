#!/usr/bin/env bash
set -euo pipefail

appdir=target/release/bundle/appimage/Merframe.AppDir
appimage=$(realpath target/release/bundle/appimage/Merframe_*.AppImage)
tool=target/appimagetool

rm -f "$appdir"/usr/lib/libwayland-*.so* "$appdir"/usr/lib/libxkbcommon.so*
rm -rf "$appdir"/usr/lib/i386-linux-gnu
curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x "$tool"
rm -f "$appimage" "$appimage.sig"
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "$appimage"
pnpm --dir app tauri signer sign "$appimage"
