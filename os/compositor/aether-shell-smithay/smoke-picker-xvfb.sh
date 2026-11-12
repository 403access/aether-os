#!/usr/bin/env bash
#
# Workspace picker: start the compositor under Xvfb, click the second
# workspace indicator with xdotool and assert the shell switched workspace.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-picker-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-98}"
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
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-2}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-picker.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/picker-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

# The second workspace chip starts at x=86 and spans 66x26. Clicking it
# switches to the empty "code" workspace.
export DISPLAY=":${DISPLAY_NUM}"
xdotool mousemove 100 20 click 1
sleep 1
DISPLAY=":${DISPLAY_NUM}" import -window root "${OUT}/workspace-picker.png"

if ! grep -q "workspace -> 1" "${OUT}/picker-app.log"; then
  echo "error: picker click did not switch to workspace 1" >&2
  cat "${OUT}/picker-app.log" >&2
  exit 1
fi

cat "${OUT}/picker-app.log"
echo "wrote ${OUT}/workspace-picker.png"
