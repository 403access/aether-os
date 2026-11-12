---
id: hardware
title: Hardware notes — i7-7700K
sidebar_position: 3
description: "Firmware settings, expected devices and known constraints for installing Aether on the reference machine."
---

# Hardware notes — i7-7700K

The reference machine is an Intel Core i7-7700K (Kaby Lake, 4c/8t) with HD Graphics
630. This page covers what the install assumes and how to verify it.

## Firmware settings

| Setting | Value | Consequence if wrong |
| --- | --- | --- |
| Boot mode | UEFI only, CSM off | Installer/bootloader will not be found |
| Secure Boot | Disabled | systemd-boot is unsigned |
| VT-x | Enabled | KVM/microVM tier unavailable |
| VT-d / IOMMU | Enabled | VFIO passthrough blocked by the planner |
| Above 4G decoding | Enabled | IOMMU and large BARs may fail |
| Fast Boot | Disabled during install | USB may not enumerate |
| NVMe/SATA mode | AHCI/NVMe (no RST/RAID) | Linux may not see the disk |
| XMP | Optional | Instability if the memory kit is marginal |

After boot, confirm:

```bash
dmesg | grep -i -E "DMAR|IOMMU"
ls /sys/kernel/iommu_groups | wc -l    # > 0
```

## Expected devices

| Device | Driver | Check |
| --- | --- | --- |
| CPU | `intel_pstate`, `coretemp` | `lscpu`, `sensors` |
| iGPU HD 630 | `i915` | `lspci -k -s 00:02.0`, `vainfo` |
| NVMe | `nvme` | `lsblk`, `nvme list` |
| Analog audio | `snd_hda_intel` | `aplay -l` |
| USB audio | `snd_usb_audio` | `aplay -l` after plugging |
| Bluetooth | `btusb` | `bluetoothctl show` |
| Ethernet | board-specific (`e1000e`, `igb`, `r8169`) | `ip a` |

If the Ethernet driver is missing on your board, add its kernel module to
`boot.kernelModules` in `hardware/kabylake-7700k.nix` and rebuild. Wi-Fi usually
needs firmware; wired Ethernet is the supported install path.

## Graphics

The reference path is the integrated HD 630 with the iHD VA-API driver:

```bash
vainfo | grep -E "Driver version|VAProfileH264|VAProfileHEVC"
```

Expected: `iHD` driver, H.264 and HEVC profiles. This is what remote display and
hardware video decode will use in P7/P6.

If you have a discrete NVIDIA GPU:

- The kernel still boots; the console uses the iGPU.
- Add the NVIDIA driver and userspace packages in a fork; Aether does not
  configure it.
- NVENC replaces VA-API behind the same encoder interface later.

## Audio

```bash
aplay -l
wpctl status
wpctl inspect @DEFAULT_AUDIO_SINK@
```

The install enables PipeWire + WirePlumber with a 48 kHz graph and a 1024-frame
default quantum. Bluetooth is enabled but pairing is manual until P4:

```bash
bluetoothctl
> power on
> scan on
> pair <MAC>
> connect <MAC>
```

## Storage

```bash
lsblk -o NAME,SIZE,FSTYPE,LABEL,MOUNTPOINTS
findmnt -o TARGET,SOURCE,FSTYPE,OPTIONS
```

Labels created by the installer:

| Label | Mount | FS |
| --- | --- | --- |
| `AETHER-BOOT` | `/boot` | vfat |
| `aether-root` | `/` | xfs |
| `aether-nix` | `/nix` | xfs |
| `aether-scratch` | `/scratch` | ext4 |
| `aether-home` | `/home` | xfs |

XFS labels are limited to 12 characters; keep that in mind if you rename them.

## CPU and thermal

The default governor is `performance` (set in the hardware module) to keep frame
pacing and audio deterministic. Check:

```bash
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
sensors                       # package temperature
cat /proc/loadavg
```

A 7700K under sustained all-core load will clock below its 4.5 GHz turbo. The
capacity model (3 usable cores for agents) assumes sustained clocks, not turbo.

## The PKU constraint

```bash
grep -c pku /proc/cpuinfo     # 0
```

Kaby Lake has no Memory Protection Keys. The isolation planner therefore:

- escalates untrusted `in-process` requests to Landlock processes;
- blocks microVM plans without VT-x/VT-d;
- never claims WRPKRU isolation.

This is not a missing driver. It is a property of the silicon, and the security
model is built around it. See [agent isolation](/architecture/agent-isolation).

## Variants that need a fork

| Change | Required work |
| --- | --- |
| Different CPU | New hardware profile; kernel config; re-measure budgets |
| Laptop | Power management, lid/backlight, battery budget |
| Legacy BIOS only | BIOS boot partition + GRUB; untested |
| Discrete GPU (NVIDIA) | Driver + Wayland integration; not configured |
| Wi-Fi-only | Firmware + regulatory domain; Ethernet supported instead |
| Smaller disk | Adjust disko percentages |
| Disk encryption | LUKS in disko + initrd; not provided |
| More than one disk | Extend disko; the default assumes one disk |

## Failure signals

| Signal | Meaning |
| --- | --- |
| `i915` probe fails | Kernel config or firmware; console may still work |
| No `landlock` in LSM | Kernel fragment not applied; fix before trusting agents |
| IOMMU groups empty | VT-d disabled in firmware |
| `/scratch` missing | Labels differ; check `blkid` |
| Audio codec not found | Wrong `snd_hda_intel` model; add `options snd-hda-intel model=...` |
| Thermal throttling at idle | Fan/curve/thermal paste; not an OS issue |
