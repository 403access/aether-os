---
id: virtual-machine
title: Boot the development VM
sidebar_position: 4
description: "Build and boot the separate aarch64 VM target on an Apple Silicon Mac, and why it is not the install target."
---

# Boot the development VM

The reference machine is an i7-7700K. This page is for booting Aether **on the Mac
you develop on**, inside a local virtual machine, without contaminating the
hardware installation target.

The VM is a **development harness**:

- It boots a real Aether NixOS base (systemd, SSH, tooling, the same repo).
- It is **not** the reference system: no custom 7700K kernel, no XFS layout, no
  PipeWire, no agent supervisor, no compositor.
- It is **not** an alternative supported install. The P1 gate still requires the
  hardware.

## Separation policy

VM-only settings live in `os/vm/` and are exposed as
`packages.aarch64-linux.vm-image`. They are never imported by
`nixosConfigurations.kestrel`.

| | `os/nixos` (reference) | `os/vm` (this page) |
| --- | --- | --- |
| CPU / kernel | i7-7700K + custom fragment | generic aarch64 kernel |
| Storage | XFS root/nix/home, ext4 scratch | ext4 qcow2 image |
| Audio | PipeWire + WirePlumber | disabled |
| Users | `oliver`, locked root | `aether`, password login |
| Sudo | password required | passwordless |
| Firewall | enabled | disabled (user-mode NAT) |
| Purpose | install on hardware | boot and test the base |

Shared behavior belongs in `packages/` or `os/compositor/`, not in either
configuration. The only shared Nix module is `os/common/base.nix` (locale,
keymap, flake support); it contains no hardware, users, storage or security
policy.

### The boundary is enforced

`tests/separation.test.ts` runs with `bun run test` and CI. It fails if:

- `os/nixos` references `os/vm` or configures VM disks,
- `os/vm` imports `os/nixos`,
- VM-only posture (passwordless sudo, firewall off, password SSH) appears in the
  reference,
- the `kestrel` flake block includes the VM tree,
- the VM package stops using only `os/vm/configuration.nix`.

## Requirements

| Item | Requirement |
| --- | --- |
| Host | Apple Silicon Mac (this VM target is `aarch64-linux`) |
| Builder | OrbStack/Docker, or any aarch64/x86_64 Linux machine with Nix |
| QEMU | `brew install qemu` (tested with 11.1) |
| RAM | 8 GiB free; the VM defaults to 4 GiB |
| Disk | ~10 GiB for the builder store and image |

Why `aarch64`? The host is ARM. Emulating x86_64 would be slow and would not
exercise the same kernel. The VM proves the NixOS base and packaging; the CPU
specific target remains `kestrel`.

## Build

```bash
./scripts/vm-build.sh
```

This runs `nix build .#vm-image` inside a `nixos/nix` Linux container, then copies
the qcow2 image to `artifacts/vm/`. The first run downloads nixpkgs and the system
closure (10–30 minutes); later builds reuse the `aether-vm-nix` Docker volume.
The image is about 2.9 GiB. The image builder is a NixOS disk-image derivation; it
asks for the `kvm` system feature even though it only needs `qemu-img`, so the
script enables that feature explicitly. No `/dev/kvm` is required inside the
container.

On a Linux machine with Nix, skip the container:

```bash
SKIP_DOCKER=1 ./scripts/vm-build.sh
# or
nix build .#vm-image
```

Artifacts are gitignored.

:::note
`nixos-generators` is deprecated upstream in favour of the image-building support
in NixOS itself. It is pinned here because the qcow-efi format is stable and this
is a development-only target; migrating the VM output is tracked as cleanup, not
part of the reference install.
:::

## Boot

```bash
./scripts/vm-run.sh
```

QEMU uses HVF acceleration and the EDK2 aarch64 firmware. You get a serial
console; quit with `Ctrl-A X`.

The script boots a **writable overlay** (`artifacts/vm/run.qcow2`) on top of the
pristine build, so the image is never modified. Delete the overlay (or set
`VM_RESET=1`) to reset the VM to first boot:

```bash
rm artifacts/vm/run.qcow2
```

| Field | Value |
| --- | --- |
| Login | `aether` / `aether` (VM only) |
| Sudo | passwordless |
| SSH | `ssh -p 2222 aether@127.0.0.1` |
| Console | serial (`ttyAMA0`), plus tty1 |

## Headless smoke test

```bash
./scripts/vm-smoke.sh
```

It boots without a display, waits up to 240 s for a marker written by a VM-only
systemd unit, then exits:

```text
AETHER_VM_READY arch=aarch64 kernel=6.18.62
AETHER_VM_LSM=lockdown,capability,landlock,yama,bpf,ima
AETHER_VM_USER=root
boot ok after 10s
```

That is the measured result on the development Mac (M3, HVF, 4 vCPU, 4 GiB).

The serial log is kept at `artifacts/vm/serial.log`. This is the VM equivalent of
the first-boot checklist, not a substitute for it.

## What runs in the VM

| Component | State |
| --- | --- |
| systemd multi-user, DHCP, SSH | enabled |
| `helix`, `tmux`, `htop`, `git` | installed |
| PipeWire, agent daemons, compositor | **not installed** |
| Firewall | disabled (user-mode NAT only) |
| Passwordless sudo | enabled for convenience |

The VM is an appliance: to change its configuration, edit `os/vm/configuration.nix`
and rebuild on the host with `./scripts/vm-build.sh`, then reset the overlay and
boot again. The build image is not a NixOS flake checkout and does not expose a
`nixosConfigurations` entry.

## Changing resources

```bash
VM_MEM=8192 VM_CPUS=6 ./scripts/vm-run.sh
```

Disk size is set in `os/vm/configuration.nix` (`virtualisation.diskSize`, MiB) and
takes effect on the next image build. The qcow2 is sparse; `qemu-img resize` can
grow it later.

## Cleaning up

```bash
rm -rf artifacts/
docker volume rm aether-vm-nix
```

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `docker: daemon not running` | OrbStack stopped | Open OrbStack, retry |
| Build killed while downloading | Container restart | Re-run; the store volume resumes |
| QEMU exits immediately | Missing EDK2 firmware | `brew install qemu`; or set `QEMU_EFI` |
| `HVF` error | Nested virtualization/unsupported host | Remove `-accel hvf` (slow TCG) |
| No login prompt | Console wrong | Use `-serial`/`-nographic`; kernel console is `ttyAMA0` |
| `Could not open ... Permission denied` | Old script booted the read-only image | Re-run; the current script uses a writable overlay |
| SSH refused | VM still booting or host port busy | Wait for the marker; change `VM_SSH_PORT` |

## What this does not prove

- That the i7-7700K kernel or disk layout boots.
- That PipeWire, the compositor or agent isolation work on the target.
- That the installer ISO boots on your firmware.

Those remain the P1/P2 gates. The VM makes it possible to iterate on the base
without touching the machine that matters.
