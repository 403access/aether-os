---
id: 0011-backend-neutral-shell
title: ADR-0011 — Backend-neutral shell state
sidebar_position: 12
---

# ADR-0011 — Backend-neutral shell state

**Status:** Accepted · **Date:** 2026-11-12

## Context

The compositor core was testable, but window/workspace/focus policy had no home.
Putting it in the Smithay backend would make apps depend on Wayland and make the
behavior impossible to test on macOS or in CI. A visual preview was also needed
before the Linux backend was ready.

## Decision

Add `aether-shell` as a backend-neutral state machine:

- windows, workspaces, focus, floating windows, overlay, frame plans;
- depends only on geometry/damage/layout/pacing/scene;
- no Wayland, GPU or input dependency;
- emits `FramePlan { viewport, damage, items, focus, workspace }`.

Add `aether-preview`, a software rasteriser and CLI that renders a plan to PNG.
The Smithay backend (P2) will be a separate crate consuming `FramePlan`; the shell
does not know about it.

## Consequences

- Window policy is unit-testable on any platform.
- The preview provides visual verification before the GPU/backend work.
- The backend has a narrow contract: map surfaces, present damage, route input.
- One more layer to maintain; accepted because it protects app independence.
- The preview is not a compositor and does not render client content.

## Alternatives

- **Policy inside the Smithay backend** — untestable outside Linux and couples
  apps to a toolkit.
- **Preview via a full compositor** — requires a display server and GPU before
  the basic behavior can be checked.
- **Wait for the backend** — leaves window policy undefined and blocks app work.
