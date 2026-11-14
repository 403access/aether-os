#!/usr/bin/env bash
# Virtualised explorer: open with Super+E, filter, capture, open the selected file.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-84}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"
[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
command -v xdotool >/dev/null || { echo "error: xdotool missing" >&2; exit 1; }
mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

FILES="${OUT}/explorer-fixture"
rm -rf "${FILES}"; mkdir -p "${FILES}/notes"
printf 'alpha\n' >"${FILES}/notes/alpha.md"
printf 'beta\n' >"${FILES}/beta.txt"

OPEN_MARKER="${OUT}/explorer-opener.txt"
cat >"${OUT}/explorer-opener.sh" <<OPEN
#!/usr/bin/env bash
echo "\$1" >>"${OPEN_MARKER}"
OPEN
chmod +x "${OUT}/explorer-opener.sh"
export AETHER_FILE_ROOTS="${FILES}"
export AETHER_OPENER="${OUT}/explorer-opener.sh"
export AETHER_OPEN_MARKER="${OPEN_MARKER}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-explorer.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/explorer-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"
export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+e
sleep 1
import -window root "${OUT}/explorer.png"
xdotool type --delay 120 "alpha"
sleep 1
import -window root "${OUT}/explorer-filtered.png"
xdotool key Return
sleep 1
grep -q "alpha.md" "${OPEN_MARKER}" || { echo "error: explorer did not open the selected file" >&2; exit 1; }
if command -v convert >/dev/null 2>&1; then
  "${ROOT}/../../../scripts/check-explorer-screenshot.sh" "${OUT}/explorer.png"
fi
echo "wrote ${OUT}/explorer.png and explorer-filtered.png"
