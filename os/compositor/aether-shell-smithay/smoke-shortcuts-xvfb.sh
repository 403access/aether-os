#!/usr/bin/env bash
# Shortcut UX: open the in-shell cheat sheet with Super+/, capture it, then
# launch a terminal with Super+Return and assert the fast path fired.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-89}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
command -v xdotool >/dev/null || { echo "error: xdotool missing; use nix develop .#smithay" >&2; exit 1; }

mkdir -p "${OUT}"
# Debug scene recorder (capture builds only).
export AETHER_SCENE_DUMP="${AETHER_SCENE_DUMP:-${OUT}/scene.json}"
SCENES="${OUT}/scenes"; mkdir -p "${SCENES}"
snap() { cp "${AETHER_SCENE_DUMP}" "${SCENES}/aether-$1.json" 2>/dev/null || true; }
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-shortcuts.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/shortcuts-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key ctrl+alt+slash
sleep 1
import -window root "${OUT}/shortcuts.png"
snap shortcuts
grep -q "shortcuts overlay opened" "${OUT}/shortcuts-app.log" || {
  echo "error: shortcuts overlay did not open" >&2
  cat "${OUT}/shortcuts-app.log" >&2
  exit 1
}

xdotool key Escape
sleep 0.5
xdotool key super+Return
sleep 2
import -window root "${OUT}/shortcut-terminal.png"
snap shortcut-terminal
grep -q "shortcut new terminal" "${OUT}/shortcuts-app.log" || {
  echo "error: Super+Return did not trigger the terminal fast path" >&2
  cat "${OUT}/shortcuts-app.log" >&2
  exit 1
}
grep -q "spawned app terminal" "${OUT}/shortcuts-app.log" || {
  echo "error: Super+Return did not spawn a terminal" >&2
  cat "${OUT}/shortcuts-app.log" >&2
  exit 1
}

cat "${OUT}/shortcuts-app.log"
echo "wrote ${OUT}/shortcuts.png and ${OUT}/shortcut-terminal.png"
