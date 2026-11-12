#!/usr/bin/env bash
#
# Build the aarch64 development VM image using a Linux builder.
#
# Default: OrbStack/Docker. Override with:
#   BUILDER=docker ./scripts/vm-build.sh
#   SKIP_DOCKER=1 ./scripts/vm-build.sh   # run nix build on a Linux host
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ART="${ROOT}/artifacts/vm"
STORE_VOLUME="${STORE_VOLUME:-aether-vm-nix}"

say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }

# Previous images are read-only (Nix store copy); make sure a rebuild can
# replace them.
mkdir -p "${ART}"
chmod -R u+w "${ART}" 2>/dev/null || true
rm -rf "${ART}"
mkdir -p "${ART}"

if [ "${SKIP_DOCKER:-0}" = "1" ]; then
  say "building with the local Nix (must be aarch64-linux)"
  out="$(nix build "${ROOT}#vm-image" --no-link --print-out-paths)"
  cp -Lr "${out}/." "${ART}/"
  echo "${out}" >"${ART}/store-path.txt"
  say "artifacts in ${ART}"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker/OrbStack is required (or set SKIP_DOCKER=1 on Linux)" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "error: the Docker daemon is not running. Start OrbStack and retry." >&2
  exit 1
fi

say "building aarch64 VM image in a Linux Nix container (first run downloads nixpkgs)"
docker run --rm --privileged \
  --platform linux/arm64 \
  -v "${ROOT}:/src" \
  -v "${STORE_VOLUME}:/nix" \
  -w /src \
  -e NIX_CONFIG="experimental-features = nix-command flakes" \
  -e GIT_CONFIG_COUNT=1 \
  -e GIT_CONFIG_KEY_0=safe.directory \
  -e GIT_CONFIG_VALUE_0=/src \
  nixos/nix:latest \
  sh -ec '
    set -e
    nix build .#vm-image --no-link --print-out-paths \
      --option sandbox false \
      --option system-features "kvm nixos-test benchmark big-parallel" > /tmp/out
    out="$(cat /tmp/out)"
    chmod -R u+w /src/artifacts/vm 2>/dev/null || true
    rm -rf /src/artifacts/vm
    mkdir -p /src/artifacts/vm
    if [ -d "$out" ]; then
      cp -Lr "$out"/. /src/artifacts/vm/
    else
      cp -L "$out" /src/artifacts/vm/
    fi
    printf "%s\n" "$out" > /src/artifacts/vm/store-path.txt
    echo "built: $out"
  '

say "artifacts in ${ART}"
ls -lh "${ART}"
