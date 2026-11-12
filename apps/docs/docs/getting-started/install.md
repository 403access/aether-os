---
id: install
title: Install on the i7-7700K
sidebar_position: 1
description: "Step-by-step installation of Aether OS on the reference machine, from USB to first boot."
---

# Install on the i7-7700K

This guide installs Aether on one disk: the reference machine (Intel i7-7700K,
UEFI, NVMe). It wipes the target disk. Read the whole page before starting.

:::danger
The installer destroys every partition on the disk you give it. Disconnect or
back up any drive you care about. Double-check `lsblk -o NAME,SIZE,MODEL,SERIAL`
before confirming.
:::

## What you get

After this guide the machine boots **Aether OS on bare metal**: the custom kernel,
XFS/ext4/tmpfs layout, PipeWire, the operator account and a console session.

What it does **not** include yet: the `aether-shell` compositor and the daemons.
Those are P2/P3 deliverables. First boot lands in a TTY (see
[first boot](/getting-started/first-boot)). This is deliberate: a verified base is
the prerequisite for the UI, not a substitute for it.

## Requirements

| Item | Requirement |
| --- | --- |
| CPU | Intel Core i7-7700K (Kaby Lake). Other CPUs need a fork. |
| Firmware | UEFI. Legacy BIOS is not supported by the default config. |
| Disk | NVMe, ≥ 512 GiB recommended (the layout is percentage-based). |
| RAM | 16 GiB minimum; 32 GiB recommended for agents. |
| USB | 8 GiB or larger, will be erased. |
| Network | Wired Ethernet recommended. The live environment needs it to fetch packages. |
| GPU | HD 630 integrated graphics is the reference. A discrete GPU is not configured. |

:::tip Want to try it first?
You can boot a separate aarch64 development VM on an Apple Silicon Mac without
touching any hardware: see [boot the development VM](/getting-started/virtual-machine).
It is a test harness, not this installation target.
:::

## 1. Get an installer

You have two options. **Option A is recommended for the first install** because it
uses a known-good, widely tested live environment.

### Option A — official NixOS minimal ISO

