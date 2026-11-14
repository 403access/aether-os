#!/usr/bin/env bash
#
# Cursor feedback: move the pointer over chrome and assert the winit cursor
# icon changes (resize divider, workspace chip, window, Alt-move, palette).
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-cursor-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-94}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built; run cargo build --manifest-path ${ROOT}/Cargo.toml" >&2
  exit 1
}
command -v xdotool >/dev/null || {
  echo "error: xdotool missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-2}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-cursor.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/cursor-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

assert_cursor() {
  local expected="$1"
  sleep 0.4
  if ! grep -q "cursor -> ${expected}" "${OUT}/cursor-app.log"; then
    echo "error: missing cursor ${expected}" >&2
    cat "${OUT}/cursor-app.log" >&2
    exit 1
  fi
}

# Master/stack divider.
xdotool mousemove 702 400
assert_cursor ColResize

# Workspace chip.
xdotool mousemove 100 20
assert_cursor Pointer

# Client window, then Alt over it.
xdotool mousemove 400 400
assert_cursor Default
xdotool keydown alt
xdotool mousemove 401 400
assert_cursor Move
xdotool keyup alt

# Palette row and input row.
xdotool key super+space
sleep 0.5
xdotool mousemove 500 300
assert_cursor Pointer
xdotool mousemove 500 255
assert_cursor Text
import -window root "${OUT}/software-cursor.png"
xdotool key Escape

cat "${OUT}/cursor-app.log"
echo "wrote ${OUT}/software-cursor.png"
