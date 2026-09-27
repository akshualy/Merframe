#!/usr/bin/env bash
set -euo pipefail
shopt -s globstar nullglob inherit_errexit

version=$1
url=https://github.com/akshualy/Merframe/releases/download/v$version
cd bundles

entry() {
  local files=(**/$1)
  if [ ${#files[@]} -ne 1 ]; then
    echo "expected one $1 under bundles, found ${#files[@]}" >&2
    exit 1
  fi
  local signature
  signature=$(cat "${files[0]}.sig")
  jq -n --arg url "$url/$(basename "${files[0]}")" --arg signature "$signature" '{url: $url, signature: $signature}'
}

appimage=$(entry 'Merframe_*.AppImage')
deb=$(entry 'Merframe_*.deb')
rpm=$(entry 'Merframe-*.rpm')
nsis=$(entry 'Merframe_*-setup.exe')
msi=$(entry 'Merframe_*.msi')

jq -n \
  --arg version "$version" \
  --arg pub_date "$(date -u +%FT%TZ)" \
  --argjson appimage "$appimage" \
  --argjson deb "$deb" \
  --argjson rpm "$rpm" \
  --argjson nsis "$nsis" \
  --argjson msi "$msi" \
  '{$version, $pub_date, platforms: {
    "linux-x86_64-appimage": $appimage,
    "linux-x86_64-deb": $deb,
    "linux-x86_64-rpm": $rpm,
    "windows-x86_64-nsis": $nsis,
    "windows-x86_64-msi": $msi
  }}'
