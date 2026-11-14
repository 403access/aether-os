#!/usr/bin/env bash
#
# Clipboard: copy in one terminal with wl-copy and paste in the other with
# wl-paste. Exercises data-device focus forwarding between clients.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-clipboard-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-90}"
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
command -v wl-copy >/dev/null || {
  echo "error: wl-copy missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export OUT
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT="${AETHER_TEST_CLIENT_COUNT:-2}"

PASTE_FILE="${OUT}/clipboard-paste.txt"
PRIMARY_FILE="${OUT}/primary-paste.txt"
rm -f "${PASTE_FILE}" "${PRIMARY_FILE}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-clipboard.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/clipboard-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# Focus the left terminal, set the clipboard.
xdotool mousemove 300 300 click 1
sleep 0.5
xdotool type --delay 25 "printf 'aether-clip-test' | wl-copy"
xdotool key Return
sleep 1

# Focus the right terminal and paste into a file.
xdotool mousemove 900 300 click 1
sleep 0.5
xdotool type --delay 25 "wl-paste > \"${PASTE_FILE}\""
xdotool key Return
sleep 2
import -window root "${OUT}/clipboard-paste.png"

if [ ! -f "${PASTE_FILE}" ]; then
  echo "error: wl-paste did not create ${PASTE_FILE}" >&2
  cat "${OUT}/clipboard-app.log" >&2
  exit 1
fi
if ! grep -q "aether-clip-test" "${PASTE_FILE}"; then
  echo "error: pasted content did not match the copied text" >&2
  cat "${PASTE_FILE}" >&2
  cat "${OUT}/clipboard-app.log" >&2
  exit 1
fi

# Repeat with the primary selection (middle-click clipboard).
xdotool mousemove 300 300 click 1
sleep 0.5
xdotool type --delay 25 "printf 'aether-primary-test' | wl-copy --primary"
xdotool key Return
sleep 1

xdotool mousemove 900 300 click 1
sleep 0.5
xdotool type --delay 25 "wl-paste --primary > \"${PRIMARY_FILE}\""
xdotool key Return
sleep 2
import -window root "${OUT}/clipboard-primary.png"

if [ ! -f "${PRIMARY_FILE}" ]; then
  echo "error: wl-paste --primary did not create ${PRIMARY_FILE}" >&2
  cat "${OUT}/clipboard-app.log" >&2
  exit 1
fi
if ! grep -q "aether-primary-test" "${PRIMARY_FILE}"; then
  echo "error: primary selection content did not match" >&2
  cat "${PRIMARY_FILE}" >&2
  cat "${OUT}/clipboard-app.log" >&2
  exit 1
fi

cat "${OUT}/clipboard-app.log"
echo "clipboard content: $(cat "${PASTE_FILE}")"
echo "primary content: $(cat "${PRIMARY_FILE}")"
