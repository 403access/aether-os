#!/usr/bin/env bash
#
# Regenerate the browser-facing screenshots: marketing site, remote viewer,
# docs and the Water design-system gallery.
#
#   ./scripts/capture-web-screenshots.sh
#   ./scripts/capture-web-screenshots.sh --no-build
#   ./scripts/capture-web-screenshots.sh --out artifacts/screenshots/web
#
# The Playwright spec is skipped during normal `bun run test:e2e`; this wrapper
# opts in with UPDATE_SCREENSHOTS=1.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${SCREENSHOT_OUT:-${ROOT}/artifacts/screenshots/web}"
BUILD=1
GREP=""

usage() {
  cat <<'EOF'
Regenerate the browser screenshots with Playwright.

Options:
  --no-build     Reuse apps/web/dist (skip `bun run build`)
  --out DIR      Capture directory (default: artifacts/screenshots/web)
  --grep TEXT    Only run matching Playwright tests
  --help         Show this text
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) BUILD=0 ;;
    --out)
      OUT="${2:?--out needs a value}"
      shift
      ;;
    --grep)
      GREP="${2:?--grep needs a value}"
      shift
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    *) printf 'error: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
  shift
done

mkdir -p "${OUT}"
OUT="$(cd "${OUT}" && pwd)"
cd "${ROOT}"

if [ "${BUILD}" = 1 ]; then
  printf '\n==> building the assembled site\n'
  bun run build
fi

if [ ! -f "${ROOT}/apps/web/dist/index.html" ]; then
  printf 'error: apps/web/dist is missing; run the build first\n' >&2
  exit 1
fi

# A full run rewrites every named capture. Remove stale PNGs so a removed
# surface cannot linger in the published set. Targeted --grep runs keep them.
if [ -z "${GREP}" ]; then
  find "${OUT}" -maxdepth 1 -type f -name '*.png' -delete
fi

printf '\n==> capturing browser screenshots into %s\n' "${OUT}"
args=(e2e/screenshots.spec.ts --workers=1)
if [ -n "${GREP}" ]; then
  args+=(--grep "${GREP}")
fi

UPDATE_SCREENSHOTS=1 SCREENSHOT_OUT="${OUT}" bunx playwright test "${args[@]}"

printf '\n==> %s PNG capture(s) in %s\n' \
  "$(find "${OUT}" -maxdepth 1 -name '*.png' | wc -l | tr -d ' ')" "${OUT}"
printf '\nnext: ./scripts/publish-screenshots.sh --web-only\n'