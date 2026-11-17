#!/usr/bin/env bash
#
# One entry point for every screenshot in the project.
#
#   ./scripts/capture-screenshots.sh                 # shell + web, then publish
#   ./scripts/capture-screenshots.sh --web-only
#   ./scripts/capture-screenshots.sh --shell-only
#   ./scripts/capture-screenshots.sh --no-build      # reuse the existing site build
#
# Outputs:
#   artifacts/screenshots/shell/        Xvfb compositor captures
#   artifacts/screenshots/web/          Playwright browser captures
#   artifacts/screenshots/index.html    contact sheet
#   artifacts/screenshots/manifest.json dimensions, hashes and git revision
#
# Publishing copies the curated images into apps/web/public/screenshots and
# apps/docs/static/img. The final build folds them into apps/web/dist.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DO_SHELL=1
DO_WEB=1
DO_PUBLISH=1
DO_BUILD=1

usage() {
  cat <<'EOF'
Regenerate all Aether screenshots.

Options:
  --shell-only   Only capture the compositor surfaces
  --web-only     Only capture the browser surfaces
  --no-publish   Leave captures in artifacts/screenshots
  --no-build     Reuse the current apps/web/dist for web captures
  --no-rebuild   Do not rebuild the Docker capture image
  --help         Show this text
EOF
}

NO_REBUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --shell-only)
      DO_WEB=0
      ;;
    --web-only)
      DO_SHELL=0
      ;;
    --no-publish)
      DO_PUBLISH=0
      ;;
    --no-build)
      DO_BUILD=0
      ;;
    --no-rebuild)
      NO_REBUILD=1
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    *) printf 'error: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
  shift
done

cd "${ROOT}"
status=0

if [ "${DO_SHELL}" = 1 ]; then
  if [ "${NO_REBUILD}" = 1 ]; then
    "${ROOT}/scripts/capture-shell-screenshots.sh" --no-rebuild || status=$?
  else
    "${ROOT}/scripts/capture-shell-screenshots.sh" || status=$?
  fi

  # Publish before the browser pass so the marketing gallery renders this
  # run's compositor captures rather than the previous set.
  if [ "${DO_PUBLISH}" = 1 ] && [ "${DO_WEB}" = 1 ]; then
    bun run "${ROOT}/scripts/publish-screenshots.ts" --shell-only || status=$?
  fi
fi

if [ "${DO_WEB}" = 1 ]; then
  if [ "${DO_BUILD}" = 1 ]; then
    "${ROOT}/scripts/capture-web-screenshots.sh"
  else
    "${ROOT}/scripts/capture-web-screenshots.sh" --no-build
  fi || status=$?
fi

if [ "${DO_PUBLISH}" = 1 ]; then
  bun run "${ROOT}/scripts/publish-screenshots.ts" || status=$?
fi

if [ "${DO_BUILD}" = 1 ] && [ "${DO_PUBLISH}" = 1 ]; then
  printf '\n==> rebuilding the assembled site with the new images\n'
  bun run build || status=$?
fi

printf '\n==> screenshots ready\n'
printf '  contact sheet: artifacts/screenshots/index.html\n'
printf '  manifest:      artifacts/screenshots/manifest.json\n'
exit "${status}"