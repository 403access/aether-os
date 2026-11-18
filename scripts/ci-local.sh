#!/usr/bin/env bash
#
# Run the GitHub workflow jobs on this machine, before spending CI minutes.
#
# Each job replays the exact commands from .github/workflows/*.yml in order.
# Steps are identical; only the runner host differs (this Mac/box instead of
# ubuntu-latest), so a pass here is strong evidence and a failure is the real
# error you would otherwise read from a red log ten minutes later.
#
#   ./scripts/ci-local.sh                    # verify, rust, e2e, bench, pages
#   ./scripts/ci-local.sh --job rust         # one job
#   ./scripts/ci-local.sh --job rust --job e2e
#   ./scripts/ci-local.sh --all              # also the Nix-only smithay job
#   ./scripts/ci-local.sh --no-install       # skip the bun install steps
#   ./scripts/ci-local.sh --dry-run          # print the commands, run nothing
#   ./scripts/ci-local.sh --native           # rust clippy/test off Linux too
#   ./scripts/ci-local.sh --list             # job names and their workflow
#
# The rust job is only authoritative on Linux: parts of the compositor are
# cfg-gated per target, so clippy on a Mac compiles stub branches that CI never
# sees. Off Linux, clippy and test are skipped with that reason instead of
# reporting a failure CI would not reproduce. --native runs them anyway.
#
# Not reproduced, because the runner is the point of the job:
#   drm-vm      needs [self-hosted, linux, arm64, kvm] + virtiofsd
#   screenshots needs the Nix smithay shell and Xvfb; publish needs a real token
#   nix         evaluating the flake on a Mac is not the same as building it
#
# On failure the remaining steps of that job are skipped, as in CI, but the
# following jobs still run so one invocation shows every broken job.
#
# Exit status: 0 pass, 1 failure, 2 usage/environment error.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JOBS=()
WANT_ALL=0
NO_INSTALL=0
DRY_RUN=0
LIST=0
NATIVE=0

usage() {
  # Print the header comment block, which ends at the first `set` line.
  awk 'NR > 1 && /^set / { exit } NR > 1 { sub(/^# ?/, ""); print }' "${BASH_SOURCE[0]}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --job)
      [ $# -ge 2 ] || { echo "error: --job needs a name" >&2; exit 2; }
      JOBS+=("${2}")
      shift 2
      ;;
    --all) WANT_ALL=1; shift ;;
    --no-install) NO_INSTALL=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --native) NATIVE=1; shift ;;
    --list) LIST=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

job_exists() {
  case "$1" in
    verify | rust | e2e | bench | pages | smithay) return 0 ;;
    *) return 1 ;;
  esac
}

if [ "$LIST" -eq 1 ]; then
  cat <<'EOF'
verify   ci.yml        bun install, typecheck, unit tests, build
rust     ci.yml        cargo fmt, clippy, test (os/compositor; clippy/test Linux-only)
e2e      ci.yml        bun install, playwright chromium, build, playwright test
bench    ci.yml        bun install, bench --check --tolerance=3
pages    pages.yml     typecheck, unit tests, build with BASE_PATH/DOCS_BASE_URL
smithay  smithay.yml   Nix shell: fmt, clippy, build, test, curated Xvfb smokes

skipped by design: drm-vm, screenshots, nix (self-hosted / Nix / token gated)
EOF
  exit 0
fi

if [ "${#JOBS[@]}" -eq 0 ]; then
  JOBS=(verify rust e2e bench pages)
  [ "$WANT_ALL" -eq 1 ] && JOBS=(verify rust e2e bench pages smithay)
else
  for job in "${JOBS[@]}"; do
    job_exists "$job" || {
      echo "error: unknown job: $job (see --list)" >&2
      exit 2
    }
  done
  if [ "$WANT_ALL" -eq 1 ]; then
    for job in verify rust e2e bench pages smithay; do
      printf '%s\n' "${JOBS[@]}" | grep -qx "$job" || JOBS+=("$job")
    done
  fi
fi

have() { command -v "$1" >/dev/null 2>&1; }

need() {
  have "$1" || {
    echo "error: $1 not found ($2)" >&2
    exit 2
  }
}

cd "${ROOT}" || exit 2

BASE="$(git remote get-url origin 2>/dev/null | sed 's#/*$##; s#.*/##; s#\.git$##')"
[ -n "${BASE}" ] || BASE="aether-os"

RESULTS=()
CURRENT_JOB=""
CURRENT_WORKFLOW=""
JOB_FAILED=0

