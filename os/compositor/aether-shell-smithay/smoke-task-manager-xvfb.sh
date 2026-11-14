#!/usr/bin/env bash
# Task manager: open from the palette, capture processes/users/resources tabs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-91}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
command -v xdotool >/dev/null || { echo "error: xdotool missing; use nix develop .#smithay" >&2; exit 1; }

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-task-manager.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/task-manager-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key ctrl+alt+space
sleep 0.5
xdotool type --delay 50 "task"
sleep 0.5
xdotool key Return
sleep 1
import -window root "${OUT}/task-manager.png"
grep -q "task manager opened" "${OUT}/task-manager-app.log" || {
  echo "error: task manager did not open from the palette" >&2
  cat "${OUT}/task-manager-app.log" >&2
  exit 1
}

xdotool key Tab
sleep 0.7
import -window root "${OUT}/task-manager-users.png"
xdotool key Tab
sleep 0.7
import -window root "${OUT}/task-manager-resources.png"

cat "${OUT}/task-manager-app.log"
echo "wrote task-manager.png, task-manager-users.png, task-manager-resources.png"