Download the current **NixOS minimal installation ISO (x86_64)** from
[nixos.org/download](https://nixos.org/download/). Verify the published SHA-256:

```bash
sha256sum nixos-minimal-*.iso
```

The Aether flake pins nixpkgs; the live ISO version only needs to be recent enough
to understand flakes (any 24.11+ image works).

### Option B — build the Aether installer ISO

This builds a minimal live image with `git`, `disko`, `helix` and hardware probes
preinstalled. It requires an **x86_64 Linux machine with Nix** (or a remote
builder); macOS cannot build a Linux system closure without one.

```bash
nix build .#iso
ls result/iso/
```

The ISO is at `result/iso/*.iso`. A cold build takes a while (kernel and initrd
included).

Both options install the same system; the Aether ISO just saves the `git clone`
and package install on the live side.

## 2. Write the USB

### macOS

```bash
diskutil list
diskutil unmountDisk /dev/diskN
sudo dd if=nixos-minimal-*.iso of=/dev/rdiskN bs=4m status=progress
sync
```

Use the raw device (`rdiskN`), not `diskN`. Example: `/dev/rdisk4`.

### Linux

```bash
lsblk
sudo dd if=nixos-minimal-*.iso of=/dev/sdX bs=4M status=progress oflag=sync
sync
```

## 3. Configure the firmware

On a 2017 Kaby Lake board (ASUS/Gigabyte/MSI/ASRock), enter the firmware setup
(`Del` or `F2`) and set:

| Setting | Value | Why |
| --- | --- | --- |
| Boot mode | **UEFI only**, CSM disabled | The default config uses systemd-boot |
| Secure Boot | **Disabled** | NixOS bootloader is unsigned |
| Intel VT-x | Enabled | MicroVM tier and KVM |
| Intel VT-d / IOMMU | Enabled | VFIO passthrough |
| Above 4G decoding | Enabled | Large BARs, IOMMU |
| XMP | Your choice | Not required; stability first |
| Fast Boot | Disabled for the install | USB detection |
| SATA/NVMe mode | AHCI/NVMe | No RAID/RST |

Save, insert the USB, and open the one-time boot menu (`F8`/`F11`/`F12`, depending
on the board). Choose the **UEFI** entry for the USB, not the legacy one.

:::note Legacy BIOS
If the machine has no UEFI option, this configuration will not boot as-is. You
need a 1 MiB BIOS boot partition (`EF02`) and GRUB:

```nix
boot.loader.systemd-boot.enable = false;
boot.loader.grub = {
  enable = true;
  device = "/dev/nvme0n1";
};
```

and a matching partition in `os/nixos/disko.nix`. This is untested on the
reference machine and outside the supported path.
:::

## 4. Boot the live environment

The ISO boots to a root console (or auto-logs in as `nixos`). Check the basics:

```bash
ip a
cat /sys/firmware/efi/firmware_vendor   # must succeed if UEFI
```

Connect to the network:

```bash
nmtui          # graphical TUI: activate a connection
# or
nmcli device status
nmcli connection up "Wired connection 1"
ping -c 3 nixos.org
```

Set the keyboard layout if needed:

```bash
loadkeys de
```

## 5. Get this repository

```bash
git clone https://github.com/olivermolnar/aether-os.git
cd aether-os
```

If the repository is not published yet, copy the checkout to the USB and mount it:

```bash
mkdir -p /mnt/usb
mount /dev/sdX1 /mnt/usb
cp -r /mnt/usb/aether-os ~/aether-os
cd ~/aether-os
```

:::tip
A NixOS live environment usually has `git` available. If not:
`nix-shell -p git` or use the Aether installer ISO.
:::

## 6. Choose the disk

```bash
lsblk -o NAME,SIZE,MODEL,SERIAL,TYPE,MOUNTPOINTS
```

The target is the **NVMe drive**, typically `/dev/nvme0n1`. Do not pick the USB
stick (often `/dev/sda`, ~8–32 GB).

Edit the one line in `os/nixos/disk.nix`:

```nix
{
  device = "/dev/nvme0n1";
}
```

The default layout is ESP 1 GiB, root 80 GiB, nix 120 GiB, scratch 60 GiB and
home on the rest. If your disk is smaller than ~512 GiB, reduce the fixed sizes
in `os/nixos/disko.nix`; only the last partition may use `size = "100%"`.

## 7. Install

### Recommended: the install script

```bash
sudo ./scripts/install.sh /dev/nvme0n1
```

The script:

1. refuses to run unless you are root, on UEFI, and the target is a block device;
2. prints the model and size and asks you to type the device path again;
3. checks that the flake evaluates;
4. runs **disko** (partition, format, mount);
5. runs `nixos-install` for `.#kestrel`;
6. leaves root locked (use `oliver` + `sudo`).

### Manual equivalent

```bash
# Enable flakes for this shell (the Aether ISO does this already)
export NIX_CONFIG="experimental-features = nix-command flakes"

# 1. partition, format, mount at /mnt
sudo -E nix run nixpkgs#disko -- --mode disko --flake .#kestrel

# 2. install the system closure
sudo -E nixos-install --root /mnt --flake .#kestrel --no-root-passwd
```

`nixos-install` copies the flake to `/mnt/etc/nixos` and installs the bootloader
(systemd-boot) into the EFI partition.

## 8. First boot

```bash
sudo reboot
```

Remove the USB as the machine restarts. In the UEFI boot menu select the NixOS /
Aether entry.

Log in on tty1 as:

| Field | Value |
| --- | --- |
| User | `oliver` |
| Password | `aether` |

**Change the password immediately:**

```bash
passwd
```

Then follow [first boot](/getting-started/first-boot) to verify the kernel,
Landlock, filesystems, audio and GPU.

## Updating

From the installed system, clone the repo (or use `/etc/nixos`):

```bash
cd ~
git clone https://github.com/olivermolnar/aether-os.git
cd aether-os
sudo nixos-rebuild switch --flake .#kestrel
```

Every switch creates a generation. systemd-boot lists the last five.

## Rollback and recovery

### Roll back a bad rebuild

```bash
sudo nixos-rebuild switch --rollback
```

Or reboot and choose the previous generation in systemd-boot.

### Repair a system that does not boot

Boot the live ISO again, mount the installed system, and enter it:

```bash
sudo mount /dev/disk/by-label/aether-root /mnt
sudo mount /dev/disk/by-label/aether-nix /mnt/nix
sudo mount /dev/disk/by-label/AETHER-BOOT /mnt/boot
sudo mount --bind /dev /mnt/dev
sudo mount --bind /proc /mnt/proc
sudo mount --bind /sys /mnt/sys
sudo mount --bind /run /mnt/run
sudo nixos-enter --root /mnt
# inside the chroot:
nixos-rebuild switch --flake /etc/nixos#kestrel
```

The data partitions are untouched by `nixos-rebuild`.

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| USB not in boot menu | Legacy/CSM mode, Fast Boot, or wrong port | UEFI-only, disable Fast Boot, use a rear USB 2/3 port |
| “no bootable device” after install | Secure Boot on, or booted the USB in legacy mode | Disable Secure Boot; reinstall in UEFI mode |
| Boot stops in emergency shell | Filesystem labels not found | `blkid`; verify `aether-root`, `aether-nix`, `aether-home` |
| `disko` fails on a partition | Disk smaller than the fixed sizes | Lower percentages in `disko.nix` |
| Installer has no network | Missing firmware for the NIC/Wi-Fi | Use wired Ethernet; add firmware to `hardware/kabylake-7700k.nix` |
| Kernel panic at boot | Custom kernel config conflict | Boot the previous generation; check `journalctl -k -b -1` |
| Audio devices missing | Codec not detected | `aplay -l`, `wpctl status`; see [hardware](/getting-started/hardware) |
| `nixos-install` cannot fetch | No network or DNS | `ping nixos.org`; configure DNS in `nmtui` |

If the install fails before formatting, nothing is lost. If it fails after
formatting, the disk is empty but reusable: rerun disko.

## What this install does not do

- No compositor/UI (P2), no `aetherd`/`aether-index` services (P3).
- No disk encryption by default. LUKS is possible but not wired into
  `disko.nix`; treat it as a fork until P1 adds a tested encrypted layout.
- No dual-boot support. Aether assumes the whole disk.
- No NVIDIA/discrete GPU configuration. Integrated HD 630 is the reference.
