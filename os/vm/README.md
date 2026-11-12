# `os/vm` — development VM target

This directory exists so Aether can be booted and tested on the development
machine without touching the real installation target.

## Separation rules

| | `os/nixos` (reference) | `os/vm` (this directory) |
| --- | --- | --- |
| CPU | i7-7700K, custom kernel fragment | generic aarch64 kernel |
| Storage | XFS root/nix/home, ext4 scratch | ext4 qcow2 image |
| Audio | PipeWire + WirePlumber | disabled |
| Daemons | aetherd/index planned | not installed |
| Users | `oliver`, locked root | `aether`, password login |
| Sudo | password required | passwordless |
| Firewall | enabled | disabled (user-mode NAT) |
| Purpose | install on hardware | boot/test the base |

**Do not import `os/nixos` from here, and do not copy anything from here into
`os/nixos` unless it is a genuinely shared fix.** Shared behavior belongs in
`os/common/` (target-agnostic Nix), `packages/` or `os/compositor/`.

Both targets import `os/common/base.nix` (locale, keymap, flakes). The boundary
is enforced by `tests/separation.test.ts`, not just by convention.

## Build

The VM is `aarch64-linux`, so it must be built on a Linux builder. On this Mac,
OrbStack provides one:

```bash
./scripts/vm-build.sh
```

Equivalent on any x86_64/arm64 Linux host with Nix:

```bash
nix build .#vm-image
```

Artifacts are copied to `artifacts/vm/` (gitignored).

## Boot

```bash
./scripts/vm-run.sh          # interactive serial console
./scripts/vm-smoke.sh        # headless boot, waits for AETHER_VM_READY
```

QEMU is driven directly with HVF acceleration and the EDK2 aarch64 firmware.
The image is `qcow-efi`, so no bootloader setup is needed on the host.

- Login: `aether` / `aether`
- SSH: `ssh -p 2222 aether@127.0.0.1`
- Serial log: `artifacts/vm/serial.log`
- The build image is read-only; boots use a writable overlay (`run.qcow2`), so
  the base stays pristine. Delete the overlay or set `VM_RESET=1` to reset.

Measured on the development Mac (M3, HVF, 4 vCPU, 4 GiB): **multi-user in ~10 s**,
`AETHER_VM_LSM=lockdown,capability,landlock,yama,bpf,ima`.

## Not the product

There is no compositor, no agent supervisor and no reference kernel here. This
image proves the base boots and is useful for testing packaging, systemd units
and future shell integration. The P1 hardware gate still requires the i7-7700K.
