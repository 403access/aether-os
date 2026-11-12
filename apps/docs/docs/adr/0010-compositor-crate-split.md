---
id: 0010-compositor-crate-split
title: ADR-0010 — Split the compositor core into reusable crates
sidebar_position: 11
---

# ADR-0010 — Split the compositor core into reusable crates

**Status:** Accepted · **Date:** 2026-11-12

## Context

The compositor core started as one crate (`aether-scene`) with internal modules.
That is fine for a demo, but apps will eventually be built on geometry, damage,
layout and pacing without wanting the whole shell. A single crate also makes it
easy for a GPU/Wayland dependency to leak into pure logic.

## Decision

Split `os/compositor` into small crates with a one-way dependency graph:

```text
aether-geometry   → aether-damage → aether-scene
        ↘            aether-layout
aether-pacing (leaf)
aether-compositor (facade + demo binary)
```

- No GPU, Wayland or input dependencies in any core crate.
- Every crate is `#![forbid(unsafe_code)]`.
- Apps depend on the smallest crate they need; `aether-compositor` provides a
  facade and `prelude` for shell-side code.
- The Smithay/Vulkan backend (P2) will be a separate `aether-shell` crate that
  depends on this workspace, never the reverse.

## Consequences

- Clear reuse boundaries and smaller compile units.
- More manifests and a slightly larger workspace; acceptable for the clarity.
- Versioning is per crate, with `SCENE_MODEL_VERSION` for scene semantics.
- The Nix package builds `aether-compositor` and installs the demo binary.
- Tests remain in the crate that owns the behavior; one integration test uses the
  facade.

## Alternatives

- **One crate with modules** — simpler today, but applications would pull in
  everything and backend code could creep into pure logic.
- **One crate per type** — excessive fragmentation; geometry/damage/layout are
  already the natural boundaries.
- **A separate repository** — no benefit at this size; the monorepo keeps the
  contract and tests together.
