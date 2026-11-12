---
id: 0001-nixos-base
title: ADR-0001 — NixOS as the base
sidebar_position: 2
---

# ADR-0001 — NixOS as the base

**Status:** Accepted · **Date:** 2026-11-11

## Context

The OS needs a reproducible base, atomic upgrades, rollback, isolated build
environments and declarative hardware configuration. The custom parts (kernel,
compositor, daemons) must be buildable without polluting a global root filesystem.

## Decision

Use NixOS with a pinned flake, a minimal profile, and custom packages built with
`-C target-cpu=native`. The base is declarative; runtime state is explicitly
documented as non-declarative.

## Consequences

- Reproducible system closures and one-command rollback.
- A steep learning curve and a large store.
- `target-cpu=native` store paths are machine-specific — acceptable for one target.
- Patching a package sometimes means writing a Nix overlay, not editing source in
  place.

## Alternatives

- **Arch Linux** — rolling, simple packaging, no atomic rollback. Rejected because
  reproducibility is a stated goal.
- **Debian minimal** — stable but slow-moving; custom packaging is more work.
- **Build from LFS** — maximal control, maximal maintenance; not the project.
- **Fedora Atomic** — good immutability, weaker declarative config and rollback
  ergonomics for custom kernels.
