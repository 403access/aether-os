#!/usr/bin/env bash
#
# Partial damage: schedule client output with no input at commit time and
# assert the resulting frame is drawn with a partial damage rect.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-damage-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-88}"
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
export AETHER_TEST_CLIENT_COUNT=2
export AETHER_FRAME_LOG=1

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-damage.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/damage-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool mousemove 300 300 click 1
sleep 0.3
xdotool type --delay 25 "sleep 1; echo partial-damage-ok"
xdotool key Return
sleep 3
import -window root "${OUT}/partial-damage.png"

if ! grep -q "damage=partial" "${OUT}/damage-app.log"; then
  echo "error: client output did not produce a partial-damage frame" >&2
  cat "${OUT}/damage-app.log" >&2
  exit 1
fi

grep "damage=" "${OUT}/damage-app.log" | tail -8
echo "wrote ${OUT}/partial-damage.png"
