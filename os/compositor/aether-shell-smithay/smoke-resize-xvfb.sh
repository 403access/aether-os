#!/usr/bin/env bash
#
# Interactive layout resize: drag the master/stack divider with xdotool and
# assert the shell applied the new ratio.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-resize-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-97}"
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

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-resize.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/resize-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

# With 1280x800, outer_gap 16, gap 10 and ratio 0.55 the divider is at
# x=697..707. Drag from its centre to x=900 (ratio ~0.71).
export DISPLAY=":${DISPLAY_NUM}"
xdotool mousemove 702 400
xdotool mousedown 1
xdotool mousemove 900 400
xdotool mouseup 1
sleep 1

import -window root "${OUT}/resize-drag.png"

if ! grep -q "resize drag started" "${OUT}/resize-app.log"; then
  echo "error: divider press did not start a resize drag" >&2
  cat "${OUT}/resize-app.log" >&2
  exit 1
fi
if ! grep -q "master ratio -> 0.71" "${OUT}/resize-app.log"; then
  echo "error: drag did not apply a ~0.71 master ratio" >&2
  cat "${OUT}/resize-app.log" >&2
  exit 1
fi

cat "${OUT}/resize-app.log"
echo "wrote ${OUT}/resize-drag.png"
