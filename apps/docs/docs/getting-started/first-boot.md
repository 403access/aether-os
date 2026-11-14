---
id: first-boot
title: First boot checklist
sidebar_position: 2
description: "Verify the installed Aether base: kernel, Landlock, filesystems, GPU, audio and services."
---

# First boot checklist

The machine boots two sessions: the console on **tty1** and the Aether compositor
on **tty7**. Switch with `Ctrl+Alt+F1` and `Ctrl+Alt+F7`. If the native DRM
backend fails, `aether-session` falls back to cage + the Smithay adapter.

This page verifies the base before you trust it with work.

## Sign in and secure the account

| Field | Value |
| --- | --- |
| User | `oliver` |
| Initial password | `aether` |

```bash
passwd
```

The root account is locked; use `sudo`. SSH is disabled and there is no inbound
network service.

## 5-minute verification

```bash
uname -a
cat /sys/kernel/security/lsm
findmnt -o TARGET,SOURCE,FSTYPE,OPTIONS
free -m
systemd-analyze
vainfo
aplay -l
wpctl status
```

Expected:

| Check | Expected |
| --- | --- |
| `uname -r` | ends in `-aether` |
| LSM line | contains `landlock`, `yama`, `bpf` |
| `/` | xfs, `noatime,largeio,inode64` |
| `/nix` | xfs |
| `/home` | xfs |
| `/scratch` | ext4, `commit=60` |
| `/var/index` | tmpfs, 2 GiB |
| `free -m` | record it; this replaces the idle-RAM budget |
| `systemd-analyze` | record userspace boot time; target < 1.5 s |
| `vainfo` | VA-API opens the iHD driver (HD 630) |
| `aplay -l` | at least one analog HDA device |
| `wpctl status` | PipeWire graph with a default sink |

If a check fails, see [hardware](/getting-started/hardware) and the
[troubleshooting table](/getting-started/install#troubleshooting).

## Confirm the hardware assumptions

```bash
lscpu | grep -E "Model name|Core|Thread|MHz"
grep -c pku /proc/cpuinfo          # 0 on Kaby Lake
dmesg | grep -i -E "iommu|DMAR"    # VT-d enabled
ls /sys/kernel/iommu_groups        # non-empty
lsblk -o NAME,SIZE,FSTYPE,LABEL,MOUNTPOINTS
```

Kaby Lake has **no PKU**. That is expected; the isolation planner blocks PKU-based
plans and uses Landlock processes instead.

## Use the compositor

Switch to tty7 (`Ctrl+Alt+F7`). The session starts the native DRM backend and
draws the workspace pills, status bar and shell surfaces directly on the output.

Try the fast paths:

```text
Super+Space     command palette
Super+Return    terminal
Super+E         explorer
Super+C         chat drawer
Super+A         audio routes
Super+/         shortcut cheat sheet
```

The full key list and workflows are in [Using Aether](/getting-started/using-aether).
`aether-scene` remains available as a headless layout/damage sanity check; the
`aether-shell-preview` binary renders a frame plan to PNG.

## What is running

| Service | State |
| --- | --- |
| systemd (multi-user) | enabled |
| NetworkManager | enabled |
| PipeWire + WirePlumber | enabled |
| dbus | enabled |
| systemd-journald | enabled, 500 MB cap |
| aether-session (native DRM, cage fallback) | enabled on tty7 |
| aether-index | enabled as a user service |
| aetherd (agent supervisor) | not installed |

Check:

```bash
systemctl --failed
systemctl status pipewire wireplumber NetworkManager --no-pager
journalctl -b -p warning --no-pager | tail -30
```

`systemctl --failed` should be empty.

## Audio smoke test

```bash
speaker-test -c 2 -t sine -l 1     # analog output
wpctl status                        # routes and default sink
```

Switch outputs interactively with `Super+A`, or with
`wpctl set-default <id>`.

## Rebuild and rollback

The installed system keeps the flake at `/etc/nixos`. To build from a working
checkout:

```bash
cd ~
git clone https://github.com/olivermolnar/aether-os.git
cd aether-os
sudo nixos-rebuild switch --flake .#kestrel
```

- `switch` activates immediately.
- `boot` builds for the next boot.
- `--rollback` returns to the previous generation.

List generations:

```bash
sudo nix-env --list-generations --profile /nix/var/nix/profiles/system
```

## Useful local configuration

| Change | How |
| --- | --- |
| Hostname | `networking.hostName` in `configuration.nix` |
| Timezone | `time.timeZone` |
| Keyboard layout | `console.keyMap`, or `loadkeys` for the session |
| SSH (temporary) | `services.openssh.enable = true;` then rebuild |
| Extra packages | `environment.systemPackages` |
| Disk scratch | `filesystem.nix` and `disko.nix` must agree |

Do not edit `/etc/nixos` and expect it to survive a repo-based rebuild. Pick one
source of truth: this repository.

## Known gaps at this milestone

- The compositor and index daemon are packaged; the agent supervisor, browser and
  media apps are not.
- Live peak input meters and TCP/Bluetooth matrix testing remain P5/target work.
- No screen-reader stack.
- No LUKS layout.
- No NVIDIA/discrete GPU configuration.
- Power management is fixed to the `performance` governor; there is no battery
  profile because the reference machine is a desktop.

## Record the numbers

Write the output of these into `docs/measurements/` (create it) or a note:

```bash
date -Is
uname -a
free -m
systemd-analyze
```

The roadmap is explicit: budgets are replaced by measurements at P1. This is the
first place that happens.
