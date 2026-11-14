#!/usr/bin/env bash
# MRU focus: two clients, Super+Tab cycles the focus history.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-83}"
[ -x "${BIN}" ] || { echo "error: build ${BIN}" >&2; exit 1; }
mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT=2
Xvfb ":${DISPLAY_NUM}" -screen 0 1280x800x24 -ac >"${OUT}/xvfb-focus.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/focus-app.log" 2>&1 &
APP_PID=$!
sleep 5
export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+Tab
sleep 0.5
xdotool key super+Tab
sleep 0.5
grep -q "mru focus" "${OUT}/focus-app.log" || { echo "error: no MRU focus log" >&2; cat "${OUT}/focus-app.log" >&2; exit 1; }
grep "mru focus" "${OUT}/focus-app.log"
