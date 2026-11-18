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
# Fill probes tolerate one step of sRGB rounding.
check_near() {
  local name="$1" x="$2" y="$3" expected="$4" tolerance="${5:-3}"
  local actual
  actual="$(probe "${x}" "${y}")"
  local er=$((16#${expected:0:2})) eg=$((16#${expected:2:2})) eb=$((16#${expected:4:2}))
  local ar=$((16#${actual:0:2})) ag=$((16#${actual:2:2})) ab=$((16#${actual:4:2}))
  local dr=$((ar > er ? ar - er : er - ar))
  local dg=$((ag > eg ? ag - eg : eg - ag))
  local db=$((ab > eb ? ab - eb : eb - ab))
  if [ "${dr}" -gt "${tolerance}" ] || [ "${dg}" -gt "${tolerance}" ] || [ "${db}" -gt "${tolerance}" ]; then
    echo "error: ${name} at (${x},${y}) is #${actual}, expected ~#${expected}" >&2
    exit 1
  fi
  echo "${name}: #${actual}"
}
# 1280x800 screenshot; full-output WorkspaceWater gradient.
TOP="$(probe 10 400)"
BOTTOM="$(probe 10 790)"
[ "${TOP}" != "${BOTTOM}" ] || { echo "error: background gradient is flat (#${TOP})" >&2; exit 1; }
echo "background gradient: #${TOP} -> #${BOTTOM}"
# Geometry follows `.os-window--explorer`: 38 px menu bar, window at 16/54,
# 40 px title bar, 220 px sidebar, toolbar + search row, table, status strip.
check_near "menu bar" 640 30 "062833" 5
check_near "window title bar" 300 70 "0A3342" 6
check_near "toolbar" 300 110 "062632" 5
check_near "sidebar" 100 400 "052531" 5
check_near "column header" 300 190 "062632" 5
check "selection bar" 245 218 "38D6FF"
check_near "selection fill" 400 218 "114D5F" 8
check_near "status bar" 640 702 "052531" 5
echo "explorer screenshot matches design tokens"
