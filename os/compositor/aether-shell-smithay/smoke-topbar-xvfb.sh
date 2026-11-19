#!/usr/bin/env bash
# Menu bar and dock: verify pointer cursors, menu popovers and app launches.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-90}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
command -v xdotool >/dev/null || { echo "error: xdotool missing; use nix develop .#smithay" >&2; exit 1; }

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-topbar.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/topbar-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# The bar is always on screen: hovering the clock yields a pointer cursor.
# 1280 px: the clock sits at x=1064..1208, y=6..32.
xdotool mousemove 1130 19
sleep 0.5
import -window root "${OUT}/topbar.png"
sleep 0.3
grep -q "cursor -> Pointer" "${OUT}/topbar-app.log" || {
  echo "error: menu bar did not produce a pointer cursor" >&2
  cat "${OUT}/topbar-app.log" >&2
  exit 1
}

# Clicking the clock opens the Calendar tray; its footer opens the app.
xdotool click 1
sleep 1.0
import -window root "${OUT}/topbar-menu.png"
xdotool mousemove 1054 375
sleep 0.3
xdotool click 1
sleep 1.0
grep -q "calendar opened" "${OUT}/topbar-app.log" || {
  echo "error: clock tray did not open the Calendar" >&2
  tail -20 "${OUT}/topbar-app.log" >&2
  exit 1
}
xdotool key Escape
sleep 0.5

# The dock launches applications: Terminal spawns a client, Search opens the
# palette. Tiles are 46 px wide starting at x=429.
xdotool mousemove 606 760
sleep 0.3
xdotool click 1
sleep 2
grep -q "spawned app terminal" "${OUT}/topbar-app.log" || {
  echo "error: dock Terminal did not spawn a terminal" >&2
  tail -20 "${OUT}/topbar-app.log" >&2
  exit 1
}
import -window root "${OUT}/topbar-terminal.png"

xdotool mousemove 502 760
sleep 0.3
xdotool click 1
sleep 1
import -window root "${OUT}/topbar-palette.png"
xdotool key Escape
sleep 0.4

cat "${OUT}/topbar-app.log"
echo "wrote ${OUT}/topbar.png, ${OUT}/topbar-menu.png and ${OUT}/topbar-terminal.png"