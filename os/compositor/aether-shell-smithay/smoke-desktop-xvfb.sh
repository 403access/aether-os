#!/usr/bin/env bash
#
# Desktop-app provider: point XDG data dirs at a fixture, open the palette,
# run the discovered application and assert a real Wayland client is spawned.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-desktop-xvfb.sh
#
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
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-1}"

# Isolated application database with one launchable entry.
export XDG_DATA_HOME="${OUT}/xdg-home"
export XDG_DATA_DIRS="${OUT}/xdg-data"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications"
cat >"${XDG_DATA_DIRS}/applications/aether-test.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Aether Test App
Comment=Fixture launcher
Exec=weston-terminal
Terminal=false
DESKTOP

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-desktop.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/desktop-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 0.5
xdotool type --delay 120 "test"
sleep 1
import -window root "${OUT}/provider-desktop.png"
xdotool key Return
sleep 2
import -window root "${OUT}/provider-desktop-open.png"

if ! grep -q "loaded 1 desktop apps" "${OUT}/desktop-app.log"; then
  echo "error: desktop fixture was not discovered" >&2
  cat "${OUT}/desktop-app.log" >&2
  exit 1
fi
if ! grep -q "palette action LaunchApp(0)" "${OUT}/desktop-app.log"; then
  echo "error: Enter did not launch the discovered app" >&2
  cat "${OUT}/desktop-app.log" >&2
  exit 1
fi
if ! grep -q "spawned app Aether Test App (pid" "${OUT}/desktop-app.log"; then
  echo "error: discovered app process was not spawned" >&2
  cat "${OUT}/desktop-app.log" >&2
  exit 1
fi
connections="$(grep -c "client connected" "${OUT}/desktop-app.log" || true)"
if [ "${connections}" -lt 2 ]; then
  echo "error: launched app did not connect (${connections} clients)" >&2
  cat "${OUT}/desktop-app.log" >&2
  exit 1
fi

cat "${OUT}/desktop-app.log"
echo "wrote ${OUT}/provider-desktop.png and ${OUT}/provider-desktop-open.png"
