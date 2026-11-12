#!/usr/bin/env bash
#
# Aether OS installer — run from the live NixOS ISO, inside a clone of this repo.
#
#   sudo ./scripts/install.sh /dev/nvme0n1
#
# This WIPES the target disk. It refuses to run unless the device is a block
# device, the machine booted via UEFI, and you type the device name to confirm.
#
# What it does:
#   1. validates the target and prints its model/size
#   2. runs disko with os/nixos/disko.nix for that device
#   3. mounts the result at /mnt
#   4. installs nixosConfigurations.kestrel from this checkout
#   5. leaves the root account locked (use the operator user + sudo)
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLAKE_PATH="${REPO_ROOT}"
DEVICE="${1:-}"
FLAKE_ATTR="kestrel"
MOUNT_POINT="/mnt"

say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\n\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if [ "${EUID}" -ne 0 ]; then
  die "run as root (sudo ./scripts/install.sh /dev/nvme0n1)"
fi

if [ -z "${DEVICE}" ]; then
  die "usage: sudo ./scripts/install.sh /dev/nvme0n1"
fi

if [ ! -e "${DEVICE}" ]; then
  die "device ${DEVICE} does not exist"
fi

if [ ! -b "${DEVICE}" ]; then
  die "${DEVICE} is not a block device"
fi

if [ ! -f "${REPO_ROOT}/os/nixos/flake.nix" ]; then
  die "run this script from the Aether repository (os/nixos/flake.nix not found)"
fi

if [ ! -d /sys/firmware/efi ]; then
  die "this installer expects UEFI. Reboot in UEFI mode (disable CSM/Legacy) or adapt os/nixos/configuration.nix to GRUB."
fi

if ! command -v nix >/dev/null 2>&1; then
  die "nix is not on PATH; are you on the NixOS installer image?"
fi

if mount | grep -q "^${DEVICE}"; then
  die "${DEVICE} is mounted. Unmount it before installing."
fi

say "target device"
lsblk -o NAME,SIZE,MODEL,TYPE,MOUNTPOINTS "${DEVICE}" || true

echo
echo "Everything on ${DEVICE} will be destroyed."
echo "Partitions created: ESP 1G, root 25%, nix 30%, scratch 10%, home remainder."
echo
read -r -p "Type the device path again to continue (${DEVICE}): " CONFIRM
if [ "${CONFIRM}" != "${DEVICE}" ]; then
  die "confirmation did not match; nothing was written"
fi

say "checking the flake evaluates"
nix --extra-experimental-features "nix-command flakes" \
  flake check "${FLAKE_PATH}" --no-build >/dev/null

say "partitioning and formatting ${DEVICE}"
DISKO_CONFIG="$(mktemp /tmp/aether-disko-XXXXXX.nix)"
trap 'rm -f "${DISKO_CONFIG}"' EXIT
cat >"${DISKO_CONFIG}" <<EOF
import ${REPO_ROOT}/os/nixos/disko.nix { device = "${DEVICE}"; }
EOF

nix --extra-experimental-features "nix-command flakes" \
  run nixpkgs#disko -- --mode disko "${DISKO_CONFIG}"

if ! mountpoint -q "${MOUNT_POINT}"; then
  die "disko finished but ${MOUNT_POINT} is not mounted; inspect the output above"
fi

say "installed filesystems"
findmnt -R "${MOUNT_POINT}" -o TARGET,SOURCE,FSTYPE,OPTIONS || true

say "installing Aether OS (this builds the system closure)"
nixos-install \
  --root "${MOUNT_POINT}" \
  --flake "${FLAKE_PATH}#${FLAKE_ATTR}" \
  --no-root-passwd

say "done"
cat <<EOF
Aether OS is installed on ${DEVICE}.

Next:
  1. Reboot and remove the USB stick.
  2. Select the NixOS entry in the UEFI boot menu.
  3. Log in as "oliver" (initial password: aether) and run: passwd
  4. Read the first-boot checklist:
     https://github.com/olivermolnar/aether-os/blob/main/apps/docs/docs/getting-started/first-boot.md

Rollback: the previous generation is in the systemd-boot menu.
EOF