run_step() {
  local label="$1"
  shift
  if [ "${JOB_FAILED}" -eq 1 ]; then
    printf '  skip    %s\n' "${label}"
    RESULTS+=("${CURRENT_JOB}/${label}|skipped")
    return 0
  fi
  printf '\n  ==> %s\n' "${label}"
  printf '      $ %s\n' "$*"
  if [ "${DRY_RUN}" -eq 1 ]; then
    RESULTS+=("${CURRENT_JOB}/${label}|dry-run")
    return 0
  fi
  local start="${SECONDS}"
  if "$@"; then
    printf '      ok (%ss)\n' "$((SECONDS - start))"
    RESULTS+=("${CURRENT_JOB}/${label}|pass")
  else
    local rc=$?
    JOB_FAILED=1
    TOTAL_FAILED=$((TOTAL_FAILED + 1))
    printf '      FAILED (exit %s)\n' "${rc}" >&2
    RESULTS+=("${CURRENT_JOB}/${label}|fail")
  fi
}

# Used by jobs whose runner has a tool this host may not have.
skip_job() {
  local reason="$1"
  printf '  skip    %s\n' "${reason}"
  RESULTS+=("${CURRENT_JOB}/*|skipped")
}

skip_step() {
  local label="$1" reason="$2"
  printf '  skip    %s (%s)\n' "${label}" "${reason}"
  RESULTS+=("${CURRENT_JOB}/${label}|skipped")
}

install_step() {
  if [ "${NO_INSTALL}" -eq 1 ]; then
    printf '  skip    bun install --frozen-lockfile (--no-install)\n'
    RESULTS+=("${CURRENT_JOB}/install|skipped")
    return 0
  fi
  run_step "install" bun install --frozen-lockfile
}

# --- jobs ------------------------------------------------------------------

job_verify() {
  CURRENT_JOB="verify"
  CURRENT_WORKFLOW="ci.yml"
  need bun https://bun.sh
  install_step
  run_step "typecheck" bun run typecheck
  run_step "unit and DOM tests" bun run test:unit
  run_step "build site and docs" bun run build
}

job_rust() {
  CURRENT_JOB="rust"
  CURRENT_WORKFLOW="ci.yml"
  need cargo https://rustup.rs
  local manifest="os/compositor/Cargo.toml"
  run_step "format" cargo fmt --manifest-path "${manifest}" --all -- --check
  # sample_linux() and its helpers are #[cfg(target_os = "linux")], so a Mac
  # compiles the stub branch and clippy reports dead code CI never sees.
  if [ "$(uname -s)" = "Linux" ] || [ "${NATIVE}" -eq 1 ]; then
    run_step "clippy" cargo clippy --manifest-path "${manifest}" --all-targets -- -D warnings
    run_step "test" cargo test --manifest-path "${manifest}"
  else
    skip_step "clippy" "cfg-gated stubs make this a false red off Linux; --native to run"
    skip_step "test" "the Linux paths are what CI exercises; --native to run"
  fi
}

job_e2e() {
  CURRENT_JOB="e2e"
  CURRENT_WORKFLOW="ci.yml"
  need bun https://bun.sh
  install_step
  # Playwright's installer is idempotent; skip the network check when the
  # browser cache is already populated.
  local cache="${PLAYWRIGHT_BROWSERS_PATH:-}"
  if [ -z "${cache}" ]; then
    case "$(uname -s)" in
      Darwin) cache="${HOME}/Library/Caches/ms-playwright" ;;
      *) cache="${HOME}/.cache/ms-playwright" ;;
    esac
  fi
  if [ "${DRY_RUN}" -eq 0 ] && [ -d "${cache}" ]; then
    printf '  skip    install browser (cached in %s)\n' "${cache}"
    RESULTS+=("${CURRENT_JOB}/install browser|skipped")
  else
    run_step "install browser" bunx playwright install chromium
  fi
  run_step "build" bun run build
  run_step "end-to-end" bun run test:e2e
}

job_bench() {
  CURRENT_JOB="bench"
  CURRENT_WORKFLOW="ci.yml"
  need bun https://bun.sh
  install_step
  run_step "benchmarks" bun run packages/bench/src/cli.ts --check --tolerance=3
}

job_pages() {
  CURRENT_JOB="pages"
  CURRENT_WORKFLOW="pages.yml"
  need bun https://bun.sh
  install_step
  run_step "typecheck" bun run typecheck
  run_step "unit tests" bun run test:unit
  run_step "build for GitHub Pages" \
    env "BASE_PATH=/${BASE}/" "DOCS_BASE_URL=/${BASE}/docs/" bun run build
}

