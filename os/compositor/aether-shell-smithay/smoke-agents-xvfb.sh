#!/usr/bin/env bash
#
# External launcher provider: publish an agent-session entry in a JSON manifest,
# find it in the palette and launch it.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-agents-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-84}"
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
export AETHER_TEST_CLIENT_COUNT=1

export XDG_DATA_HOME="${OUT}/agent-home"
export XDG_DATA_DIRS="${OUT}/agent-data"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications"
export AETHER_FILE_ROOTS="${OUT}/agent-files"
rm -rf "${AETHER_FILE_ROOTS}"
mkdir -p "${AETHER_FILE_ROOTS}"

AGENTS="${OUT}/agent-launchers.json"
cat >"${AGENTS}" <<'JSON'
[
  {
    "name": "Agent Session Alpha",
    "detail": "running - build watcher",
    "program": "weston-terminal",
    "args": []
  }
]
JSON
export AETHER_PALETTE_JSON="${AGENTS}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-agents.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/agents-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 1
xdotool type --delay 120 "agent"
sleep 1
import -window root "${OUT}/provider-agent.png"
xdotool key Return
sleep 2

if ! grep -q "loaded 1 desktop apps" "${OUT}/agents-app.log"; then
  echo "error: external launcher manifest was not loaded" >&2
  cat "${OUT}/agents-app.log" >&2
  exit 1
fi
if ! grep -q "palette action LaunchApp(0)" "${OUT}/agents-app.log"; then
  echo "error: agent manifest entry could not be launched" >&2
  cat "${OUT}/agents-app.log" >&2
  exit 1
fi
if ! grep -q "spawned app Agent Session Alpha (pid" "${OUT}/agents-app.log"; then
  echo "error: agent manifest command was not spawned" >&2
  cat "${OUT}/agents-app.log" >&2
  exit 1
fi

cat "${OUT}/agents-app.log"
echo "wrote ${OUT}/provider-agent.png"
