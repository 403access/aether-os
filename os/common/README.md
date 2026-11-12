# `os/common` — shared composition layer

This directory is the **only** place where the physical target (`os/nixos`) and
the development VM (`os/vm`) share configuration.

## What may live here

- Locale, keyboard map, timezone-neutral presentation defaults.
- Nix feature flags (`nix-command`, `flakes`).
- Eventually: pure helper functions with no hardware, user, storage or security
  assumptions.

## What must not live here

| Category | Belongs in |
| --- | --- |
| CPU/kernel/GPU/audio hardware | `os/nixos/hardware`, `os/nixos/kernel.nix` |
| Disk layout, filesystems, bootloader | `os/nixos` (disko/filesystem), `os/vm` (image format) |
| Users, sudo, firewall, SSH policy | each target, explicitly |
| VM devices, overlays, serial console | `os/vm` |
| Agent isolation policy | `packages/agents` (shared code, not Nix modules) |

## Dependency direction

```text
os/nixos ─┐
          ├──> os/common/base.nix
os/vm ────┘

os/nixos  ──X──> os/vm      (forbidden)
os/vm     ──X──> os/nixos   (forbidden)
```

The boundary is enforced by `tests/separation.test.ts`, which runs in
`bun run test` / CI. It fails if:

- the reference configuration imports or references the VM tree,
- the VM imports the reference configuration,
- VM-only posture (passwordless sudo, firewall off, password SSH, VM disk)
  appears under `os/nixos`,
- the `kestrel` flake block includes the VM modules.

If a genuinely shared behavior is needed, move it into `os/common` (Nix) or
`packages/` (application code) — never copy it across targets.