job_smithay() {
  CURRENT_JOB="smithay"
  CURRENT_WORKFLOW="smithay.yml"
  if ! have nix; then
    skip_job "nix not found; smithay.yml needs the flake's Nix shell"
    return 0
  fi
  need cargo https://rustup.rs
  run_step "build and clippy all backends" nix develop .#smithay --command bash -lc '
    set -euo pipefail
    for crate in aether-shell-wayland aether-shell-smithay aether-shell-drm; do
      cargo fmt --manifest-path os/compositor/$crate/Cargo.toml -- --check
      cargo clippy --manifest-path os/compositor/$crate/Cargo.toml -- -D warnings
      cargo build --manifest-path os/compositor/$crate/Cargo.toml
    done
  '
  run_step "test standalone backends" nix develop .#smithay --command bash -lc '
    set -euo pipefail
    cargo test --manifest-path os/compositor/aether-shell-wayland/Cargo.toml
    cargo test --manifest-path os/compositor/aether-shell-drm/Cargo.toml
  '
  run_step "curated presentation smokes" nix develop .#smithay --command bash -lc '
    set -euo pipefail
    BIN=os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay
    for smoke in \
      smoke-damage-xvfb.sh smoke-idle-xvfb.sh smoke-cursor-xvfb.sh \
      smoke-palette-xvfb.sh smoke-explorer-xvfb.sh smoke-chat-xvfb.sh \
      smoke-focus-xvfb.sh smoke-fullscreen-xvfb.sh smoke-provider-xvfb.sh \
      smoke-index-xvfb.sh smoke-index-slow-xvfb.sh smoke-text-input-xvfb.sh \
      smoke-agent-ipc-xvfb.sh smoke-clipboard-persist-xvfb.sh \
      smoke-actions-xvfb.sh smoke-audio-xvfb.sh smoke-shortcuts-xvfb.sh \
      smoke-topbar-xvfb.sh smoke-task-manager-xvfb.sh; do
      echo "=== ${smoke}"
      BIN="${BIN}" "./os/compositor/aether-shell-smithay/${smoke}"
    done
  '
}

# --- driver ----------------------------------------------------------------

STARTED="${SECONDS}"

for job in "${JOBS[@]}"; do
  JOB_FAILED=0
  case "$job" in
    verify | rust | e2e | bench | pages | smithay) ;;
    *) echo "error: unknown job: $job" >&2; exit 2 ;;
  esac
  # Set the workflow label before the header so a Nix skip reads correctly.
  CURRENT_WORKFLOW=""
  case "$job" in
    verify | rust | e2e | bench) CURRENT_WORKFLOW="ci.yml" ;;
    pages) CURRENT_WORKFLOW="pages.yml" ;;
    smithay) CURRENT_WORKFLOW="smithay.yml" ;;
  esac
  printf '\n=== %s (.github/workflows/%s)\n' "$job" "${CURRENT_WORKFLOW}"
  [ "${DRY_RUN}" -eq 1 ] && printf '  (dry run)\n'
  "job_${job}"
done

ELAPSED="$((SECONDS - STARTED))"

PASSED=0
FAILED=0
SKIPPED=0
PLANNED=0
for record in "${RESULTS[@]}"; do
  case "${record##*|}" in
    pass) PASSED=$((PASSED + 1)) ;;
    dry-run) PLANNED=$((PLANNED + 1)) ;;
    fail) FAILED=$((FAILED + 1)) ;;
    *) SKIPPED=$((SKIPPED + 1)) ;;
  esac
done

echo
echo "Aether local CI · $(date -u +%Y-%m-%dT%H:%M:%SZ) · $(uname -s) $(uname -m)"
echo "jobs: ${JOBS[*]}"
if [ "${FAILED}" -gt 0 ]; then
  echo
  echo "failures"
  for record in "${RESULTS[@]}"; do
    case "${record##*|}" in
      fail) printf '  %s\n' "${record%|*}" ;;
    esac
  done
fi
echo
if [ "${DRY_RUN}" -eq 1 ]; then
  printf 'steps: %s planned, %s skipped · %ss\n' "${PLANNED}" "${SKIPPED}" "${ELAPSED}"
  echo "RESULT: dry run (nothing executed)"
  exit 0
fi
printf 'steps: %s passed, %s failed, %s skipped · %ss\n' \
  "${PASSED}" "${FAILED}" "${SKIPPED}" "${ELAPSED}"

if [ "${FAILED}" -gt 0 ]; then
  echo "RESULT: fail (${FAILED} step(s))"
  exit 1
fi
echo "RESULT: pass"
exit 0