#!/usr/bin/env bash
#
# Provider refresh: install a desktop entry and a file while the compositor is
# running, then open the palette and assert both providers pick them up.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-refresh-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-87}"
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

export XDG_DATA_HOME="${OUT}/refresh-home"
export XDG_DATA_DIRS="${OUT}/refresh-data"
FILES="${OUT}/refresh-files"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}" "${FILES}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications" "${FILES}"
export AETHER_FILE_ROOTS="${FILES}"
export AETHER_FILE_LIMIT=100
export AETHER_FILE_DEPTH=3

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-refresh.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/refresh-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

if ! grep -q "loaded 0 desktop apps" "${OUT}/refresh-app.log"; then
  echo "error: compositor did not start with an empty app database" >&2
  cat "${OUT}/refresh-app.log" >&2
  exit 1
fi

# Install a desktop entry and a file while the compositor is running.
cat >"${XDG_DATA_DIRS}/applications/refresh-test.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Refresh Test App
Comment=Installed at runtime
Exec=weston-terminal
Terminal=false
DESKTOP
printf 'new' >"${FILES}/new-note.txt"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 1
xdotool type --delay 120 "refresh"
sleep 1
import -window root "${OUT}/provider-refresh.png"
xdotool key Return
sleep 2

if ! grep -q "reloaded 1 desktop apps, 1 files" "${OUT}/refresh-app.log"; then
  echo "error: opening the palette did not refresh both providers" >&2
  cat "${OUT}/refresh-app.log" >&2
  exit 1
fi
if ! grep -q "palette action LaunchApp(0)" "${OUT}/refresh-app.log"; then
  echo "error: refreshed app could not be launched" >&2
  cat "${OUT}/refresh-app.log" >&2
  exit 1
fi

cat "${OUT}/refresh-app.log"
echo "wrote ${OUT}/provider-refresh.png"
