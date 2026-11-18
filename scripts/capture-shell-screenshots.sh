#!/usr/bin/env bash
#
# Regenerate the compositor screenshots.
#
# The screenshots come from the Smithay winit adapter running under Xvfb. The
# compositor is Linux-only, so on macOS the smokes run in a disposable Linux
# container. On Linux (or inside `nix develop .#smithay`) the same script runs
# them natively.
#
#   ./scripts/capture-shell-screenshots.sh
#   ./scripts/capture-shell-screenshots.sh --native
#   ./scripts/capture-shell-screenshots.sh --docker --rebuild
#   ./scripts/capture-shell-screenshots.sh --only palette,explorer,chat
#   ./scripts/capture-shell-screenshots.sh --list
#
# Environment:
#   OUT=artifacts/screenshots/shell   capture directory
#   IMAGE=aether-shell-capture:local  Docker image tag
#   SETTLE=6                          seconds to wait for the compositor
#   AETHER_TEST_CLIENT=...            terminal client (default: foot fixture)
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SMOKE_DIR="${ROOT}/os/compositor/aether-shell-smithay"
OUT="${OUT:-${ROOT}/artifacts/screenshots/shell}"
IMAGE="${IMAGE:-aether-shell-capture:local}"
SETTLE="${SETTLE:-6}"
MODE="${MODE:-auto}"
REBUILD=1
ONLY=""

TERMINAL="${SMOKE_DIR}/aether-terminal.sh"

# name, display number, command. Extra smokes are available on a Linux host
# with Bun; the Docker image intentionally keeps only the visual surface set.
SMOKES=(
  "topbar:70:smoke-topbar-xvfb.sh"
  "palette:71:smoke-palette-xvfb.sh"
  "explorer:72:smoke-explorer-xvfb.sh"
  "chat:73:smoke-chat-xvfb.sh"
  "audio:74:smoke-audio-xvfb.sh"
  "themes:75:smoke-themes-xvfb.sh"
  "task-manager:76:smoke-task-manager-xvfb.sh"
  "remote:77:smoke-remote-xvfb.sh"
  "shortcuts:78:smoke-shortcuts-xvfb.sh"
  "window:79:smoke-window-xvfb.sh"
  "fullscreen:80:smoke-fullscreen-xvfb.sh"
  "picker:81:smoke-picker-xvfb.sh"
  "cursor:82:smoke-cursor-xvfb.sh"
  "client:83:smoke-client-xvfb.sh"
  "clipboard:84:smoke-clipboard-xvfb.sh"
  "provider:86:smoke-provider-xvfb.sh"
  "desktop:87:smoke-desktop-xvfb.sh"
  "files:88:smoke-files-xvfb.sh"
  "refresh:89:smoke-refresh-xvfb.sh"
  "actions:90:smoke-actions-xvfb.sh"
  "agents:91:smoke-agents-xvfb.sh"
  "partial-damage:92:smoke-damage-xvfb.sh"
  "system:95:smoke-system-apps-xvfb.sh"
)

# Linux-only visual smokes that need tools the Docker image does not package.
NATIVE_SMOKES=(
  "clipboard-persist:85:smoke-clipboard-persist-xvfb.sh"
)

# These need Bun and the JavaScript indexer. Native runs use the Nix shell;
# the Docker image installs Bun and links the indexer packages.
EXTRA_SMOKES=(
  "index:97:smoke-index-xvfb.sh"
  "index-live:98:smoke-index-live-xvfb.sh"
  "index-slow:99:smoke-index-slow-xvfb.sh"
  "agent-ipc:100:smoke-agent-ipc-xvfb.sh"
)

usage() {
  cat <<'EOF'
Regenerate the Aether compositor screenshots under Xvfb.

  ./scripts/capture-shell-screenshots.sh [options]

Options:
  --native       Run in this Linux environment (requires Xvfb, xdotool, ImageMagick)
  --docker       Run in the disposable Linux capture image (default on macOS)
  --rebuild      Rebuild the Docker image (default)
  --no-rebuild   Reuse the current Docker image
  --out DIR      Capture directory (default: artifacts/screenshots/shell)
  --only NAMES   Comma-separated names from --list; e.g. palette,explorer,chat
  --list         Print the surface names and exit
  --help         Show this text

Native runs also include the index/agent-IPC smokes when Bun is installed.
Docker runs need no host dependencies beyond Docker.
EOF
}

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --native) MODE=native ;;
    --docker) MODE=docker ;;
    --rebuild) REBUILD=1 ;;
    --no-rebuild) REBUILD=0 ;;
    --out)
      OUT="${2:?--out needs a value}"
      shift
      ;;
    --only)
      ONLY="${2:?--only needs a value}"
      shift
      ;;
    --list)
      for entry in "${SMOKES[@]}" "${NATIVE_SMOKES[@]}" "${EXTRA_SMOKES[@]}"; do
        printf '%s\n' "${entry%%:*}"
      done
      exit 0
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done

mkdir -p "${OUT}"
OUT="$(cd "${OUT}" && pwd)"

# A full run is a snapshot. Remove stale images and logs so a failed smoke
# cannot leave an old file behind. Partial --only runs keep the other surfaces.
if [ -z "${ONLY}" ]; then
  find "${OUT}" -maxdepth 1 -type f \
    \( -name '*.png' -o -name '*.log' -o -name '*.json' -o -name '*.txt' \
       -o -name '*.ndjson' -o -name '*.sh' -o -name '*.marker' \) -delete
fi

