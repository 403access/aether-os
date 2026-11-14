#!/usr/bin/env bash
#
# System index service: start the Bun indexer on a fixture, query it from the
# palette by file content and open the result.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-index-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "${ROOT}/../../.." && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-81}"
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
command -v bun >/dev/null || {
  echo "error: bun missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT=1

FILES="${OUT}/index-fixture"
rm -rf "${FILES}"
mkdir -p "${FILES}/notes"
printf 'contains the indexer magic word\n' >"${FILES}/notes/notes-alpha.txt"
printf 'unrelated content\n' >"${FILES}/other.txt"

SOCKET="${XDG_RUNTIME_DIR}/aether-index.sock"
export AETHER_INDEX_ROOTS="${FILES}"
export AETHER_INDEX_SOCKET="${SOCKET}"
export AETHER_INDEX_LIMIT=100
export AETHER_FILE_ROOTS="${FILES}"

OPEN_MARKER="${OUT}/index-opened-path.txt"
rm -f "${OPEN_MARKER}"
cat >"${OUT}/index-opener.sh" <<'OPENER'
#!/bin/sh
printf '%s\n' "$1" > "$AETHER_OPEN_MARKER"
OPENER
chmod +x "${OUT}/index-opener.sh"
export AETHER_OPENER="${OUT}/index-opener.sh"
export AETHER_OPEN_MARKER="${OPEN_MARKER}"

export XDG_DATA_HOME="${OUT}/index-home"
export XDG_DATA_DIRS="${OUT}/index-data"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications"

bun run "${REPO}/apps/indexer-service/src/main.ts" \
  >"${OUT}/index-service.log" 2>&1 &
INDEX_PID=$!

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-index.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  [ -n "${INDEX_PID}" ] && kill "${INDEX_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${INDEX_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 50); do
  [ -S "${SOCKET}" ] && grep -q "aether-index: ready" "${OUT}/index-service.log" && break
  sleep 0.2
done
if ! grep -q "aether-index: ready" "${OUT}/index-service.log"; then
  echo "error: index service did not start" >&2
  cat "${OUT}/index-service.log" >&2
  exit 1
fi

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/index-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

xdotool key super+space
sleep 1
xdotool type --delay 80 "magic"
sleep 1
import -window root "${OUT}/provider-index.png"
xdotool key Return
sleep 1

if ! grep -q 'palette query -> "magic"' "${OUT}/index-app.log"; then
  echo "error: palette query did not reach the compositor" >&2
  cat "${OUT}/index-app.log" >&2
  exit 1
fi
if ! grep -q "palette action OpenFile(0)" "${OUT}/index-app.log"; then
  echo "error: index result could not be opened" >&2
  cat "${OUT}/index-app.log" >&2
  cat "${OUT}/index-service.log" >&2
  exit 1
fi
if ! grep -q "notes-alpha.txt" "${OPEN_MARKER}"; then
  echo "error: opener was not invoked with the indexed file" >&2
  cat "${OPEN_MARKER}" >&2 || true
  cat "${OUT}/index-app.log" >&2
  exit 1
fi

cat "${OUT}/index-service.log"
cat "${OUT}/index-app.log"
echo "wrote ${OUT}/provider-index.png"
