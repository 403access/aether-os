#!/usr/bin/env bash
# Audio overlay: open with Super+A, switch the default output, cycle a latency
# preset and capture both states. Uses a fixture so no sound server is needed.
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
export AETHER_AUDIO_FIXTURE="${AETHER_AUDIO_FIXTURE:-${ROOT}/audio-fixture.json}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-audio.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/audio-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+a
sleep 1
import -window root "${OUT}/audio.png"

xdotool key Down
xdotool key Return
sleep 0.5
xdotool key p
sleep 0.5
import -window root "${OUT}/audio-preset.png"

grep -q "audio overlay opened" "${OUT}/audio-app.log" || {
  echo "error: audio overlay did not open" >&2
  cat "${OUT}/audio-app.log" >&2
  exit 1
}
grep -q "audio default output -> USB Headset (43)" "${OUT}/audio-app.log" || {
  echo "error: route switch was not applied" >&2
  cat "${OUT}/audio-app.log" >&2
  exit 1
}
grep -q "audio preset -> voice (256)" "${OUT}/audio-app.log" || {
  echo "error: latency preset was not applied" >&2
  cat "${OUT}/audio-app.log" >&2
  exit 1
}

xdotool key Escape
sleep 0.3
cat "${OUT}/audio-app.log"
echo "wrote ${OUT}/audio.png and ${OUT}/audio-preset.png"
