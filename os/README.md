# Aether OS — NixOS layer

Status: **installable, evaluation-verified, not yet booted in CI.** The installed
system and the live installer ISO both evaluate cleanly with `nix flake check`.
An actual boot requires x86_64 hardware.

## Flake outputs

| Output | Purpose |
| --- | --- |
| `nixosConfigurations.kestrel` | The installed system |
| `nixosConfigurations.installer` | Live installer environment |
| `packages.x86_64-linux.iso` | Installer ISO (alias: `default`, `installer-iso`) |
| `diskoConfigurations.kestrel` | Disk layout for `disko --flake` |
| `packages.x86_64-linux.aether-scene` | Rust scene/pacing demo |
| `devShells.x86_64-linux.default` | Bun, Rust, Nix tooling |

## Layout

```text
flake.nix / flake.lock       inputs, systems, ISO, disko config (repo root)
os/nixos/
├── configuration.nix        system-wide settings, boot, network, packages
├── hardware/kabylake-7700k.nix  CPU/GPU/audio/NVMe facts, microcode
├── kernel.nix                custom kernel + cmdline + sysctl
├── kernel/kestrel.config     config fragment (appended to defconfig)
├── filesystem.nix            XFS root, ext4 scratch, tmpfs index
├── disko.nix                 GPT layout (ESP, root, nix, scratch, home)
├── disk.nix                  the ONE disk device you edit
├── installer.nix             live ISO: git, disko, helix, probe tools
├── services.nix              PipeWire, WirePlumber, udev, journald
└── users.nix                 operator, service user, agent UID range
```

## Development VM (separate target)

`os/vm/` boots Aether in a local aarch64 VM (Apple Silicon/QEMU) for testing. It
does not import this directory and must never be merged into `kestrel`.

```bash
./scripts/vm-build.sh
./scripts/vm-run.sh
```

See `os/vm/README.md` and the docs page `getting-started/virtual-machine.md`.

## Validate

```bash
nix flake check --no-build     # evaluation (any host)
nix eval .#diskoConfigurations.kestrel --json
```

Evaluation is what CI can do on macOS/ARM. Building the ISO or the system closure
requires an x86_64 Linux host with Nix (or a remote builder).

## Build the installer ISO

```bash
nix build .#iso
ls result/iso/
```

Then write it to a USB stick:

```bash
sudo dd if=result/iso/*.iso of=/dev/rdiskN bs=4m status=progress   # macOS
sudo dd if=result/iso/*.iso of=/dev/sdX bs=4M status=progress oflag=sync  # Linux
```

You do **not** need this ISO to install Aether. Any current NixOS minimal ISO
works; the install guide fetches the flake from a clone. The custom ISO just
preinstalls `git`, `disko`, `helix` and hardware probes.

## Install

On the live ISO, from a clone of this repository:

```bash
# 1. identify the target
lsblk -o NAME,SIZE,MODEL,SERIAL,TYPE,MOUNTPOINTS

# 2. set it in os/nixos/disk.nix
#    device = "/dev/nvme0n1";

# 3. install (wipes the target; asks for confirmation)
sudo ./scripts/install.sh /dev/nvme0n1
```

Manual equivalent:

```bash
sudo nix --extra-experimental-features "nix-command flakes" \
  run nixpkgs#disko -- --mode disko --flake .#kestrel

sudo nixos-install --root /mnt --flake .#kestrel --no-root-passwd
```

Full instructions, firmware settings, recovery and hardware checks:

- `apps/docs/docs/getting-started/install.md`
- `apps/docs/docs/getting-started/first-boot.md`
- `apps/docs/docs/getting-started/hardware.md`

## Disk layout

`disko.nix` creates one GPT disk:

| Partition | Size | FS | Mount | Label |
| --- | --- | --- | --- | --- |
| `01-esp` | 1 GiB | vfat | `/boot` | `AETHER-BOOT` |
| `02-root` | 80 GiB | xfs | `/` | `aether-root` |
| `03-nix` | 120 GiB | xfs | `/nix` | `aether-nix` |
| `04-scratch` | 60 GiB | ext4 | `/scratch` | `aether-scratch` |
| `05-home` | remainder | xfs | `/home` | `aether-home` |

The mount configuration uses `/dev/disk/by-label/*`, so labels are part of the
contract. XFS labels are limited to 12 characters; do not rename them without
updating `filesystem.nix`.

## First boot

1. Reboot, remove the USB, choose the NixOS/Aether entry in the UEFI menu.
2. Log in on tty1 as `oliver` (initial password `aether`) and run `passwd`.
3. Follow the first-boot checklist: `apps/docs/docs/getting-started/first-boot.md`.

Expected at this milestone:

- Custom kernel ending in `-aether`, Landlock in `/sys/kernel/security/lsm`.
- XFS/ext4/tmpfs mounts per `filesystem.nix`.
- PipeWire/WirePlumber running; NetworkManager running.
- `aether-scene` demo available.
- **No compositor and no aetherd/index services yet** — P2/P3.

## Kernel workflow

`kernel/kestrel.config` is a **fragment**, not a full `.config`. To tune on the
real machine:

```bash
zcat /proc/config.gz > /tmp/current.config     # if IKCONFIG_PROC is enabled
# or run make localmodconfig against the nixpkgs kernel source
# diff the result and commit only the deliberate deltas
```

Rebuild with `nixos-rebuild switch --flake .#kestrel`. Keep the previous generation;
a bad kernel is recovered in systemd-boot.

## What still needs to happen on hardware (P1 gate)

- [ ] First boot of `nixosConfigurations.kestrel`.
- [ ] `uname -r` ends in `-aether`.
- [ ] `/sys/kernel/security/lsm` contains `landlock`.
- [ ] Mounts match `filesystem.nix`.
- [ ] `vainfo` opens the iHD driver.
- [ ] `wpctl status` shows a default sink; analog/USB/Bluetooth enumerated.
- [ ] Idle RAM/CPU and userspace boot time recorded, replacing budgets.
- [ ] `localmodconfig` delta reviewed.

## Legacy BIOS and encryption

Neither is supported by the default install path. BIOS requires a 1 MiB `EF02`
partition and GRUB; LUKS requires a disko encryption layer and initrd changes.
Both are documented as forks in the install guide.
