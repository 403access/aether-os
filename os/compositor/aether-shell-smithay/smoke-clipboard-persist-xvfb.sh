#!/usr/bin/env bash
#
# Clipboard persistence: wl-clip-persist re-owns a clipboard after the original
# source exits, so a second paste succeeds.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-clipboard-persist-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-76}"
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
command -v wl-clip-persist >/dev/null || {
  echo "error: wl-clip-persist missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export OUT
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT=2

FIRST="${OUT}/persist-first.txt"
SECOND="${OUT}/persist-second.txt"
rm -f "${FIRST}" "${SECOND}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-persist.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
PERSIST_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  [ -n "${PERSIST_PID}" ] && kill "${PERSIST_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${PERSIST_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/persist-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

SOCKET="$(grep -o 'WAYLAND_DISPLAY=[^ ]*' "${OUT}/persist-app.log" | head -1 | cut -d= -f2)"
WAYLAND_DISPLAY="${SOCKET}" wl-clip-persist --clipboard regular \
  >"${OUT}/persist-helper.log" 2>&1 &
PERSIST_PID=$!
sleep 1

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# Copy from the left terminal with a one-shot source.
xdotool mousemove 300 300 click 1
sleep 0.4
xdotool type --delay 25 "printf 'persist-live' | wl-copy --paste-once"
xdotool key Return
sleep 1

# First paste consumes the source, then the helper should keep it alive.
xdotool mousemove 900 300 click 1
sleep 0.4
xdotool type --delay 25 "wl-paste > \"${FIRST}\""
xdotool key Return
sleep 1
xdotool type --delay 25 "wl-paste > \"${SECOND}\""
xdotool key Return
sleep 2
import -window root "${OUT}/clipboard-persist.png"

if ! grep -q "persist-live" "${FIRST}"; then
  echo "error: first paste failed" >&2
  cat "${FIRST}" 2>/dev/null || true
  cat "${OUT}/persist-app.log" >&2
  cat "${OUT}/persist-helper.log" >&2
  exit 1
fi
if ! grep -q "persist-live" "${SECOND}"; then
  echo "error: clipboard did not persist after the source exited" >&2
  cat "${SECOND}" 2>/dev/null || true
  cat "${OUT}/persist-helper.log" >&2
  exit 1
fi

cat "${OUT}/persist-helper.log"
echo "clipboard persisted after source exit"
