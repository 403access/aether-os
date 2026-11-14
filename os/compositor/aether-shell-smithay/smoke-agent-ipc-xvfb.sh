#!/usr/bin/env bash
#
# Live agent IPC: publish a session snapshot over AETHER_AGENT_SOCKET, then
# find and launch it from the palette.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-agent-ipc-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-77}"
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
export AETHER_AGENT_SOCKET="${XDG_RUNTIME_DIR}/agent-sessions.sock"
unset AETHER_PALETTE_JSON
export AETHER_FILE_ROOTS="${OUT}/agent-ipc-files"
rm -rf "${AETHER_FILE_ROOTS}"
mkdir -p "${AETHER_FILE_ROOTS}"

export XDG_DATA_HOME="${OUT}/agent-ipc-home"
export XDG_DATA_DIRS="${OUT}/agent-ipc-data"
rm -rf "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}"
mkdir -p "${XDG_DATA_HOME}" "${XDG_DATA_DIRS}/applications"

cat >"${OUT}/agent-publisher.ts" <<'PUBLISHER'
const socket = process.env.AETHER_AGENT_SOCKET!;
for (let attempt = 0; attempt < 50; attempt++) {
  try {
    const stream = await Bun.connect({
      unix: socket,
      socket: { data() {} },
    });
    stream.write(
      JSON.stringify([
        {
          name: "Live Agent Session",
          detail: "running - ipc test",
          program: "weston-terminal",
          args: [],
        },
      ]) + "\n",
    );
    await Bun.sleep(15000);
    process.exit(0);
  } catch {
    await Bun.sleep(100);
  }
}
process.exit(1);
PUBLISHER

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-agent-ipc.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
PUBLISHER_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  [ -n "${PUBLISHER_PID}" ] && kill "${PUBLISHER_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${PUBLISHER_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/agent-ipc-app.log" 2>&1 &
APP_PID=$!
for _ in $(seq 1 50); do
  [ -S "${AETHER_AGENT_SOCKET}" ] && break
  sleep 0.1
done
bun run "${OUT}/agent-publisher.ts" >"${OUT}/agent-publisher.log" 2>&1 &
PUBLISHER_PID=$!
for _ in $(seq 1 50); do
  grep -q "agent sessions ->" "${OUT}/agent-ipc-app.log" && break
  sleep 0.2
done
sleep "${SETTLE}"

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+space
sleep 1
xdotool type --delay 80 "live agent"
sleep 1
import -window root "${OUT}/provider-agent-ipc.png"
xdotool key Return
sleep 2

if ! grep -q "agent sessions ->" "${OUT}/agent-ipc-app.log"; then
  echo "error: agent snapshot did not reach the compositor" >&2
  cat "${OUT}/agent-ipc-app.log" >&2
  cat "${OUT}/agent-publisher.log" >&2
  exit 1
fi
if ! grep -q "palette action LaunchApp(0)" "${OUT}/agent-ipc-app.log"; then
  echo "error: agent session could not be launched" >&2
  cat "${OUT}/agent-ipc-app.log" >&2
  exit 1
fi
if ! grep -q "spawned app Live Agent Session (pid" "${OUT}/agent-ipc-app.log"; then
  echo "error: agent session command did not spawn" >&2
  cat "${OUT}/agent-ipc-app.log" >&2
  exit 1
fi

cat "${OUT}/agent-ipc-app.log"
echo "wrote ${OUT}/provider-agent-ipc.png"
