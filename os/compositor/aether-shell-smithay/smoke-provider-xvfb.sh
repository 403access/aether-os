#!/usr/bin/env bash
#
# Palette command provider: open the palette, run "New terminal" and assert a
# real Wayland client is spawned and tiled.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-provider-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-93}"
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

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-provider.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/provider-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 0.5
xdotool type --delay 120 "new"
sleep 1
import -window root "${OUT}/provider-new-terminal.png"
xdotool key Return
sleep 2
import -window root "${OUT}/provider-terminal-open.png"

if ! grep -q "palette action SpawnTerminal" "${OUT}/provider-app.log"; then
  echo "error: Enter did not run the new-terminal command" >&2
  cat "${OUT}/provider-app.log" >&2
  exit 1
fi
if ! grep -q "spawned terminal (pid" "${OUT}/provider-app.log"; then
  echo "error: terminal process was not spawned" >&2
  cat "${OUT}/provider-app.log" >&2
  exit 1
fi
connections="$(grep -c "client connected" "${OUT}/provider-app.log" || true)"
if [ "${connections}" -lt 2 ]; then
  echo "error: spawned terminal did not connect (${connections} clients)" >&2
  cat "${OUT}/provider-app.log" >&2
  exit 1
fi

cat "${OUT}/provider-app.log"
echo "wrote ${OUT}/provider-new-terminal.png and ${OUT}/provider-terminal-open.png"
