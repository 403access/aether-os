---
id: 0009-water-design-system
title: ADR-0009 — Adopt the Water design system and reference explorer
sidebar_position: 10
---

# ADR-0009 — Adopt the Water design system and reference explorer

**Status:** Accepted · **Date:** 2026-11-12

## Context

Aether needs a visual language for a dense, keyboard-first workstation and a
two-pane file explorer. The Data Cloud Compare project already has both:

- `/Users/olivermolnar/Developer/jottacloud-comparison/design-system`
- `/Users/olivermolnar/Developer/jottacloud-comparison/app`
- Live reference: `http://localhost:8787/`

Inventing a new language would duplicate solved work and create drift between the
shell, docs and web surfaces.

## Decision

Vendor the Water design system (tokens, components, water effects, showcase) into
`packages/design-system` and rebuild the explorer's pane as `FilePane` in
`packages/ui`. Treat the reference instance as the visual baseline for parity work
in P2/P3.

## Consequences

- Four surfaces share one vocabulary and one token source.
- Provenance and a divergence policy are documented; upstream changes are re-imported
  deliberately, not merged blindly.
- The shell must map tokens to its scene-graph styles (P2).
- React is now a dependency of the web surfaces only.
- Accessibility rules and motion constraints come with the system.

## Alternatives

- **Design from scratch** — slower, riskier, and unnecessary.
- **Use an existing system (Radix, shadcn, Fluent)** — generic; would not match the
  explorer and would add dependencies.
- **Copy only the colours** — loses density, motion and component semantics.
- **Depend on the reference app at runtime** — unacceptable coupling.

## Follow-up

- Parity screenshots against the reference at 1440×900 (P2).
- Token → compositor style mapping (P2).
- Explorer virtualisation and column model (P3).
