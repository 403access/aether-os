#!/usr/bin/env bash
# Remote overlay: open with Super+R, toggle input and clipboard, then stop the
# session and capture both the live and closed states. Uses a fixture snapshot.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-92}"
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
export AETHER_REMOTE_FIXTURE=1

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-remote.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/remote-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+r
sleep 1
import -window root "${OUT}/remote.png"

xdotool key Down
xdotool key Return
sleep 0.4
xdotool key i
xdotool key c
sleep 0.4
import -window root "${OUT}/remote-toggles.png"

xdotool key s
sleep 0.4
import -window root "${OUT}/remote-stopped.png"

grep -q "remote overlay opened" "${OUT}/remote-app.log" || {
  echo "error: remote overlay did not open" >&2
  cat "${OUT}/remote-app.log" >&2
  exit 1
}
grep -q "remote input -> false" "${OUT}/remote-app.log" || {
  echo "error: input toggle was not applied" >&2
  cat "${OUT}/remote-app.log" >&2
  exit 1
}
grep -q "remote clipboard -> true" "${OUT}/remote-app.log" || {
  echo "error: clipboard toggle was not applied" >&2
  cat "${OUT}/remote-app.log" >&2
  exit 1
}
grep -q "remote session stopped" "${OUT}/remote-app.log" || {
  echo "error: session stop was not applied" >&2
  cat "${OUT}/remote-app.log" >&2
  exit 1
}

xdotool key Escape
sleep 0.3
cat "${OUT}/remote-app.log"
echo "wrote ${OUT}/remote.png, remote-toggles.png and remote-stopped.png"
