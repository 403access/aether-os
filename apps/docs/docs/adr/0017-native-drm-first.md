---
id: 0017-native-drm-first
title: "0017 — Native DRM first, cage as fallback"
sidebar_position: 17
---

# 0017 — Native DRM first, cage as fallback

- **Status:** accepted
- **Date:** 2026-11-14

## Context

The target session can either run the single-process `aether-shell-drm`
backend, which owns seat, libinput, KMS and Wayland itself, or run `cage` and
host the winit adapter inside it. The native backend is now implemented and
validated on a virtio-gpu VM, but cage remains a useful safety net for unknown
GPU/session combinations.

## Decision

`aether-session` starts `aether-shell-drm` first. If it exits non-zero, the
wrapper execs `cage -s -m extend -- aether-shell-smithay`. The fallback is
visible in the journal and is not silent.

The native backend stays the default even while it has less field time than
cage: the P2 gate is measured on it, and running cage by default would hide
native regressions.

## Consequences

- A native startup failure costs one process restart before the session works.
- `os/nixos/services.nix` must export Mesa/libglvnd/GBM paths for the native
  binary because Smithay dlopens `libEGL`/`libgbm`.
- Removing cage is a separate decision after the native backend survives daily
  use; it is not part of P2.
