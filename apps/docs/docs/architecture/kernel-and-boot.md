---
id: kernel-and-boot
title: Kernel and boot
sidebar_position: 3
description: Kernel configuration strategy, boot chain, command line and security parameters.
---

# Kernel and boot

The kernel is compiled for this CPU and this machine. No distribution kernel, no
initramfs full of drivers for hardware that does not exist.

## Strategy

1. **Probe the machine.** Boot a live NixOS image, run `lspci -nnk`, `lsusb`, and
   `lsmod` to enumerate the exact hardware.
2. **`make localmodconfig`** to keep only loaded modules, then re-enable what is
   needed for boot (storage, input, filesystem) that the probe did not load.
3. **Disable** unused filesystems, network protocols, debug facilities, legacy
   buses, hypervisor guest drivers (unless running microVMs) and sound subsystems
   other than the ones PipeWire uses.
4. **Enable** Landlock, seccomp, cgroups v2, user namespaces, `memfd_create`,
   `io_uring` (bounded), KVM (for microVMs), VFIO, and the Kaby Lake graphics stack.
5. **Build** with the pinned toolchain; the config is committed as `kestrel.config`.

The configuration lives in `os/nixos/kernel/kestrel.config` and is wired through
`os/nixos/kernel.nix`.

## Boot chain

```text
UEFI → systemd-boot → kernel (bzImage) → initrd (minimal) →
switch_root → systemd (multi-user) → aether-session.target →
aether-shell (on seat0)
```

| Stage | Contents | Budget |
| --- | --- | --- |
| Firmware | UEFI POST | not controlled |
| Bootloader | systemd-boot, 1 entry + generations | < 100 ms |
| Kernel | decompress, initrd | ~300–600 ms |
| initrd | udev, NVMe, XFS, resume | ~200 ms |
| systemd | ~40 units, no network-online wait | ~400 ms |
| Session | shell + index + audio | ~300 ms |
| **Total userspace** | kernel handoff → first frame | **< 1.5 s** |

No Plymouth splash, no display manager, no “waiting for network”. If a service is
not needed to paint the first frame or accept input, it starts after the shell.

## Command line

```text
quiet loglevel=3 mitigations=auto,nosmt=off
nowatchdog nmi_watchdog=0
zswap.enabled=0
split_lock_detect=off
intel_pstate=active
```

| Parameter | Reason |
| --- | --- |
| `mitigations=auto,nosmt=off` | Keep SMT; accept the documented trade-off. MicroVM tier is the answer for hostile code. |
| `nowatchdog` | No lockup detector in the hot path |
| `zswap.enabled=0` | No compressed swap; memory budgets are enforced, not papered over |
| `intel_pstate=active` | Predictable frequency scaling for audio and frame pacing |
| `split_lock_detect=off` | Avoid SIGBUS surprises in emulation paths; documented risk |

The command line is versioned with the kernel config. Changing it requires a
measurement note in the PR.

## Required kernel features

```text
CONFIG_SECCOMP=y
CONFIG_SECCOMP_FILTER=y
CONFIG_SECURITY_LANDLOCK=y
CONFIG_SECURITY_YAMA=y
CONFIG_CGROUPS=y
CONFIG_CGROUP_BPF=y
CONFIG_MEMFD_CREATE=y
CONFIG_USER_NS=y
CONFIG_KVM=y
CONFIG_KVM_INTEL=y
CONFIG_VFIO=y
CONFIG_VFIO_PCI=y
CONFIG_IOMMU_SUPPORT=y
CONFIG_DRM_I915=y
CONFIG_DRM_I915_USERPTR=y
CONFIG_SND_HDA_INTEL=y
CONFIG_BLK_DEV_NVME=y
CONFIG_XFS_FS=y
CONFIG_EXT4_FS=y
CONFIG_TMPFS=y
CONFIG_INOTIFY_USER=y
CONFIG_FANOTIFY=y
```

Everything else is a deliberate deletion. A smaller kernel means fewer drivers to
attack, fewer modules to load and a shorter initramfs.

## initrd

The initrd contains only what is needed to mount the root:

- NVMe driver
- XFS
- dm-crypt only if the root is encrypted (the reference machine is)
- udev
- resume support if hibernation is enabled (it is not, by default)

No network modules. No rescue shell by default; a debug generation is kept.

## systemd units

The session target starts a minimal set:

```text
aether-session.target
├── aetherd.service          broker / scheduler
├── aether-index.service     sandboxed indexer
├── aether-shell.service     compositor (WantedBy=graphical.target)
├── pipewire.service         user
├── pipewire-pulse.service   user
├── wireplumber.service      user
└── aether-audio.service     route policy
```

Explicitly disabled: `NetworkManager-wait-online`, `bluetooth` until requested,
`ModemManager`, `cups`, `avahi`, `tracker`, `packagekit`, `fwupd` (manual only).

## Security parameters

| Control | Setting |
| --- | --- |
| Kernel lockdown | `integrity` when secure boot is available |
| `kptr_restrict` | `2` |
| `dmesg_restrict` | `1` |
| `unprivileged_bpf_disabled` | `1` |
| `perf_event_paranoid` | `2` (3 during normal use, lowered explicitly for profiling) |
| `vm.unprivileged_userfaultfd` | `0` |
| `kernel.yama.ptrace_scope` | `1` |
| Core dumps | disabled for agents; local-only for shell |

These are sysctl settings in `os/nixos/configuration.nix`, not a hardening
afterthought.

## Failure modes

| Failure | Detection | Recovery |
| --- | --- | --- |
| Kernel panics on boot | No first frame within 5 s | systemd-boot previous generation |
| Missing driver | Device absent in `lspci` after boot | Boot live image, update config |
| initrd too large | Boot measurement regression | Remove modules, rebuild |
| Landlock unavailable | Planner blocks process tier | Fix kernel config; do not weaken policy |
| GPU hang | i915 reset, shell restarts | Expose a recovery surface; log to local audit |

## What is not optimised yet

- No PGO for the kernel (GCC PGO on the kernel is P8 work).
- No real-time scheduling for the compositor; `SCHED_OTHER` with nice levels is
  the baseline until audio measurements justify more.
- No hibernation. Suspend-to-RAM is supported on the reference machine.

## Verification

P1 exits when:

1. `uname -a` shows the Aether kernel and the expected config hash.
2. `/sys/kernel/security/lsm` includes `landlock`.
3. `systemd-analyze` reports userspace < 1.5 s cold, three consecutive runs.
4. Idle RAM and CPU are recorded in `docs/measurements/` and replace the budgets.
