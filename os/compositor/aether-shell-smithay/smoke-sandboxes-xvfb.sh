#!/usr/bin/env bash
#
# Agent sandboxes surface: registry rows, stale detection and stop.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-sandboxes-xvfb.sh
#
# Uses --no-uid so the smoke can run without root. Production launchers drop to
# a reserved UID and are stopped through the per-sandbox control socket.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
SANDBOX="${SANDBOX:-${ROOT}/../target/debug/aether-sandbox}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-92}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built" >&2
  exit 1
}
[ -x "${SANDBOX}" ] || {
  echo "error: ${SANDBOX} not built; run cargo build -p aether-sandbox" >&2
  exit 1
}
command -v xdotool >/dev/null || {
  echo "error: xdotool missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_SANDBOX_REGISTRY="${OUT}/sandbox-registry"
export AETHER_CONTROL_SOCKET="${XDG_RUNTIME_DIR}/aether-control.sock"
rm -rf "${AETHER_SANDBOX_REGISTRY}"
mkdir -p "${AETHER_SANDBOX_REGISTRY}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-sandboxes.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
SANDBOX_PIDS=()
cleanup() {
  for pid in "${SANDBOX_PIDS[@]:-}"; do kill "${pid}" 2>/dev/null || true; done
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
"${SANDBOX}" run --no-uid --id build --label "Build agent" \
  --read /tmp --write /tmp --timeout 60 -- "$(command -v sleep)" 60 \
  >"${OUT}/sandbox-build.log" 2>&1 &
SANDBOX_PIDS+=($!)
"${SANDBOX}" run --no-uid --id research --label "Research agent" \
  --read /tmp --write /tmp --net --timeout 60 -- "$(command -v sleep)" 60 \
  >"${OUT}/sandbox-research.log" 2>&1 &
SANDBOX_PIDS+=($!)
for _ in $(seq 1 50); do
  [ -f "${AETHER_SANDBOX_REGISTRY}/build.json" ] &&
    [ -f "${AETHER_SANDBOX_REGISTRY}/research.json" ] && break
  sleep 0.1
done
# Clone one record with a dead PID to exercise the stale row.
sed -e 's/"id": "build"/"id": "stale-demo"/' \
  -e 's/"label": "Build agent"/"label": "Crashed agent"/' \
  -e 's/"pid": [0-9]*/"pid": 999999/' \
  -e 's/"started_unix": [0-9]*/"started_unix": 1/' \
  "${AETHER_SANDBOX_REGISTRY}/build.json" >"${AETHER_SANDBOX_REGISTRY}/stale-demo.json"

DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/sandbox-app.log" 2>&1 &
APP_PID=$!
sleep 3
export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"
xdotool key super+b
sleep 2
import -window root "${OUT}/agent-sandboxes.png"
# The first row is selected; stop it through the launcher socket.
xdotool key k
sleep 2
import -window root "${OUT}/agent-sandboxes-stopped.png"

grep -q "agent sandboxes opened (3 records, 2 running)" "${OUT}/sandbox-app.log" || {
  echo "error: sandbox panel did not open with the expected records" >&2
  cat "${OUT}/sandbox-app.log" >&2
  exit 1
}
grep -q "SIGTERM sent to Build agent" "${OUT}/sandbox-app.log" || {
  echo "error: stop did not reach the sandbox" >&2
  cat "${OUT}/sandbox-app.log" >&2
  exit 1
}
[ ! -f "${AETHER_SANDBOX_REGISTRY}/build.json" ] || {
  echo "error: stopped sandbox record was not removed" >&2
  ls -l "${AETHER_SANDBOX_REGISTRY}" >&2
  exit 1
}
[ -f "${AETHER_SANDBOX_REGISTRY}/stale-demo.json" ] || {
  echo "error: stale record disappeared unexpectedly" >&2
  exit 1
}

echo "agent sandboxes smoke passed"
echo "wrote ${OUT}/agent-sandboxes.png"
