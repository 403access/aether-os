#!/usr/bin/env bash
# Chat drawer: stream markdown tokens for ~3s, capture the drawer and assert
# that presentation continued while tokens arrived.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-86}"
CHAT_SECONDS="${CHAT_SECONDS:-5}"
MIN_FRAMES="${MIN_FRAMES:-15}"
[ -x "${BIN}" ] || { echo "error: ${BIN} not built" >&2; exit 1; }
mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
STATS="${OUT}/chat-frames.ndjson"
rm -f "${STATS}"
Xvfb ":${DISPLAY_NUM}" -screen 0 1280x800x24 -ac >"${OUT}/xvfb-chat.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" AETHER_CHAT_DEMO=1 AETHER_FRAME_STATS="${STATS}" \
  "${BIN}" >"${OUT}/chat-app.log" 2>&1 &
APP_PID=$!
sleep "${CHAT_SECONDS}"
DISPLAY=":${DISPLAY_NUM}" import -window root "${OUT}/chat.png"
FRAMES="$(grep -c '"kind":"present"' "${STATS}" || true)"
[ "${FRAMES}" -ge "${MIN_FRAMES}" ] || {
  echo "error: only ${FRAMES} frames in ${CHAT_SECONDS}s while streaming (need ${MIN_FRAMES})" >&2
  exit 1
}
echo "streamed chat, ${FRAMES} presented frames"
echo "wrote ${OUT}/chat.png"
