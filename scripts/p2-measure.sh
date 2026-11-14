#!/usr/bin/env bash
#
# P2 reference measurement runbook (7700K). Collects the three gate metrics:
# sustained interval, input→damage and queue depth. Run from the native DRM
# session on the reference machine with the palette workload available.
#
#   DURATION=600 ./scripts/p2-measure.sh
#
# The working directory must be writable for artifacts/perf/p2.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${P2_OUT:-${ROOT}/artifacts/perf/p2}"
DURATION="${DURATION:-600}"
export AETHER_FRAME_STATS="${OUT}/frames.ndjson"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

mkdir -p "${OUT}"
say "preflight"
"${ROOT}/scripts/target-check.sh" || die "target preflight failed"
grep -q landlock /sys/kernel/security/lsm || die "landlock missing from LSM list"

say "start the native session (or attach to the running one), open the palette"
say "workload: ${DURATION}s of typing/workspace switching with the palette open"
if ! command -v wtype >/dev/null 2>&1; then
  say "wtype is not installed; drive the palette manually, then press Enter here"
  read -r _
else
  (
    sleep 2
    for i in $(seq 1 "${DURATION}"); do
      wtype "p2-${i}" || true
      sleep 1
    done
  ) &
  WORKLOAD=$!
fi

[ -n "${WORKLOAD:-}" ] && wait "${WORKLOAD}" || true
"${ROOT}/scripts/frame-stats.py" "${AETHER_FRAME_STATS}" | tee "${OUT}/summary.txt"
say "raw samples: ${OUT}/frames.ndjson"
say "record the machine, governor, kernel, connector mode and these numbers in claims-and-evidence.md"
