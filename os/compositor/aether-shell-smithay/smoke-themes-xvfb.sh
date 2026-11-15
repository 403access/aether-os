#!/usr/bin/env bash
# Themes app: open with Super+T, cycle the Water tone and capture both states.
# Uses a throwaway theme config so the run never touches the developer's file.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-89}"
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
export AETHER_THEME_CONFIG="${AETHER_THEME_CONFIG:-${OUT}/themes-fixture.json}"
rm -f "${AETHER_THEME_CONFIG}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-themes.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/themes-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+t
sleep 1
import -window root "${OUT}/themes.png"

# Theme, water effects, drips, ripples, caustics, row motion, speed, drip
# energy, background, tone. Walk to the tone row and cycle Ocean -> Deep.
for _ in $(seq 1 9); do
  xdotool key Down
  sleep 0.1
done
xdotool key Right
sleep 0.8
import -window root "${OUT}/themes-tone.png"

grep -q "themes opened" "${OUT}/themes-app.log" || {
  echo "error: themes overlay did not open" >&2
  cat "${OUT}/themes-app.log" >&2
  exit 1
}
grep -q "theme Changed -> .*Deep" "${OUT}/themes-app.log" || {
  echo "error: water tone did not change" >&2
  cat "${OUT}/themes-app.log" >&2
  exit 1
}
grep -q '"tone": "deep"' "${AETHER_THEME_CONFIG}" || {
  echo "error: theme config did not persist the new tone" >&2
  cat "${AETHER_THEME_CONFIG}" >&2
  exit 1
}

xdotool key Escape
sleep 0.3
cat "${OUT}/themes-app.log"
echo "wrote ${OUT}/themes.png and ${OUT}/themes-tone.png"
