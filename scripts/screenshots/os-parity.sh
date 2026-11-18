#!/usr/bin/env bash
#
# Water OS design parity: capture the compositor states, capture the live
# reference, then diff them at 1280 x 800.
#
#   ./scripts/screenshots/os-parity.sh                 # docker capture + diff
#   ./scripts/screenshots/os-parity.sh --no-capture    # reuse existing captures
#   ./scripts/screenshots/os-parity.sh --reference     # recapture the live demo
#
# Outputs:
#   artifacts/os-parity/reference/   live demo screenshots
#   artifacts/os-parity/diff/        per-state diff images
#   artifacts/os-parity/report.md    mismatch by region
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${OUT:-${ROOT}/artifacts/screenshots/shell}"
IMAGE="${IMAGE:-aether-shell-capture:local}"
CONTAINER="${CONTAINER:-aether-parity}"
CAPTURE=1
REFERENCE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --no-capture) CAPTURE=0 ;;
    --reference) REFERENCE=1 ;;
    --help | -h)
      sed -n '2,20p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *) printf 'error: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

mkdir -p "${OUT}"
OUT="$(cd "${OUT}" && pwd)"

if [ "${CAPTURE}" = 1 ]; then
  if ! docker image inspect "${IMAGE}" >/dev/null 2>&1; then
    printf 'error: docker image %s is missing; run ./scripts/capture-shell-screenshots.sh --only palette\n' "${IMAGE}" >&2
    exit 1
  fi
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
  docker run -d --name "${CONTAINER}" \
    -e OUT=/home/aether/work \
    -v "${OUT}:/home/aether/work" \
    -v "${ROOT}/os/compositor:/src/os/compositor" \
    "${IMAGE}" sleep infinity >/dev/null
  trap 'docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true' EXIT

  printf '==> building the compositor in %s\n' "${CONTAINER}"
  docker exec "${CONTAINER}" bash -c \
    'export PATH=/usr/local/cargo/bin:$PATH; cd /src/os/compositor && cargo build --manifest-path aether-shell-smithay/Cargo.toml' \
    >/dev/null

  printf '==> capturing parity states\n'
  docker exec \
    -e OUT=/home/aether/work \
    -e DISPLAY_NUM=96 \
    -e AETHER_TERMINAL=/src/os/compositor/aether-shell-smithay/aether-terminal.sh \
    -e AETHER_TEST_CLIENT=/src/os/compositor/aether-shell-smithay/aether-terminal.sh \
    "${CONTAINER}" \
    bash /src/os/compositor/aether-shell-smithay/smoke-os-parity-xvfb.sh

  trap - EXIT
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
fi

if [ "${REFERENCE}" = 1 ] || [ ! -f "${ROOT}/artifacts/os-parity/reference/desktop.png" ]; then
  node "${ROOT}/scripts/screenshots/os-parity.mjs" --reference
fi

node "${ROOT}/scripts/screenshots/os-parity.mjs" --compare