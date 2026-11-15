#!/usr/bin/env bash
# Signaling + status smoke for `aether-remote serve`.
#
# Starts the host on loopback with a deterministic pairing code, checks /health,
# posts a hello, and asserts that a welcome and an SDP offer come back. The media
# source is synthetic; this proves the signaling and status path without a GPU.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-${ROOT}/artifacts/p7}"
PORT="${PORT:-18446}"
CODE="${CODE:-K7QM2H4P}"
BIN="${BIN:-${ROOT}/os/compositor/target/debug/aether-remote}"

mkdir -p "${OUT}"
cargo build -q --manifest-path "${ROOT}/os/compositor/Cargo.toml" -p aether-remote

"${BIN}" serve \
  --http "127.0.0.1:${PORT}" \
  --code "${CODE}" \
  --status "${OUT}/serve-status.json" \
  --seconds 8 \
  >"${OUT}/serve.log" 2>&1 &
SERVE_PID=$!
cleanup() {
  kill "${SERVE_PID}" 2>/dev/null || true
  wait "${SERVE_PID}" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 40); do
  if curl -sf "http://127.0.0.1:${PORT}/health" >"${OUT}/serve-health.json"; then
    break
  fi
  sleep 0.1
done

grep -q '"ok":true' "${OUT}/serve-health.json"
grep -q '"pairing_code"' "${OUT}/serve-hello.json" 2>/dev/null || true

curl -s -X POST \
  -H 'content-type: application/json' \
  --data "{\"type\":\"hello\",\"version\":1,\"pairing_code\":\"${CODE}\",\"client_name\":\"smoke\",\"udp_blocked\":false,\"caps\":{\"webrtc\":true,\"websocket\":true,\"webcodecs\":true,\"jpeg\":true,\"h264\":true,\"hevc\":false,\"av1\":true}}" \
  "http://127.0.0.1:${PORT}/signal" \
  -o "${OUT}/serve-hello.json"

grep -q '"welcome"' "${OUT}/serve-hello.json"
grep -q '"offer"' "${OUT}/serve-hello.json"
grep -q '"codec":"h264"' "${OUT}/serve-hello.json"
grep -q '"tier":"webrtc"' "${OUT}/serve-hello.json"

sleep 0.5
grep -q '"pairing_code"' "${OUT}/serve-status.json"
grep -q '"codec": "H.264"' "${OUT}/serve-status.json"

cat "${OUT}/serve.log"
echo "remote smoke: signaling ok, status published"
