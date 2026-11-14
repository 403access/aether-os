#!/usr/bin/env bash
#
# Frame skipping: with no clients and no input, the compositor must stop
# presenting frames; a pointer event must produce another frame.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-idle-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-89}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-3}"

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
unset AETHER_TEST_CLIENT
export AETHER_FRAME_LOG=1

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-idle.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/idle-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

count_frames() {
  grep -c "aether-shell-smithay: frame " "${OUT}/idle-app.log" || true
}

before="$(count_frames)"
sleep 3
after="$(count_frames)"
if [ "${after}" -ne "${before}" ]; then
  echo "error: compositor kept rendering while idle (${before} -> ${after})" >&2
  cat "${OUT}/idle-app.log" >&2
  exit 1
fi

DISPLAY=":${DISPLAY_NUM}" xdotool mousemove 1000 700
sleep 0.5
moved="$(count_frames)"
if [ "${moved}" -le "${after}" ]; then
  echo "error: pointer input did not trigger a frame (${after} -> ${moved})" >&2
  cat "${OUT}/idle-app.log" >&2
  exit 1
fi

echo "idle frames: ${before} -> ${after}; after input: ${moved}"
