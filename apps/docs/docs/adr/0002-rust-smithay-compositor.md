---
id: 0002-rust-smithay-compositor
title: ADR-0002 — Rust + Smithay compositor
sidebar_position: 3
---

# ADR-0002 — Rust + Smithay compositor

**Status:** Accepted · **Date:** 2026-11-11

## Context

The compositor owns the display, input and the shell's UI layers. It must be small,
safe under concurrency, and fast enough to keep a 120 Hz deadline while rendering
palette and chat layers.

## Decision

Write `aether-shell` in Rust on Smithay, with a wgpu/Vulkan renderer. Implement the
pure scene graph, damage tracking, layout and frame pacing as a dependency-free
crate so they can be tested without a GPU.

## Consequences

- Memory safety and fearless concurrency in the hot path.
- A smaller ecosystem than Qt/GTK; some Wayland protocols must be implemented.
- Text shaping and IME require additional crates (cosmic-text/swash, ibus).
- The pure core is testable in CI today; the integration is P2.

## Alternatives

- **Hyprland + Quickshell** — fast to ship, but pulls in Qt/QML and a second
  compositor architecture. Rejected for footprint and control.
- **Mutter/KWin** — far too large; GNOME/KDE dependencies.
- **wlroots** — C, good protocol coverage, but a smaller safety story and a
  different architecture (no Smithay-style compositor framework).
- **Custom C compositor** — maximum control, unacceptable memory-safety risk.
