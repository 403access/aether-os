---
id: 0013-bitmap-text
title: ADR-0013 — Bitmap text for compositor chrome
sidebar_position: 14
---

# ADR-0013 — Bitmap text for compositor chrome

**Status:** Accepted · **Date:** 2026-11-12

## Context

The compositor owns chrome that clients cannot draw: workspace names, the shell
overlay header/search/results/footer and shortcut hints. The software preview
already had an 8×8 bitmap font, and the winit backend needed to show the same
labels without introducing a full font stack.

Options were a system font stack (FreeType/fontconfig), an embedded vector-font
rasterizer, or sharing the existing bitmap approach. A display backend should not
depend on host font configuration, and deterministic screenshots matter for
tests.

## Decision

Add `aether-text`, a small crate that rasterizes 8×8 glyphs (`font8x8`) into
tightly cropped RGBA buffers:

- no fontconfig, FreeType, system fonts or glyph atlas;
- one implementation shared by `aether-preview` (composites pixels) and the
  Smithay backend (uploads `MemoryRenderBuffer` with `Fourcc::Abgr8888`);
- unknown characters keep their advance so layout stays stable;
- per-frame buffers for now; caching/atlas is a later performance step.

## Consequences

- Preview, tests and every backend render identical labels.
- No host font dependency and a very small dependency footprint.
- Limited typography: one 8×8 face, integer scaling only, limited Unicode.
- Text-heavy apps still need their own text stack; this is chrome text, not a
  document renderer.
- Rebuilding small buffers each frame is acceptable for labels but must be
  cached before text-heavy UI.

## Alternatives

- **FreeType/fontconfig** — more capable, but a large dependency and
  non-deterministic across hosts.
- **Embedded TTF rasterizer** — better typography, much more code and data.
- **Glyph atlas immediately** — premature; caching can be added behind the same
  `TextBitmap` API.
- **Client-drawn chrome** — the overlay belongs to the compositor, not a client.