if [ "${MODE}" = auto ]; then
  if command -v Xvfb >/dev/null 2>&1 && command -v xdotool >/dev/null 2>&1; then
    MODE=native
  elif command -v docker >/dev/null 2>&1; then
    MODE=docker
  else
    die "neither Xvfb/xdotool nor Docker is available"
  fi
fi

selected() {
  local name="$1"
  [ -z "${ONLY}" ] && return 0
  case ",${ONLY}," in
    *",${name},"*) return 0 ;;
    *) return 1 ;;
  esac
}

run_native() {
  local bin="${BIN:-${SMOKE_DIR}/target/debug/aether-shell-smithay}"
  [ -x "${bin}" ] || die "missing ${bin}; run cargo build --manifest-path ${SMOKE_DIR}/Cargo.toml"
  for tool in Xvfb xdotool import; do
    command -v "${tool}" >/dev/null || die "${tool} is missing; use nix develop .#smithay"
  done

  local list=("${SMOKES[@]}" "${NATIVE_SMOKES[@]}")
  if command -v bun >/dev/null 2>&1; then
    list+=("${EXTRA_SMOKES[@]}")
  else
    printf 'note: Bun is missing; skipping index and agent-IPC captures\n'
  fi

  local entry name display script start end
  local failed=0
  for entry in "${list[@]}"; do
    IFS=: read -r name display script <<<"${entry}"
    selected "${name}" || continue
    say "${name} (${script})"
    start="$(date +%s)"
    if OUT="${OUT}" BIN="${bin}" DISPLAY_NUM="${display}" SETTLE="${SETTLE}" \
      AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-${TERMINAL}}" \
      PS1="${AETHER_CAPTURE_PS1:-aether@kestrel ~ % }" \
      bash "${SMOKE_DIR}/${script}"; then
      end="$(date +%s)"
      printf 'ok: %s in %ss\n' "${name}" "$((end - start))"
    else
      end="$(date +%s)"
      printf 'warning: %s failed after %ss (continuing)\n' "${name}" "$((end - start))" >&2
      failed=$((failed + 1))
    fi
  done

  [ "${failed}" -eq 0 ] || printf 'warning: %d capture(s) failed\n' "${failed}" >&2
}

run_docker() {
  command -v docker >/dev/null 2>&1 || die "Docker is required for --docker"
  if [ "${REBUILD}" = 1 ] || ! docker image inspect "${IMAGE}" >/dev/null 2>&1; then
    say "building ${IMAGE}"
    docker build -f "${ROOT}/scripts/screenshots/Dockerfile" -t "${IMAGE}" \
      "${ROOT}"
  fi

  local terminal_in_container="/src/os/compositor/aether-shell-smithay/aether-terminal.sh"
  local client="${AETHER_TEST_CLIENT:-${terminal_in_container}}"
  local containers=()
  cleanup_containers() {
    local container
    for container in ${containers[@]+"${containers[@]}"}; do
      docker rm -f "${container}" >/dev/null 2>&1 || true
    done
  }
  trap cleanup_containers EXIT

  local failed=0
  local entry name display script container
  for entry in "${SMOKES[@]}" "${EXTRA_SMOKES[@]}"; do
    IFS=: read -r name display script <<<"${entry}"
    selected "${name}" || continue

    container="aether-shot-${name}-$$"
    docker rm -f "${container}" >/dev/null 2>&1 || true
    containers+=("${container}")
    say "${name} (${script})"

    # A fresh container per smoke keeps the process list clean for the task
    # manager capture and prevents one failed smoke from leaking Xvfb clients
    # into the next surface.
    if ! docker run -d --name "${container}" \
      -e OUT=/home/aether/work \
      -e SETTLE="${SETTLE}" \
      -e DISPLAY_NUM="${display}" \
      -e AETHER_TEST_CLIENT="${client}" \
      -e AETHER_TERMINAL="/src/os/compositor/aether-shell-smithay/aether-terminal.sh" \
      -v "${OUT}:/home/aether/work" \
      "${IMAGE}" sleep infinity >/dev/null; then
      printf 'warning: %s container failed to start\n' "${name}" >&2
      failed=$((failed + 1))
      continue
    fi

    if docker exec \
      -e OUT=/home/aether/work \
      -e SETTLE="${SETTLE}" \
      -e DISPLAY_NUM="${display}" \
      -e AETHER_TEST_CLIENT="${client}" \
      -e AETHER_TERMINAL="/src/os/compositor/aether-shell-smithay/aether-terminal.sh" \
      "${container}" \
      bash "/src/os/compositor/aether-shell-smithay/${script}"; then
      printf 'ok: %s\n' "${name}"
    else
      printf 'warning: %s failed (continuing)\n' "${name}" >&2
      failed=$((failed + 1))
    fi

    docker rm -f "${container}" >/dev/null 2>&1 || true
  done

  trap - EXIT
  cleanup_containers
  [ "${failed}" -eq 0 ] || printf 'warning: %d capture(s) failed\n' "${failed}" >&2
}

case "${MODE}" in
  native) run_native ;;
  docker) run_docker ;;
  *) die "unknown mode: ${MODE}" ;;
esac

shopt -s nullglob
pngs=("${OUT}"/*.png)
shopt -u nullglob

if [ "${#pngs[@]}" -eq 0 ]; then
  die "no PNGs were written to ${OUT}"
fi

printf '\n==> %d PNG capture(s) in %s\n' "${#pngs[@]}" "${OUT}"
if command -v identify >/dev/null 2>&1; then
  for png in "${pngs[@]}"; do
    identify -format '%f %wx%h %b\n' "${png}" || true
  done
else
  printf '%s\n' "${pngs[@]##*/}"
fi
printf '\nnext: ./scripts/publish-screenshots.sh --shell-only\n'