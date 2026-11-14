#!/usr/bin/env bash
#
# File provider: index a fixture tree, search a filename in the palette and
# open it through a deterministic opener script.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-files-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-91}"
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

FILES="${OUT}/files-fixture"
rm -rf "${FILES}"
mkdir -p "${FILES}/notes" "${FILES}/target/debug"
printf 'a' >"${FILES}/notes-alpha.txt"
printf 'b' >"${FILES}/notes/notes-beta.txt"
printf 's' >"${FILES}/.secret"
printf 'i' >"${FILES}/target/debug/ignored.txt"
export AETHER_FILE_ROOTS="${FILES}"
export AETHER_FILE_LIMIT=100
export AETHER_FILE_DEPTH=3

OPEN_MARKER="${OUT}/opened-path.txt"
rm -f "${OPEN_MARKER}"
cat >"${OUT}/opener.sh" <<'OPENER'
#!/bin/sh
printf '%s\n' "$1" > "$AETHER_OPEN_MARKER"
OPENER
chmod +x "${OUT}/opener.sh"
export AETHER_OPENER="${OUT}/opener.sh"
export AETHER_OPEN_MARKER="${OPEN_MARKER}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-files.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/files-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 0.5
xdotool type --delay 120 "alpha"
sleep 1
import -window root "${OUT}/provider-file.png"
xdotool key Return
sleep 1

if ! grep -q "indexed 2 files" "${OUT}/files-app.log"; then
  echo "error: file fixture was not indexed as expected" >&2
  cat "${OUT}/files-app.log" >&2
  exit 1
fi
if ! grep -q "palette action OpenFile(0)" "${OUT}/files-app.log"; then
  echo "error: Enter did not open the filtered file" >&2
  cat "${OUT}/files-app.log" >&2
  exit 1
fi
if ! grep -q "opened .*notes-alpha.txt" "${OUT}/files-app.log"; then
  echo "error: opener was not invoked with the file path" >&2
  cat "${OUT}/files-app.log" >&2
  exit 1
fi
if ! grep -q "notes-alpha.txt" "${OPEN_MARKER}"; then
  echo "error: opener marker does not contain the file path" >&2
  cat "${OPEN_MARKER}" >&2 || true
  exit 1
fi

cat "${OUT}/files-app.log"
echo "wrote ${OUT}/provider-file.png"
