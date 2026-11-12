---
id: 0012-smithay-standalone-backend
title: ADR-0012 — Smithay backend as a standalone Linux crate
sidebar_position: 13
---

# ADR-0012 — Smithay backend as a standalone Linux crate

**Status:** Accepted · **Date:** 2026-11-12

## Context

The shell core (`aether-shell`) and software preview are platform-independent.
The first real backend must use Smithay (Wayland/EGL), which only builds on Linux.
Putting it in the default workspace would break macOS builds and make app code
depend on a display stack.

## Decision

Create `os/compositor/aether-shell-smithay` as a **standalone** Cargo crate:

- excluded from the root workspace (`[workspace]` in its manifest);
- depends on the shell/core by path and consumes `FramePlan`;
- winit/EGL backend first, udev/DRM later;
- a Nix dev shell (`.#smithay`) supplies Mesa/EGL/X11/Xvfb;
- an Xvfb smoke script provides headless verification;
- no app or core crate depends on it.

## Consequences

- macOS and the default Rust workspace stay toolkit-free and fast.
- The backend can evolve without destabilizing app-facing crates.
- Platform-specific build/CI is explicit and manual/optional.
- The adapter now renders client surfaces, routes input, and draws overlay and
  workspace chrome (including `aether-text` bitmap text); udev/DRM remains the
  target-specific backend.
- Two build paths exist (workspace vs standalone), which must be documented.

## Alternatives

- **Workspace member** — breaks macOS and couples apps to Wayland.
- **Feature-gated workspace member** — still resolves Smithay and complicates the
  lockfile and CI; the boundary is clearer as a separate crate.
- **Use `aether-preview` as the real backend** — no Window/Wayland support.
- **Skip Smithay and write DRM directly** — more work and no client protocol
  ecosystem; Smithay is the chosen foundation.
