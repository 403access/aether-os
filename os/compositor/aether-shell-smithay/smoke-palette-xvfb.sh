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
# Debug scene recorder (capture builds only).
export AETHER_SCENE_DUMP="${AETHER_SCENE_DUMP:-${OUT}/scene.json}"
SCENES="${OUT}/scenes"; mkdir -p "${SCENES}"
snap() { cp "${AETHER_SCENE_DUMP}" "${SCENES}/aether-$1.json" 2>/dev/null || true; }
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-${ROOT}/aether-terminal.sh}"
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
# Poll the compositor log instead of guessing with fixed sleeps; the palette
# and the test client are asynchronous under Xvfb.
wait_for() {
  local pattern="$1" timeout="${2:-6}" tick
  for tick in $(seq 1 $((timeout * 10))); do
    grep -q "${pattern}" "${OUT}/palette-app.log" && return 0
    sleep 0.1
  done
  return 1
}

WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool mousemove 0 0
xdotool key Up Up
sleep 1
import -window root "${OUT}/palette-default.png"
snap palette-default
xdotool type --delay 120 "web"
wait_for 'palette query -> "web"' || true
sleep 0.3
import -window root "${OUT}/palette-query.png"
snap palette-query

# Click the first result row (workspace "web") instead of pressing Enter.
xdotool mousemove 600 300 click 1
if ! wait_for "palette action SwitchWorkspace(2)"; then
  echo "error: row click did not activate the web workspace" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi
if ! wait_for "workspace -> 2"; then
  echo "error: palette activation did not switch workspace" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi

# Reopen and filter to the client window provider, then activate with Enter.
wait_for "client connected" 10 || true
xdotool key super+space
sleep 1
xdotool type --delay 120 "wayland"
wait_for 'palette query -> "wayland"' || true
sleep 0.3
import -window root "${OUT}/palette-window.png"
snap palette-window
xdotool key Return

if ! wait_for "palette action FocusWindow(1)"; then
  echo "error: Enter did not focus the filtered window" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi
if ! wait_for "workspace -> 0"; then
  echo "error: focusing the window did not return to workspace 0" >&2
  cat "${OUT}/palette-app.log" >&2
  exit 1
fi

cat "${OUT}/palette-app.log"
echo "wrote ${OUT}/palette-query.png and ${OUT}/palette-window.png"
