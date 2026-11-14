#!/usr/bin/env bash
#
# Boot the development VM image with QEMU/HVF on macOS.
#
#   ./scripts/vm-run.sh             interactive serial console
#   ./scripts/vm-run.sh --gui       persistent virtio-gpu desktop + VNC/cocoa
#   ./scripts/vm-run.sh --smoke     headless, wait for AETHER_VM_READY
#   ./scripts/vm-run.sh --drm-smoke headless virtio-gpu, wait for the native
#                                   DRM backend result
#
#   VM_GUI_DISPLAY=cocoa ./scripts/vm-run.sh --gui   # native QEMU window
#   VM_GUI_DISPLAY=vnc   ./scripts/vm-run.sh --gui   # default, 127.0.0.1:5905
#   VM_VNC_DISPLAY=6     ./scripts/vm-run.sh --gui   # 127.0.0.1:5906
#
# The VM target lives in os/vm; it is not the i7-7700K installation target.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ART="${ROOT}/artifacts/vm"
MODE="${1:-run}"
GUI=0
[ "${MODE}" = "--gui" ] && GUI=1
MEM="${VM_MEM:-4096}"
CPUS="${VM_CPUS:-4}"
SSH_PORT="${VM_SSH_PORT:-2222}"
TIMEOUT="${VM_TIMEOUT:-240}"

die() { printf '\n\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }

BASE_IMAGE="$(find "${ART}" -maxdepth 2 -name "*.qcow2" ! -name "run.qcow2" 2>/dev/null | head -1 || true)"
[ -n "${BASE_IMAGE}" ] || die "no .qcow2 in ${ART}; run scripts/vm-build.sh first"

command -v qemu-system-aarch64 >/dev/null 2>&1 || die "qemu is missing: brew install qemu"
command -v qemu-img >/dev/null 2>&1 || die "qemu-img is missing: brew install qemu"

# Boot a writable overlay so the built image stays pristine and a broken VM
# can be reset by deleting one file.
IMAGE="${ART}/run.qcow2"
if [ ! -f "${IMAGE}" ] || [ "${VM_RESET:-0}" = "1" ] || [ "${MODE}" = "--smoke" ]; then
  rm -f "${IMAGE}"
  qemu-img create -q -f qcow2 -F qcow2 -b "${BASE_IMAGE}" "${IMAGE}"
fi

FIRMWARE="${QEMU_EFI:-/opt/homebrew/share/qemu/edk2-aarch64-code.fd}"
if [ ! -f "${FIRMWARE}" ]; then
  PREFIX="$(brew --prefix qemu 2>/dev/null || true)"
  FIRMWARE="${PREFIX}/share/qemu/edk2-aarch64-code.fd"
fi
[ -f "${FIRMWARE}" ] || die "EDK2 aarch64 firmware not found (set QEMU_EFI)"

QEMU_ARGS=(
  -M virt
  -accel hvf
  -cpu host
  -smp "${CPUS}"
  -m "${MEM}"
  -bios "${FIRMWARE}"
  -drive "file=${IMAGE},if=virtio,format=qcow2"
  -device virtio-rng-pci
)

# The smoke test does not need SSH; avoiding a fixed host port also prevents
# conflicts when an interactive VM is already running.
if [ "${MODE}" = "--smoke" ]; then
  QEMU_ARGS+=(-netdev "user,id=net0" -device virtio-net-pci,netdev=net0)
else
  QEMU_ARGS+=(
    -netdev "user,id=net0,hostfwd=tcp::${SSH_PORT}-:22"
    -device virtio-net-pci,netdev=net0
  )
fi

GUI_ARGS=()
if [ "${GUI}" = "1" ]; then
  QEMU_ARGS+=(
    -device virtio-gpu-pci
    -device virtio-keyboard-pci
    -device virtio-tablet-pci
    -fw_cfg "name=opt/aether/gui,string=1"
  )
  if [ "${VM_GUI_DISPLAY:-vnc}" = "cocoa" ]; then
    GUI_ARGS+=(-display cocoa,show-cursor=on)
  else
    VNC_DISPLAY="${VM_VNC_DISPLAY:-5}"
    VNC_PORT=$((5900 + VNC_DISPLAY))
    GUI_ARGS+=(-display "vnc=127.0.0.1:${VNC_DISPLAY}")
  fi
fi

if [ "${MODE}" = "--drm-smoke" ]; then
  VNC_DISPLAY="${VM_VNC_DISPLAY:-5}"
  VNC_PORT=$((5900 + VNC_DISPLAY))
  LOG="${ART}/drm-serial.log"
  : >"${LOG}"
  say "booting headless with virtio-gpu; waiting up to ${TIMEOUT}s for AETHER_VM_DRM_STATUS"
  qemu-system-aarch64 "${QEMU_ARGS[@]}" \
    -device virtio-gpu-pci \
    -display "vnc=127.0.0.1:${VNC_DISPLAY}" \
    -monitor none -serial "file:${LOG}" &
  PID=$!
  trap 'kill "${PID}" 2>/dev/null || true; wait "${PID}" 2>/dev/null || true' EXIT

  START="$(date +%s)"
  while true; do
    if grep -q "AETHER_VM_DRM_STATUS=" "${LOG}" 2>/dev/null; then
      say "DRM result after $(( $(date +%s) - START ))s"
      grep -E "AETHER_VM_|aether-shell" "${LOG}" | tail -30
      STATUS_LINE="$(grep "AETHER_VM_DRM_STATUS=" "${LOG}" | tail -1)"
      REMOVED="$(grep -c "removed (0 remaining)" "${LOG}" || true)"
      REBOUND="$(grep -c "AETHER_VM_DRM_BIND=" "${LOG}" || true)"
      [ "${REMOVED}" -ge 1 ] || die "hotplug did not remove the DRM output"
      [ "${REBOUND}" -ge 1 ] || die "hotplug did not rebind the DRM device"
      case "${STATUS_LINE}" in
        *"=0"*) exit 0 ;;
        *) die "native DRM backend failed: ${STATUS_LINE}" ;;
      esac
    fi
    if ! kill -0 "${PID}" 2>/dev/null; then
      echo "--- serial log ---" >&2
      tail -60 "${LOG}" >&2
      die "QEMU exited before the DRM result"
    fi
    if [ $(( $(date +%s) - START )) -gt "${TIMEOUT}" ]; then
      echo "--- serial log ---" >&2
      tail -60 "${LOG}" >&2
      die "timed out waiting for the native DRM result"
    fi
    sleep 2
  done
