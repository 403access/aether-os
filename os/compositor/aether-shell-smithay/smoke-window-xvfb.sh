#!/usr/bin/env bash
#
# Window move/maximize: Alt+drag a tiled client to float and move it, then
# toggle maximize twice with Super+M.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-window-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-95}"
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

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-window.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/window-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# Alt+drag the stack client from (900,400) to (700,350). A tiled window is
# promoted to its 800x600 floating size, centred on the grab point.
xdotool keydown alt
sleep 0.3
xdotool mousemove 900 400
xdotool mousedown 1
sleep 0.3
xdotool mousemove 700 350
xdotool mouseup 1
xdotool keyup alt
sleep 1
import -window root "${OUT}/window-move.png"

if ! grep -q "window drag started" "${OUT}/window-app.log"; then
  echo "error: Alt+drag did not start a window move" >&2
  cat "${OUT}/window-app.log" >&2
  exit 1
fi
if ! grep -q "window moved id=" "${OUT}/window-app.log"; then
  echo "error: window move did not finish" >&2
  cat "${OUT}/window-app.log" >&2
  exit 1
fi

# Maximize, capture, then restore.
xdotool key super+m
sleep 1
import -window root "${OUT}/window-maximized.png"
xdotool key super+m
sleep 1
import -window root "${OUT}/window-restored.png"

if ! grep -q "maximized -> true" "${OUT}/window-app.log"; then
  echo "error: Super+M did not maximize the window" >&2
  cat "${OUT}/window-app.log" >&2
  exit 1
fi
if ! grep -q "maximized -> false" "${OUT}/window-app.log"; then
  echo "error: second Super+M did not restore the window" >&2
  cat "${OUT}/window-app.log" >&2
  exit 1
fi

cat "${OUT}/window-app.log"
echo "wrote ${OUT}/window-move.png, window-maximized.png, window-restored.png"
