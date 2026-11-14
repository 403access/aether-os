#!/usr/bin/env bash
#
# Palette input: open the overlay under Xvfb, type a query with xdotool,
# capture the filtered results and activate the selected workspace.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-palette-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-96}"
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
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-1}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-palette.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" --overlay >"${OUT}/palette-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool mousemove 0 0
xdotool key Up Up
sleep 1
import -window root "${OUT}/palette-default.png"
xdotool type --delay 120 "web"
sleep 1
import -window root "${OUT}/palette-query.png"

# Click the first result row (workspace "web") instead of pressing Enter.
xdotool mousemove 600 300 click 1
sleep 1

if ! grep -q 'palette query -> "web"' "${OUT}/palette-app.log"; then
  echo "error: typed query did not reach the palette" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi
if ! grep -q "palette action SwitchWorkspace(2)" "${OUT}/palette-app.log"; then
  echo "error: row click did not activate the web workspace" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi
if ! grep -q "workspace -> 2" "${OUT}/palette-app.log"; then
  echo "error: palette activation did not switch workspace" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi

# Reopen and filter to the client window provider, then activate with Enter.
xdotool key super+space
sleep 1
xdotool type --delay 120 "term"
sleep 1
import -window root "${OUT}/palette-window.png"
xdotool key Return
sleep 1

if ! grep -q "palette action FocusWindow(1)" "${OUT}/palette-app.log"; then
  echo "error: Enter did not focus the filtered window" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi
if ! grep -q "workspace -> 0" "${OUT}/palette-app.log"; then
  echo "error: focusing the window did not return to workspace 0" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi

cat "${OUT}/palette-app.log"
echo "wrote ${OUT}/palette-query.png and ${OUT}/palette-window.png"
