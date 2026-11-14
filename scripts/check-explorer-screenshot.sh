#!/usr/bin/env bash
# Pixel-probe the explorer screenshot against design tokens.
set -euo pipefail
PNG="${1:-artifacts/smithay/explorer.png}"
command -v convert >/dev/null || { echo "error: ImageMagick convert required" >&2; exit 1; }
[ -f "${PNG}" ] || { echo "error: ${PNG} missing" >&2; exit 1; }
probe() {
  convert "${PNG}" -format "%[hex:p{$1,$2}]" info: | tr '[:lower:]' '[:upper:]'
}
check() {
  local name="$1" x="$2" y="$3" expected="$4"
  local actual
  actual="$(probe "${x}" "${y}")"
  if [ "${actual}" != "${expected}" ]; then
    echo "error: ${name} at (${x},${y}) is #${actual}, expected #${expected}" >&2
    exit 1
  fi
  echo "${name}: #${actual}"
}
# 1280x800 screenshot; explorer rect is 190,100 -> 900x600.
check "background" 10 400 "031922"
check "panel" 200 400 "072733"
check "header" 200 120 "0D3D4B"
check "selection bar" 203 205 "38D6FF"
echo "explorer screenshot matches design tokens"
