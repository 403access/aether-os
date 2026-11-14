#!/usr/bin/env bash
#
# Live check for the Rust aether-index daemon: socket queries, inotify create/
# remove updates and the file-backed event ring.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-${ROOT}/os/index/aether-index/target/debug/aether-index}"
OUT="${OUT:-${ROOT}/artifacts/p3}"
WORK="$(mktemp -d)"
RING="${WORK}/events.ring"
SOCKET="${WORK}/index.sock"

mkdir -p "${OUT}" "${WORK}/root"
printf 'alpha document\n' >"${WORK}/root/alpha.txt"
printf 'notes\n' >"${WORK}/root/notes.md"

[ -x "${BIN}" ] || { echo "error: build ${BIN}" >&2; exit 1; }
command -v bun >/dev/null || { echo "error: bun required" >&2; exit 1; }

export XDG_RUNTIME_DIR="${WORK}"
AETHER_INDEX_ROOTS="${WORK}/root" \
AETHER_INDEX_SOCKET="${SOCKET}" \
AETHER_INDEX_RING="${RING}" \
"${BIN}" >"${OUT}/index.log" 2>&1 &
PID=$!
cleanup() { kill "${PID}" 2>/dev/null || true; wait "${PID}" 2>/dev/null || true; rm -rf "${WORK}"; }
trap cleanup EXIT

for _ in $(seq 1 50); do
  [ -S "${SOCKET}" ] && grep -q "aether-index: ready" "${OUT}/index.log" && break
  sleep 0.1
done
[ -S "${SOCKET}" ] || { echo "error: daemon did not start" >&2; cat "${OUT}/index.log" >&2; exit 1; }

query() {
  bun -e '
    const net = require("net");
    const socket = net.connect(process.argv[1]);
    socket.write(JSON.stringify({ query: process.argv[2], limit: 8 }) + "\n");
    socket.on("data", (data) => { process.stdout.write(data.toString()); socket.end(); });
    socket.on("error", (error) => { console.error(error); process.exit(1); });
  ' "${SOCKET}" "$1"
}

alpha="$(query alpha)"
echo "${alpha}" | grep -q "alpha.txt" || { echo "error: alpha missing: ${alpha}" >&2; exit 1; }

printf 'beta live\n' >"${WORK}/root/beta.txt"
for _ in $(seq 1 50); do
  beta="$(query beta)"
  echo "${beta}" | grep -q "beta.txt" && break
  sleep 0.1
done
echo "${beta}" | grep -q "beta.txt" || { echo "error: inotify create missing: ${beta}" >&2; exit 1; }

rm "${WORK}/root/beta.txt"
for _ in $(seq 1 50); do
  beta="$(query beta)"
  echo "${beta}" | grep -q "beta.txt" || break
  sleep 0.1
done
echo "${beta}" | grep -q "beta.txt" && { echo "error: inotify remove missing" >&2; exit 1; }

tail="$("${BIN}" --ring-tail "${RING}")"
echo "${tail}" | grep -q "alpha.txt" || { echo "error: ring missing alpha: ${tail}" >&2; exit 1; }
echo "${tail}" | grep -q "Removed.*beta.txt" || { echo "error: ring missing removal: ${tail}" >&2; exit 1; }

{
  echo "socket query alpha: ${alpha}"
  echo "socket query beta (after create): ${beta}"
  echo "ring events:"
  echo "${tail}"
} | tee "${OUT}/index-smoke.log"
echo "wrote ${OUT}/index-smoke.log"
