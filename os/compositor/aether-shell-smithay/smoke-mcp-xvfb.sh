#!/usr/bin/env bash
#
# System MCP: control socket, audited tool calls and the chat tool loop.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-mcp-xvfb.sh
#
# Starts the compositor on Xvfb, runs a scripted aether-chat backend, prompts
# the drawer with xdotool, then drives the same tools over stdio with
# aether-mcp. Asserts window control, the confirmation gate and audit output.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
MCP="${MCP:-${ROOT}/../target/debug/aether-mcp}"
CHAT="${CHAT:-${ROOT}/../target/debug/aether-chat}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-89}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-3}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built; run cargo build --manifest-path ${ROOT}/Cargo.toml" >&2
  exit 1
}
[ -x "${MCP}" ] && [ -x "${CHAT}" ] || {
  echo "error: aether-mcp/aether-chat not built; run cargo build -p aether-mcp -p aether-chat" >&2
  exit 1
}
command -v xdotool >/dev/null || {
  echo "error: xdotool missing; use nix develop .#smithay" >&2
  exit 1
}
command -v import >/dev/null || {
  echo "error: ImageMagick import missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT=1
export AETHER_CONTROL_SOCKET="${XDG_RUNTIME_DIR}/aether-control.sock"
export AETHER_CHAT_SOCKET="${XDG_RUNTIME_DIR}/aether-chat.sock"
export AETHER_MCP_AUDIT="${OUT}/mcp-audit.jsonl"
rm -f "${AETHER_MCP_AUDIT}"

SCRIPT="${OUT}/mcp-script.json"
cat >"${SCRIPT}" <<'JSON'
{"turns":[
  {"content":"Checking.","tool_calls":[{"name":"aether_system","arguments":{}}]},
  {"content":"System check complete."}
]}
JSON

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-mcp.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
CHAT_PID=""
cleanup() {
  [ -n "${CHAT_PID}" ] && kill "${CHAT_PID}" 2>/dev/null || true
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${CHAT_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/mcp-app.log" 2>&1 &
APP_PID=$!
for _ in $(seq 1 100); do
  [ -S "${AETHER_CHAT_SOCKET}" ] && break
  sleep 0.1
done
[ -S "${AETHER_CHAT_SOCKET}" ] || {
  echo "error: chat socket did not appear" >&2
  cat "${OUT}/mcp-app.log" >&2
  exit 1
}

"${CHAT}" --socket "${AETHER_CHAT_SOCKET}" --mcp "${MCP}" --script "${SCRIPT}" \
  >"${OUT}/mcp-chat.log" 2>&1 &
CHAT_PID=$!
for _ in $(seq 1 50); do
  grep -q "chat backend connected" "${OUT}/mcp-app.log" && break
  sleep 0.1
done

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+c
sleep 1
xdotool type --delay 60 "check the system"
xdotool key Return
for _ in $(seq 1 50); do
  grep -q "chat events ->" "${OUT}/mcp-app.log" && break
  sleep 0.2
done
sleep "${SETTLE}"
import -window root "${OUT}/mcp-chat.png"

{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"aether_windows","arguments":{}}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"aether_open_surface","arguments":{"surface":"chat"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"aether_close_window","arguments":{"id":1}}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"aether_close_window","arguments":{"id":1,"confirm":true}}}'
} | "${MCP}" >"${OUT}/mcp-calls.jsonl" 2>"${OUT}/mcp-calls.err"

grep -q "Wayland Terminal" "${OUT}/mcp-calls.jsonl" || {
  echo "error: windows.list did not report the test client" >&2
  cat "${OUT}/mcp-calls.jsonl" >&2
  exit 1
}
grep -q "chat events ->" "${OUT}/mcp-app.log" || {
  echo "error: the chat backend produced no events" >&2
  cat "${OUT}/mcp-app.log" >&2
  cat "${OUT}/mcp-chat.log" >&2
  exit 1
}
grep -q '"isError":true' "${OUT}/mcp-calls.jsonl" || {
  echo "error: close without confirm was not rejected" >&2
  cat "${OUT}/mcp-calls.jsonl" >&2
  exit 1
}
grep -q '"closed":1' "${OUT}/mcp-calls.jsonl" || {
  echo "error: confirmed close did not reach the window" >&2
  cat "${OUT}/mcp-calls.jsonl" >&2
  exit 1
}
grep -q '"tool":"aether_close_window"' "${AETHER_MCP_AUDIT}" || {
  echo "error: audit log is missing tool calls" >&2
  cat "${AETHER_MCP_AUDIT}" >&2
  exit 1
}

echo "MCP control + chat loop passed"
echo "wrote ${OUT}/mcp-chat.png and ${OUT}/mcp-calls.jsonl"
