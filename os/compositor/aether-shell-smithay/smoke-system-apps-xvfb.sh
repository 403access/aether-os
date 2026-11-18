#!/usr/bin/env bash
# First-party apps and tray popovers: capture each window on its own.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-95}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_FIXED_UNIX="${AETHER_FIXED_UNIX:-1795020420}"
# App-only captures: keep the floating dock out of the window crops.
export AETHER_HIDE_DOCK="${AETHER_HIDE_DOCK:-1}"
FILES="${OUT}/system-fixture"
rm -rf "${FILES}"; mkdir -p "${FILES}/notes" "${FILES}/archive"
printf 'alpha\n' >"${FILES}/notes/alpha.md"
printf 'beta\n' >"${FILES}/beta.txt"
printf 'gamma\n' >"${FILES}/archive/gamma.json"
export AETHER_FILE_ROOTS="${FILES}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-system.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
cleanup() { [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true; kill "${XVFB_PID}" 2>/dev/null || true; }
trap cleanup EXIT
sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/system-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"
export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
shot() { xdotool mousemove 640 400; sleep 0.3; import -window root "${OUT}/$1.png"; }

xdotool mousemove 20 19; sleep 0.3; xdotool click 1; sleep 0.8; shot app-about
xdotool key Escape; sleep 0.4

# Network + Wi-Fi tray.
xdotool mousemove 905 19; sleep 0.3; xdotool click 1; sleep 0.8; shot tray-network
xdotool mousemove 770 353; sleep 0.3; xdotool click 1; sleep 0.8; shot app-network
xdotool key Escape; sleep 0.4

# Devices + Bluetooth tray.
xdotool mousemove 941 19; sleep 0.3; xdotool click 1; sleep 0.8; shot tray-devices
xdotool mousemove 806 277; sleep 0.3; xdotool click 1; sleep 0.8; shot app-devices
xdotool key Escape; sleep 0.4

# Battery + tray.
xdotool mousemove 990 19; sleep 0.3; xdotool click 1; sleep 0.8; shot tray-battery
xdotool mousemove 879 295; sleep 0.3; xdotool click 1; sleep 0.8; shot app-battery
xdotool key Escape; sleep 0.4

# Sound tray.
xdotool mousemove 1040 19; sleep 0.3; xdotool click 1; sleep 0.8; shot tray-sound
xdotool key Escape; sleep 0.4

# Calendar tray, then the app.
xdotool mousemove 1130 19; sleep 0.3; xdotool click 1; sleep 0.8; shot tray-calendar
xdotool mousemove 1054 375; sleep 0.3; xdotool click 1; sleep 0.8; shot calendar
xdotool key Escape; sleep 0.4

# Settings and Tide Radio from the dock.
xdotool mousemove 533 760; sleep 0.3; xdotool click 1; sleep 0.8; shot app-settings
xdotool key Escape; sleep 0.4
xdotool mousemove 640 760; sleep 0.3; xdotool click 1; sleep 0.8; shot app-radio
xdotool key Escape; sleep 0.4

# Task Manager from Window.
xdotool mousemove 280 19; sleep 0.3; xdotool click 1; sleep 0.5
xdotool mousemove 300 104; sleep 0.3; xdotool click 1; sleep 1.0; shot app-task-manager

# Explorer with a real fixture, dock hidden.
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+e; sleep 1.2; shot app-explorer
xdotool key Escape; sleep 0.4

echo "wrote app, tray, calendar and explorer captures to ${OUT}"