fi

if [ "${MODE}" = "--smoke" ]; then
  LOG="${ART}/serial.log"
  : >"${LOG}"
  say "booting headless; waiting up to ${TIMEOUT}s for AETHER_VM_READY"
  qemu-system-aarch64 "${QEMU_ARGS[@]}" \
    -display none -monitor none -serial "file:${LOG}" &
  PID=$!
  trap 'kill "${PID}" 2>/dev/null || true; wait "${PID}" 2>/dev/null || true' EXIT

  START="$(date +%s)"
  while true; do
    if grep -q "AETHER_VM_READY" "${LOG}" 2>/dev/null; then
      say "boot ok after $(( $(date +%s) - START ))s"
      grep "AETHER_VM_" "${LOG}" | tail -5
      exit 0
    fi
    if ! kill -0 "${PID}" 2>/dev/null; then
      echo "--- serial log ---" >&2
      tail -40 "${LOG}" >&2
      die "QEMU exited before the boot marker"
    fi
    if [ $(( $(date +%s) - START )) -gt "${TIMEOUT}" ]; then
      echo "--- serial log ---" >&2
      tail -40 "${LOG}" >&2
      die "timed out waiting for the boot marker"
    fi
    sleep 2
  done
fi

if [ "${GUI}" = "1" ]; then
  say "booting ${IMAGE} with a persistent graphics session"
  if [ "${VM_GUI_DISPLAY:-vnc}" = "cocoa" ]; then
    say "a QEMU window will open; the compositor starts on tty8 after the DRM check"
  else
    say "VNC on 127.0.0.1:${VNC_PORT:-5905} (open vnc://127.0.0.1:${VNC_PORT:-5905})"
    say "the compositor starts on tty8 after the DRM check; SSH: ssh -p ${SSH_PORT} aether@127.0.0.1"
  fi
  if [ "${VM_DRY_RUN:-0}" = "1" ]; then
    printf 'qemu-system-aarch64'; printf ' %q' "${QEMU_ARGS[@]}" "${GUI_ARGS[@]}" -serial mon:stdio; printf '\n'
    exit 0
  fi
  exec qemu-system-aarch64 "${QEMU_ARGS[@]}" "${GUI_ARGS[@]}" -serial mon:stdio
fi

say "booting ${IMAGE} (Ctrl-A X to quit QEMU)"
say "login: aether / aether   ·   ssh -p ${SSH_PORT} aether@127.0.0.1"
if [ "${VM_DRY_RUN:-0}" = "1" ]; then
  printf 'qemu-system-aarch64'; printf ' %q' "${QEMU_ARGS[@]}" -nographic; printf '\n'
  exit 0
fi
exec qemu-system-aarch64 "${QEMU_ARGS[@]}" -nographic
