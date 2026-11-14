#!/usr/bin/env bash
#
# P4 audio harness: default-route switch latency and 64-frame xrun stress.
#
# Target run (on the reference machine, from a desktop session):
#   ./scripts/audio-stress.sh
#
# Dry run (no sound server; validates the harness and emits a skipped record):
#   AETHER_AUDIO_FIXTURE=$PWD/os/compositor/aether-shell-smithay/audio-fixture.json \
#     ./scripts/audio-stress.sh
#
# Results are NDJSON under artifacts/p4/. They are evidence only when produced on
# the reference machine; dev-host and fixture runs are labelled.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${AETHER_AUDIO_OUT:-${ROOT}/artifacts/p4}"
DURATION="${AETHER_AUDIO_STRESS_SECONDS:-600}"
SWITCHES="${AETHER_AUDIO_SWITCHES:-20}"
BIN="${AETHER_AUDIO_BIN:-aether-audio}"
MODE="live"
[ -n "${AETHER_AUDIO_FIXTURE:-}" ] && MODE="fixture"

mkdir -p "${OUT}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RESULT="${OUT}/audio-stress-${STAMP}.ndjson"
: >"${RESULT}"

emit() {
  printf '%s\n' "$1" | tee -a "${RESULT}"
}

if ! command -v "${BIN}" >/dev/null; then
  echo "error: ${BIN} not found (build .#aether-audio or use nix develop .#smithay)" >&2
  exit 1
fi

outputs=()
while read -r kind id _; do
  [ "${kind}" = "output" ] && outputs+=("${id}")
done < <("${BIN}" list --ids)
if [ "${#outputs[@]}" -lt 2 ]; then
  emit "{\"kind\":\"skipped\",\"mode\":\"${MODE}\",\"reason\":\"need two output routes\",\"routes\":${#outputs[@]}}"
  echo "wrote ${RESULT}"
  exit 0
fi

if [ "${MODE}" = "fixture" ]; then
  emit "{\"kind\":\"skipped\",\"mode\":\"fixture\",\"reason\":\"fixture mode does not touch PipeWire\"}"
  echo "wrote ${RESULT}"
  exit 0
fi

if ! command -v pw-metadata >/dev/null || ! command -v pw-top >/dev/null; then
  echo "error: pw-metadata/pw-top missing; run on the target or install pipewire" >&2
  exit 1
fi

# Route switch: alternate between the first two outputs and keep raw samples.
samples=()
for index in $(seq 1 "${SWITCHES}"); do
  target="${outputs[$((index % 2))]}"
  start="$(date +%s%N)"
  "${BIN}" switch "${target}" >/dev/null
  end="$(date +%s%N)"
  samples+=("$(( (end - start) / 1000000 ))")
done
summary="$(printf '%s\n' "${samples[@]}" | sort -n | awk '
  { values[NR] = $1; sum += $1 }
  END {
    n = NR
    p50 = values[int((n * 50 + 99) / 100)]
    p95 = values[int((n * 95 + 99) / 100)]
    printf "\"samples_ms\":%d,\"min_ms\":%d,\"p50_ms\":%d,\"p95_ms\":%d,\"max_ms\":%d", n, values[1], p50, p95, values[n]
  }')"
emit "{\"kind\":\"route-switch\",\"mode\":\"live\",\"routes\":[${outputs[0]},${outputs[1]}],${summary},\"budget_ms\":100}"

# 64-frame stress: force the quantum, sample ERR rows for DURATION seconds.
old_quantum="$( "${BIN}" list --json | sed -n 's/.*"preset": "\([a-z]*\)".*/\1/p' | head -1 || true)"
trap 'pw-metadata -n settings 0 clock.force-quantum 0 >/dev/null 2>&1 || true' EXIT
pw-metadata -n settings 0 clock.force-quantum 64 >/dev/null
start="$(date +%s)"
total=0
max=0
samples=0
while [ $(( $(date +%s) - start )) -lt "${DURATION}" ]; do
  xruns="$( "${BIN}" xruns )" || xruns=0
  total=$(( total + xruns ))
  [ "${xruns}" -gt "${max}" ] && max="${xruns}"
  samples=$(( samples + 1 ))
  sleep 1
done
emit "{\"kind\":\"xrun-stress\",\"mode\":\"live\",\"quantum\":64,\"duration_s\":$(( $(date +%s) - start )),\"samples\":${samples},\"xruns\":${total},\"max_xruns_per_sample\":${max},\"previous_preset\":\"${old_quantum:-unknown}\",\"budget\":0}"
echo "wrote ${RESULT}"
