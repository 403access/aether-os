---
id: 0016-gles-vs-wgpu
title: "0016 — GLES/EGL now, wgpu deferred"
sidebar_position: 16
---

# 0016 — GLES/EGL now, wgpu deferred

- **Status:** accepted
- **Date:** 2026-11-14
- **Supersedes:** the renderer wording in ADR-0002, not its Smithay decision.

## Context

ADR-0002 named a wgpu/Vulkan renderer. The implemented and tested backend uses
Smithay's `GlesRenderer` over EGL/GBM for both winit and DRM scan-out. The
question is whether to port before closing P2.

## Evidence

- Frame instrumentation (`AETHER_FRAME_STATS`, `scripts/frame-stats.py`) shows the
  current off-target cost is dominated by the host and Xvfb, not GLES: a palette
  run reports p50 input→damage ≈ 24 µs, while present intervals include xdotool
  sleeps and software rasterization.
- The renderer is not the measured bottleneck in any P2 task. Damage, chrome text
  caching, partial clears and the deadline loop all apply equally to GLES.
- Smithay's DRM path (`DrmCompositor`, GBM modifiers, direct scan-out) is written
  against the GLES renderer and is validated on virtio-gpu.
- A wgpu port would add a second renderer path, a new dependency tree and no
  known gate improvement.

## Decision

Keep GLES/EGL for P2 and the first hardware release. Do not port the renderer as
part of the gate work.

Revisit if a measured p99 frame cost is attributable to GLES after G1/G2/C1/C2,
or if a required feature (e.g. a specific DMA-BUF format or capture path) cannot
be implemented with the GLES renderer.

## Consequences

- `ComposedFrame`/`AetherRenderElement` stay tied to `GlesRenderer` for now.
- Documentation and roadmap copy must say GLES/EGL, with wgpu marked deferred.
- The renderer abstraction seam remains `AetherRenderElement`; a future port
  replaces that enum and the two presenters, not shell policy.
