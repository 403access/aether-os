#!/usr/bin/env bash
#
# Aether target preflight. Checks the Kaby Lake reference machine, UEFI, KMS,
# input devices and memory. Works against a mounted target with --root.
#
#   ./scripts/target-check.sh
#   ./scripts/target-check.sh --root /mnt
#
set -uo pipefail

ROOT="/"
while [ $# -gt 0 ]; do
  case "$1" in
    --root)
      ROOT="${2:?--root needs a path}"
      shift 2
      ;;
    -h|--help)
      echo "usage: $0 [--root PATH]"
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

ROOT="${ROOT%/}"
[ -n "${ROOT}" ] || ROOT="/"
path() {
  if [ "${ROOT}" = "/" ]; then
    printf '%s' "$1"
  else
    printf '%s%s' "${ROOT}" "$1"
  fi
}

FAILS=0
WARNS=0
pass() { printf 'PASS  %s\n' "$1"; }
warn() { printf 'WARN  %s\n' "$1"; WARNS=$((WARNS + 1)); }
fail() { printf 'FAIL  %s\n' "$1"; FAILS=$((FAILS + 1)); }

echo "Aether target check (root=${ROOT})"
echo

# UEFI
if [ -d "$(path /sys/firmware/efi)" ]; then
  pass "UEFI firmware detected"
else
  fail "no UEFI firmware at /sys/firmware/efi (legacy boot is unsupported)"
fi

# CPU
CPUINFO="$(path /proc/cpuinfo)"
if [ -r "${CPUINFO}" ]; then
  MODEL="$(grep -m1 'model name' "${CPUINFO}" | cut -d: -f2- | sed 's/^ *//')"
  if printf '%s' "${MODEL}" | grep -qi 'i7-7700K'; then
    pass "CPU: ${MODEL}"
  else
    warn "CPU is ${MODEL:-unknown}, expected Intel Core i7-7700K"
  fi
else
  warn "cannot read /proc/cpuinfo"
fi

# Memory
MEMINFO="$(path /proc/meminfo)"
if [ -r "${MEMINFO}" ]; then
  MEM_KB="$(awk '/^MemTotal:/ {print $2}' "${MEMINFO}")"
  if [ -n "${MEM_KB}" ] && [ "${MEM_KB}" -ge 8388608 ]; then
    pass "memory: $((MEM_KB / 1024 / 1024)) GiB"
  else
    warn "memory below 8 GiB (${MEM_KB:-?} kB)"
  fi
else
  warn "cannot read /proc/meminfo"
fi

# Kernel command line
CMDLINE="$(path /proc/cmdline)"
if [ -r "${CMDLINE}" ] && grep -qw 'nomodeset' "${CMDLINE}"; then
  fail "kernel command line contains nomodeset (KMS would be disabled)"
else
  pass "kernel command line does not disable KMS"
fi

# i915 driver
if [ -d "$(path /sys/module/i915)" ] || compgen -G "$(path /lib/modules/*/kernel/drivers/gpu/drm/i915)" >/dev/null; then
  pass "i915 driver available"
else
  fail "i915 driver not found"
fi

# DRM devices
shopt -s nullglob
CARDS=("$(path /dev/dri)"/card*)
RENDER_NODES=("$(path /dev/dri)"/renderD*)
if [ ${#CARDS[@]} -gt 0 ]; then
  pass "DRM cards: ${CARDS[*]##*/}"
else
  fail "no /dev/dri/card* (KMS device missing)"
fi
if [ ${#RENDER_NODES[@]} -gt 0 ]; then
  pass "render nodes: ${RENDER_NODES[*]##*/}"
else
  warn "no /dev/dri/renderD* (software fallback may be required)"
fi

# Connected connectors and modes
CONNECTED=0
MODES=0
CONNECTORS=()
for status in "$(path /sys/class/drm)"/card*-*/status; do
  [ -r "${status}" ] || continue
  if grep -qx 'connected' "${status}" 2>/dev/null; then
    CONNECTED=$((CONNECTED + 1))
    connector="$(basename "$(dirname "${status}")")"
    CONNECTORS+=("${connector}")
    modes_file="$(dirname "${status}")/modes"
    if [ -s "${modes_file}" ]; then
      MODES=$((MODES + 1))
    fi
  fi
done
if [ "${CONNECTED}" -gt 0 ]; then
  pass "connected connectors: ${CONNECTORS[*]}"
  if [ "${MODES}" -gt 0 ]; then
    pass "connected connector(s) expose KMS modes"
  else
    fail "connected connectors expose no modes"
  fi
else
  fail "no connected DRM connector (check cable/monitor)"
fi

# Input devices
INPUTS=("$(path /dev/input)"/event*)
if [ ${#INPUTS[@]} -gt 0 ]; then
  pass "input devices: ${#INPUTS[@]} evdev node(s)"
else
  fail "no /dev/input/event* (keyboard/pointer unavailable)"
fi

# Seat (only meaningful on a live system)
if [ "${ROOT}" = "/" ]; then
  if [ -e /run/systemd/seats/seat0 ] || command -v loginctl >/dev/null 2>&1; then
    pass "seat0/logind available"
  else
    warn "no seat0 detected (logind/libseat not running)"
  fi
else
  warn "seat/logind not checked on a mounted root"
fi

# Firmware tables
if [ -d "$(path /sys/firmware/acpi/tables)" ]; then
  pass "ACPI tables available"
else
  warn "no ACPI tables under /sys/firmware"
fi
shopt -u nullglob

echo
if [ "${FAILS}" -gt 0 ]; then
  echo "RESULT: ${FAILS} failure(s), ${WARNS} warning(s)"
  exit 1
fi
echo "RESULT: ready (${WARNS} warning(s))"
exit 0
