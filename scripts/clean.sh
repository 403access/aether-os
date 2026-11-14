#!/usr/bin/env bash
#
# Remove generated state from an Aether checkout.
#
# Default: build outputs, Rust target dirs, test artifacts, Nix results and
# artifacts/ (screenshots/evidence/VM images). Dependencies stay installed.
#
#   ./scripts/clean.sh --dry-run        # list what would be removed
#   ./scripts/clean.sh --yes            # clean build artifacts
#   ./scripts/clean.sh --all --yes      # also node_modules and local caches
#   ./scripts/clean.sh --docker --yes   # also remove the aether-vm-nix volume
#
# Nothing outside the repository is touched. Source, docs and committed images
# are never removed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DRY_RUN=0
YES=0
ALL=0
DOCKER=0

usage() {
  cat <<'EOF'
Usage: ./scripts/clean.sh [options]

  --dry-run   print targets without deleting
  --yes       skip the confirmation prompt
  --all       also remove node_modules, .bun and .direnv caches
  --docker    remove the aether-vm-nix Docker volume (explicit opt-in)
  -h, --help  show this help

Default targets:
  artifacts/             screenshots, fixtures and VM images
  os/compositor/target   Rust build cache
  apps/web/dist          assembled web/docs site
  apps/docs/build        Docusaurus output
  apps/docs/.docusaurus  Docusaurus cache
  coverage/, playwright-report/, test-results/, blob-report/
  packages/bench-results/
  result, result-*       Nix build symlinks
  **/.turbo, **/.cache, **/.vite
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --yes|-y) YES=1 ;;
    --all) ALL=1 ;;
    --docker) DOCKER=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

cd "${ROOT}"

TARGETS=(
  "artifacts"
  "os/compositor/target"
  "apps/web/dist"
  "apps/docs/build"
  "apps/docs/.docusaurus"
  "coverage"
  "playwright-report"
  "test-results"
  "blob-report"
  "packages/bench-results"
  "os/compositor/aether-shell-drm/drm-probe.log"
)

if [ "${ALL}" = "1" ]; then
  TARGETS+=("node_modules" ".bun" ".direnv")
  for path in apps/*/node_modules packages/*/node_modules; do
    [ -e "${path}" ] && TARGETS+=("${path}")
  done
fi

# Find build/cache directories without walking node_modules (or the already
# listed target trees) and accidentally treating dependency internals as
# separate targets.
while IFS= read -r path; do
  TARGETS+=("${path#./}")
done < <(find . \( -name node_modules -o -name .git \) -prune -o \
  -type d \( -name target -o -name .turbo -o -name .cache -o -name .vite \) \
  -prune -print 2>/dev/null)
while IFS= read -r path; do
  TARGETS+=("${path#./}")
done < <(find . -maxdepth 1 -name 'result' -o -maxdepth 1 -name 'result-*' 2>/dev/null)

# Unique, existing paths only.
UNIQUE=()
while IFS= read -r path; do
  [ -n "${path}" ] || continue
  [ -e "${path}" ] || continue
  UNIQUE+=("${path}")
done < <(printf '%s\n' "${TARGETS[@]}" | sort -u | awk '!seen[$0]++')

printf 'Aether clean — %s\n' "${ROOT}"
if [ "${#UNIQUE[@]}" -eq 0 ]; then
  echo "Nothing to clean."
  exit 0
fi

TOTAL=0
for path in "${UNIQUE[@]}"; do
  size="$(du -sh "${path}" 2>/dev/null | awk '{print $1}')"
  printf '  %8s  %s\n' "${size:-?}" "${path}"
done

if [ "${DRY_RUN}" = "1" ]; then
  echo "Dry run; nothing removed."
  exit 0
fi

if [ "${YES}" != "1" ]; then
  if [ ! -t 0 ]; then
    echo "error: non-interactive clean requires --yes" >&2
    exit 2
  fi
  printf 'Remove these paths? [y/N] '
  read -r answer
  case "${answer}" in
    y|Y|yes|YES) ;;
    *) echo "Aborted."; exit 0 ;;
  esac
fi

for path in "${UNIQUE[@]}"; do
  rm -rf -- "${path}"
done

if [ "${DOCKER}" = "1" ]; then
  if command -v docker >/dev/null 2>&1 && docker volume inspect aether-vm-nix >/dev/null 2>&1; then
    docker volume rm aether-vm-nix >/dev/null
    echo "removed Docker volume aether-vm-nix"
  else
    echo "Docker volume aether-vm-nix not present; skipped"
  fi
fi

echo "Clean complete."
git status --short 2>/dev/null || true
