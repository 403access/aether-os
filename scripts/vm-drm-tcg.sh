#!/usr/bin/env bash
#
# Boot the aarch64 development VM under TCG (no nested KVM) with virtio-gpu and
# wait for the native DRM backend result on the serial console.
#
# The NixOS build.vm runner needs virtiofsd, so run this on Linux:
#
#   ./scripts/vm-drm-tcg.sh
#
# QEMU_OPTS is forwarded to QEMU; override VM_VNC_PORT or VM_TIMEOUT as needed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${ROOT}/artifacts/vm-drm"
TIMEOUT="${VM_TIMEOUT:-900}"
VNC_PORT="${VM_VNC_PORT:-5905}"

die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }

command -v nix >/dev/null || die "nix is required"
command -v qemu-system-aarch64 >/dev/null || say "QEMU comes from the VM runner closure"

mkdir -p "${OUT}"
say "building the VM runner"
RUNNER="$(nix build "${ROOT}#vm" --no-link --print-out-paths --option sandbox false)"
RUN="$(find "${RUNNER}/bin" -maxdepth 1 -name 'run-*-vm' | head -1)"
[ -x "${RUN}" ] || die "no VM runner in ${RUNNER}"

: >"${OUT}/serial.log"
say "booting under TCG; waiting up to ${TIMEOUT}s for AETHER_VM_DRM_STATUS"
QEMU_OPTS="-smp 2 -vnc 127.0.0.1:${VNC_PORT} -serial file:${OUT}/serial.log -device virtio-gpu-pci" \
  timeout -k 10 "${TIMEOUT}" "${RUN}" >"${OUT}/run.log" 2>&1 || true

if ! grep -q "AETHER_VM_DRM_STATUS=" "${OUT}/serial.log"; then
  tail -40 "${OUT}/serial.log" >&2 || true
  die "no native DRM result (see ${OUT}/serial.log)"
fi

tail -40 "${OUT}/serial.log" | grep -E "AETHER_VM_|aether-shell|aether-shell-drm" || true
STATUS="$(grep "AETHER_VM_DRM_STATUS=" "${OUT}/serial.log" | tail -1)"
[ "${STATUS##*=}" = "0" ] || die "native DRM backend failed: ${STATUS}"
say "native DRM backend passed on virtio-gpu"
