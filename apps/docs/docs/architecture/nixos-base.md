---
id: nixos-base
title: NixOS base
sidebar_position: 4
description: Why the base is declarative, how the flake is structured, and how rollback works.
---

# NixOS base

The base system is NixOS: minimal profile, pinned nixpkgs, declarative hardware,
and a small number of services. It is the only part of Aether that is not custom
code, and it exists to make the custom code reproducible.

## Why NixOS

| Requirement | How NixOS satisfies it |
| --- | --- |
| Reproducible builds | Content-addressed store, pinned inputs |
| Atomic upgrades | Generations, rollback at boot |
| No configuration drift | Declarative modules; no manual edits to `/etc` |
| Isolated build environments | Every package builds in its own sandbox |
| Auditable system closure | `nix-store --query --requisites` |

The cost is a learning curve and a larger store. The benefit is that “works on my
machine” becomes a hash.

## Flake layout

The flake lives at the **repository root** so `nixos-install` copies the whole
source tree to `/etc/nixos` and rebuilds keep working. `os/nixos` contains the
machine configuration.

```text
flake.nix / flake.lock        inputs, systems, ISO, disko config, package
os/common/base.nix            only shared module (locale, keymap, flakes)
os/nixos/
├── configuration.nix         # system-wide modules and services
├── hardware/
│   └── kabylake-7700k.nix    # CPU, GPU, audio, NVMe facts
├── kernel.nix                # custom kernel package and cmdline
├── kernel/kestrel.config     # kernel config fragment
├── filesystem.nix            # XFS mount options, scratch ext4, tmpfs
├── disk.nix                  # the one disk device to install on
├── disko.nix                 # GPT layout used by the installer
├── installer.nix             # live ISO environment
├── services.nix              # PipeWire, WirePlumber, udev, journald
└── users.nix                 # operator user, agent UIDs, groups
os/compositor/                # Rust scene/pacing crate packaged by the overlay
```

The flake exposes:

```nix
nixosConfigurations.kestrel   = ...;  # installed system
nixosConfigurations.installer = ...;  # live ISO
packages.x86_64-linux.iso     = ...;  # installer image
diskoConfigurations.kestrel   = ...;  # disk layout
packages.x86_64-linux.aether-scene = ...;
```

## Development VM

A separate `os/vm` target boots an aarch64 qcow2 image for local testing. It
imports only `os/common/base.nix` and never imports this configuration; the boundary
is enforced by `tests/separation.test.ts`. See
[boot the development VM](/getting-started/virtual-machine).

## Pinning

Inputs are pinned in `flake.lock` and updated deliberately:

```bash
nix flake update nixpkgs          # explicit, reviewed
nix flake check                   # evaluation + build where possible
nixos-rebuild switch --flake .#kestrel
```

No channel follows master. A kernel update is a commit with a boot measurement and
a rollback note.

## Generations and rollback

Every `nixos-rebuild switch` creates a generation. systemd-boot lists the last five.
If a kernel or compositor change prevents first paint:

1. Reboot and select the previous generation in systemd-boot.
2. `nixos-rebuild switch --rollback` to restore the service set.
3. Record the failure in the audit log and open an ADR if the design is implicated.

The data volume is separate from the system closure, so rollback never touches
`/home` or the archive.

## Overlay: target-specific builds

The overlay compiles Aether binaries with the reference CPU:

```nix
aether-shell = prev.rustPlatform.buildRustPackage {
  pname = "aether-shell";
  # ...
  RUSTFLAGS = "-C target-cpu=native -C codegen-units=1";
  cargoBuildFlags = [ "--release" "--features" "vulkan" ];
};
```

`target-cpu=native` makes the store path machine-specific. That is fine: the store
is not shared with other machines. Portability is explicitly not a goal.

## Users and groups

| Account | Purpose | Login |
| --- | --- | --- |
| `oliver` | operator | yes |
| `aether` | service user for daemons | no |
| `agent-<id>` | per-agent UID, allocated at spawn | no |
| `media` | sandboxed media client | no |

Per-agent UIDs are how the kernel enforces “agents may not read each other’s
files” even before Landlock rules are applied.

## Secrets

- Disk encryption keys live in the initrd, not the store.
- API tokens for optional hosted inference live in the operator’s `~/.config`, mode
  0600, readable only by the chat process via a capability.
- No secrets in the Nix store. `sops-nix` is not used in v1.0; the single-user
  threat model does not justify the complexity.
- Agent environments never inherit the operator’s environment; variables are
  allowlisted.

## Filesystem layout

| Mount | Filesystem | Options |
| --- | --- | --- |
| `/` | XFS | `noatime,largeio,inode64` |
| `/nix` | XFS (same volume) | `noatime` |
| `/home` | XFS | `noatime` |
| `/scratch` | ext4 | `noatime,commit=60,data=writeback` |
| `/var/index` | tmpfs | `size=2G,mode=0700` |
| `/mnt/archive` | external / network | read-only by default |

See [files and storage](/architecture/files-and-storage) for the reasoning.

## What is not declarative (yet)

- The compositor’s runtime state (workspaces, window positions) is a local file,
  not a Nix option.
- Audio routes are persisted by WirePlumber.
- Agent images for the microVM tier are content-addressed tarballs, not Nix
  derivations, until P8.

Each gap is documented; none of them silently mutate `/etc`.

## Failure modes and rollback

| Failure | Symptom | Recovery |
| --- | --- | --- |
| Bad kernel config | No boot / no display | Previous generation in systemd-boot |
| Missing firmware | iGPU or Wi-Fi absent | Add `hardware.enableRedistributableFirmware` or vendor firmware |
| Store corruption | Random build failures | `nix-store --verify --check-contents` |
| Failed activation | Service does not start | `nixos-rebuild switch --rollback`, inspect journal |
| Flake input drift | Build differs from CI | Pin the input; never build from a dirty flake |

## Verification

- `nix flake check --no-build` is a manual Linux workflow; both targets are evaluated locally during development. The VM image is booted headlessly as a smoke test.
- `nixos-rebuild build-vm` smoke test for the service graph.
- `systemd-analyze critical-chain` recorded with every release.
