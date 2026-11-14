#!/usr/bin/env bash
#
# Desktop actions: expose a secondary .desktop action as a palette launcher.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-actions-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-78}"
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

export XDG_DATA_HOME="${OUT}/actions-home"
export XDG_DATA_DIRS="${OUT}/actions-data"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications"
cat >"${XDG_DATA_DIRS}/applications/actions-test.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Actions Test App
Comment=Action fixture
Exec=weston-terminal
Actions=Private;
Terminal=false

[Desktop Action Private]
Name=Private Window
Exec=weston-terminal
DESKTOP
export AETHER_FILE_ROOTS="${OUT}/actions-files"
rm -rf "${AETHER_FILE_ROOTS}"
mkdir -p "${AETHER_FILE_ROOTS}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-actions.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/actions-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+space
sleep 1
xdotool type --delay 100 "private"
sleep 1
import -window root "${OUT}/provider-action.png"
xdotool key Return
sleep 2

if ! grep -q "loaded 2 desktop apps" "${OUT}/actions-app.log"; then
  echo "error: desktop action was not exposed as a launcher" >&2
  cat "${OUT}/actions-app.log" >&2
  exit 1
fi
if ! grep -q "palette action LaunchApp(1)" "${OUT}/actions-app.log"; then
  echo "error: action launcher was not selected" >&2
  cat "${OUT}/actions-app.log" >&2
  exit 1
fi
if ! grep -q "spawned app Actions Test App - Private Window (pid" "${OUT}/actions-app.log"; then
  echo "error: action launcher did not spawn" >&2
  cat "${OUT}/actions-app.log" >&2
  exit 1
fi

cat "${OUT}/actions-app.log"
echo "wrote ${OUT}/provider-action.png"
