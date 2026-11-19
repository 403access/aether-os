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
# The demo's profile card shows this account; pin it for visual parity.
export AETHER_USER_NAME="${AETHER_USER_NAME:-Oliver Molnar}"
# App-only captures: keep the floating dock out of the window crops.
export AETHER_HIDE_DOCK="${AETHER_HIDE_DOCK:-1}"
FILES="${OUT}/system-fixture"
rm -rf "${FILES}"; mkdir -p "${FILES}/notes" "${FILES}/archive"
printf 'alpha\n' >"${FILES}/notes/alpha.md"
printf 'beta\n' >"${FILES}/beta.txt"
printf 'gamma\n' >"${FILES}/archive/gamma.json"
export AETHER_FILE_ROOTS="${FILES}"
# Emit the composed layout for the scene differ; shot() snapshots it per surface.
export AETHER_SCENE_DUMP="${AETHER_SCENE_DUMP:-${OUT}/scene.json}"
SCENES="${OUT}/scenes"; mkdir -p "${SCENES}"

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
shot() {
  # Park the pointer off the window and jiggle once so the compositor drops
  # the hand cursor from the previous click before the frame is captured.
  xdotool mousemove 5 795; sleep 0.2; xdotool mousemove 6 795; sleep 0.4
  import -window root "${OUT}/$1.png"
  cp "${AETHER_SCENE_DUMP}" "${SCENES}/aether-$1.json" 2>/dev/null || true
}

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
xdotool mousemove 1160 375; sleep 0.3; xdotool click 1; sleep 0.8; shot calendar
xdotool key Escape; sleep 0.4

# Settings, Tide Radio, Explorer and Task Manager from the Window menu.
# The dock is hidden for these captures, so it cannot be used to open apps.
window_item() {
  xdotool mousemove 285 19; sleep 0.3; xdotool click 1; sleep 0.5
  xdotool mousemove 300 $((66 + $1 * 32)); sleep 0.3; xdotool click 1; sleep 1.0
}
window_item 7; shot app-settings
# Settings has thirteen panes; capture each one at (127, 164 + i * 30).
SETTINGS_TABS=(general appearance displays desktop commands accessibility network bluetooth sound battery notifications security storage)
# Tab centres after the profile card and group labels:
# 6 System, 3 Connectivity, 4 Power & privacy.
SETTINGS_Y=(230 260 290 320 350 380 444 474 504 568 598 628 658)
for i in "${!SETTINGS_TABS[@]}"; do
  xdotool mousemove 136 "${SETTINGS_Y[$i]}"; sleep 0.25; xdotool click 1; sleep 0.45
  shot "app-settings-${SETTINGS_TABS[$i]}"
done
xdotool key Escape; sleep 0.4
window_item 8; shot app-radio
xdotool key Escape; sleep 0.4
window_item 1; shot app-explorer
xdotool key Escape; sleep 0.4
window_item 2; shot app-task-manager

# Explorer with a real fixture, dock hidden.
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+e; sleep 1.2; shot app-explorer
xdotool key Escape; sleep 0.4

echo "wrote app, tray, calendar and explorer captures to ${OUT}"
