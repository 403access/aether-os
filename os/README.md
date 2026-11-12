# Aether OS — NixOS layer

Status: **written, not yet booted**. This is the P1 configuration. The kernel
fragment and modules are intentionally explicit; nothing here needs a desktop
environment.

## Layout

```text
flake.nix                 pinned inputs, ISO, aether-scene package, dev shell
configuration.nix         system-wide settings, boot, network, packages
hardware/kabylake-7700k.nix  CPU/GPU/audio/NVMe facts
kernel.nix                custom kernel + cmdline + sysctl
kernel/kestrel.config     config fragment (appended to defconfig)
filesystem.nix            XFS root, ext4 scratch, tmpfs index
services.nix              PipeWire, WirePlumber, udev, journald
users.nix                 operator, service user, agent UID range
```

## Build

Requires Linux and Nix with flakes enabled:

```bash
nix develop ./os/nixos
nix build ./os/nixos#iso
nixos-rebuild switch --flake ./os/nixos#kestrel
```

`nix flake check ./os/nixos --no-build` validates evaluation. The ISO target is
`packages.x86_64-linux.iso` (aliased as `default`).

## Disk labels

The configuration mounts by label. Create them before the first install:

| Label | Mount | FS | Notes |
| --- | --- | --- | --- |
| `AETHER-BOOT` | `/boot` | vfat | 1 GiB |
| `aether-root` | `/` | xfs | 80 GiB |
| `aether-nix` | `/nix` | xfs | 120 GiB |
| `aether-home` | `/home` | xfs | remainder |
| `aether-scratch` | `/scratch` | ext4 | 60 GiB |

Format with:

```bash
mkfs.xfs -f -b size=4096 -L aether-root /dev/nvme0n1p2
mkfs.xfs -f -L aether-nix /dev/nvme0n1p3
mkfs.xfs -f -L aether-home /dev/nvme0n1p4
mkfs.ext4 -L aether-scratch /dev/nvme0n1p5
mkfs.vfat -F32 -n AETHER-BOOT /dev/nvme0n1p1
```

## First boot

1. systemd-boot starts the kernel; initrd mounts the XFS root.
2. systemd reaches `multi-user.target`; tty1 auto-logs in as `oliver`.
3. The `aether-shell` binary is not packaged yet. For P1 bring-up, run a
   terminal and verify the gate:
   ```bash
   systemd-analyze
   free -m
   cat /sys/kernel/security/lsm
   vainfo
   ```
4. Change the password: `passwd`.

## Kernel workflow

`kernel/kestrel.config` is a **fragment**, not a full `.config`. To tune on the
real machine:

```bash
zcat /proc/config.gz > /tmp/current.config     # if IKCONFIG_PROC is enabled
# boot a live image, run: make localmodconfig
# diff the result against the fragment and commit the delta
```

Rebuild with `nixos-rebuild switch`. Keep the previous generation; a bad kernel
is recovered in systemd-boot.

## Session units (P1/P3)

The session is deliberately not a systemd service yet. Once binaries exist:

```nix
systemd.services.aetherd = {
  wantedBy = [ "aether-session.target" ];
  serviceConfig.ExecStart = "${pkgs.aetherd}/bin/aetherd";
};
```

and `aether-shell` starts on tty1 with `WantedBy=graphical.target`. Until then,
`services.getty.autologinUser` keeps the machine usable for bring-up.

## What to verify in P1

- [ ] `uname -r` ends in `-aether`.
- [ ] `/sys/kernel/security/lsm` contains `landlock`.
- [ ] XFS mount options match `filesystem.nix` (`findmnt -o TARGET,SOURCE,FSTYPE,OPTIONS`).
- [ ] PipeWire sees analog, USB and Bluetooth devices (`wpctl status`).
- [ ] Idle RAM and CPU recorded, replacing the budgets.
- [ ] Userspace boot < 1.5 s (`systemd-analyze`).
