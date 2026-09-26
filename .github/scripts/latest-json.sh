#!/usr/bin/env bash
set -euo pipefail

version=$1
url=https://github.com/akshualy/Merframe/releases/download/v$version
cd bundles

entry() {
  jq -n --arg url "$url/$1" --arg signature "$(cat "$1.sig")" '{url: $url, signature: $signature}'
}

jq -n \
  --arg version "$version" \
  --arg pub_date "$(date -u +%FT%TZ)" \
  --argjson appimage "$(entry Merframe_*.AppImage)" \
  --argjson deb "$(entry Merframe_*.deb)" \
  --argjson rpm "$(entry Merframe-*.rpm)" \
  --argjson nsis "$(entry Merframe_*-setup.exe)" \
  --argjson msi "$(entry Merframe_*.msi)" \
  '{$version, $pub_date, platforms: {
    "linux-x86_64-appimage": $appimage,
    "linux-x86_64-deb": $deb,
    "linux-x86_64-rpm": $rpm,
    "windows-x86_64-nsis": $nsis,
    "windows-x86_64-msi": $msi
  }}'
