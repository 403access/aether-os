#!/usr/bin/env bash
# Top shortcut dock: verify pointer cursor, palette launch and terminal launch.
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

# The dock is centred; the first two buttons are Palette and Terminal.
# 1280px: dock x=287, 96px items, 4px gaps.
xdotool mousemove 340 24
sleep 0.5
import -window root "${OUT}/topbar.png"
sleep 0.5
grep -q "cursor -> Pointer" "${OUT}/topbar-app.log" || {
  echo "error: top shortcut did not produce a pointer cursor" >&2
  cat "${OUT}/topbar-app.log" >&2
  exit 1
}

xdotool click 1
sleep 1
grep -q "top shortcut palette" "${OUT}/topbar-app.log" || {
  echo "error: Palette button did not open the palette" >&2
  cat "${OUT}/topbar-app.log" >&2
  exit 1
}
xdotool key Escape
sleep 0.5

xdotool mousemove 440 24
xdotool click 1
sleep 2
import -window root "${OUT}/topbar-terminal.png"
sleep 0.5
grep -q "top shortcut terminal" "${OUT}/topbar-app.log" || {
  echo "error: Terminal button did not fire" >&2
  cat "${OUT}/topbar-app.log" >&2
  exit 1
}
grep -q "spawned app terminal" "${OUT}/topbar-app.log" || {
  echo "error: Terminal button did not spawn a terminal" >&2
  cat "${OUT}/topbar-app.log" >&2
  exit 1
}

cat "${OUT}/topbar-app.log"
echo "wrote ${OUT}/topbar.png and ${OUT}/topbar-terminal.png"
