#!/usr/bin/env bash
#
# Client-to-client drag and drop: drag a weston-dnd flower from the left client
# to the right client and assert the drop/data transfer.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-dnd-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-75}"
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
export WAYLAND_DEBUG=1
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-dnd}"
export AETHER_TEST_CLIENT_COUNT=2

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-dnd.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/dnd-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# A flower source sits near the top-left of the first client.
xdotool mousemove 184 120
sleep 0.4
xdotool mousedown 1
sleep 0.4
xdotool mousemove 900 400
sleep 0.4
xdotool mouseup 1
sleep 1
import -window root "${OUT}/dnd-drop.png"

if ! grep -q "dnd_drop_performed" "${OUT}/dnd-app.log"; then
  echo "error: drag never reached a drop" >&2
  tail -60 "${OUT}/dnd-app.log" >&2
  exit 1
fi
if ! grep -q "application/x-wayland-dnd-flower" "${OUT}/dnd-app.log"; then
  echo "error: no flower data transfer" >&2
  tail -60 "${OUT}/dnd-app.log" >&2
  exit 1
fi

grep -E "start_drag|dnd_drop_performed|application/x-wayland-dnd-flower" "${OUT}/dnd-app.log" | tail -10
echo "wrote ${OUT}/dnd-drop.png"
