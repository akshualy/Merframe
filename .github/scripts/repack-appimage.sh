#!/usr/bin/env bash
set -euo pipefail

bundle_dir="$(dirname "$0")/../../target/release/bundle/appimage"
appdir="$bundle_dir/Merframe.AppDir"
appimage="$(ls "$bundle_dir"/Merframe_*.AppImage)"
tool="$RUNNER_TEMP/appimagetool"

rm -f "$appdir"/usr/lib/libwayland-*.so* "$appdir"/usr/lib/libxkbcommon.so*
rm -rf "$appdir"/usr/lib/i386-linux-gnu
curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x "$tool"
rm -f "$appimage"
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "$appimage"
