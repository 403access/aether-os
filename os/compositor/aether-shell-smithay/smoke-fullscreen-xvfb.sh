#!/usr/bin/env bash
#
# Fullscreen: toggle the focused window with Super+F, assert the xdg state and
# capture the chrome-free output, then restore.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-fullscreen-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-86}"
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

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-fullscreen.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/fullscreen-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool mousemove 300 300 click 1
sleep 0.5

xdotool key super+f
sleep 1
import -window root "${OUT}/fullscreen.png"
xdotool key super+f
sleep 1
import -window root "${OUT}/fullscreen-restored.png"

if ! grep -q "fullscreen -> true" "${OUT}/fullscreen-app.log"; then
  echo "error: Super+F did not enter fullscreen" >&2
  cat "${OUT}/fullscreen-app.log" >&2
  exit 1
fi
if ! grep -q "fullscreen -> false" "${OUT}/fullscreen-app.log"; then
  echo "error: second Super+F did not restore the window" >&2
  cat "${OUT}/fullscreen-app.log" >&2
  exit 1
fi

cat "${OUT}/fullscreen-app.log"
echo "wrote ${OUT}/fullscreen.png and ${OUT}/fullscreen-restored.png"
