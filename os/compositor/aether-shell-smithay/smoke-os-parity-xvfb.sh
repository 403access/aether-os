#!/usr/bin/env bash
#
# Capture the two desktop states used by the Water OS design-parity check.
#
#   OUT=/home/aether/work ./smoke-os-parity-xvfb.sh
#
# Writes desktop.png (explorer open from the dock) and calendar.png (clock
# menu opens the Calendar app). The reference for both lives at
# https://403access.github.io/data-cloud-compare/design-system/os/ and is
# captured by scripts/screenshots/os-parity.mjs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-96}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
command -v xdotool >/dev/null || { echo "error: xdotool missing; use nix develop .#smithay" >&2; exit 1; }

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-${ROOT}/aether-terminal.sh}"
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-0}"
# Reproducible clock for comparisons: 2026-11-18 16:47 UTC.
export AETHER_FIXED_UNIX="${AETHER_FIXED_UNIX:-1795020420}"
# The dock is off by default; this capture exercises it.
export AETHER_SHOW_DOCK="${AETHER_SHOW_DOCK:-1}"
# Emit the composed layout for the scene differ.
export AETHER_SCENE_DUMP="${AETHER_SCENE_DUMP:-${OUT}/scene.json}"
SCENES="${OUT}/scenes"; mkdir -p "${SCENES}"
snap() { cp "${AETHER_SCENE_DUMP}" "${SCENES}/aether-$1.json" 2>/dev/null || true; }

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-parity.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/parity-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
# Park the pointer off the window and jiggle once so the compositor drops the
# hand cursor/hover state from the previous click before the frame is captured.
park() { xdotool mousemove 5 795; sleep 0.2; xdotool mousemove 6 795; sleep 0.4; }
park

# File menu popover, captured before any window is open so the app name and
# menu geometry are deterministic. The crop in app-parity.mjs is (110,0,250,215).
xdotool mousemove 125 19
sleep 0.4
xdotool click 1
sleep 0.8
park
import -window root "${OUT}/menu.png"
snap menu
xdotool key Escape
sleep 0.5
xdotool mousemove 5 795
sleep 0.3

# Explorer from the dock's first tile, then move the pointer off the chrome so
# no hover state is baked into the capture.
xdotool mousemove 445 760
sleep 0.3
xdotool click 1
sleep 1.5
park
import -window root "${OUT}/desktop.png"
snap desktop

# Escape closes the explorer overlay, then the clock opens the Calendar.
xdotool key Escape
sleep 0.6
xdotool mousemove 1130 19
sleep 0.3
xdotool click 1
sleep 1.2
park
import -window root "${OUT}/calendar.png"
snap tray-calendar

# Agent sandboxes (Super+B) is Aether-only; close the Calendar first.
xdotool key Escape
sleep 0.6
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+b
sleep 1.2
park
import -window root "${OUT}/sandboxes.png"
snap sandboxes

echo "wrote ${OUT}/desktop.png, ${OUT}/calendar.png and ${OUT}/sandboxes.png"