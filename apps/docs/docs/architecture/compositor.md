---
id: compositor
title: Compositor and shell
sidebar_position: 5
description: Smithay-based compositor, scene graph, damage tracking, frame pacing, tiling and overlays.
---

# Compositor and shell

`aether-shell` is one process that owns the display, input and the desktop
surface. There is no separate window manager, panel or launcher.

## Responsibilities

- Wayland compositor (Smithay) with a Vulkan/wgpu renderer.
- Scene graph of surfaces, including shell-owned overlays (palette, chat, status).
- Tiling layout and focus management.
- Frame pacing and damage tracking.
- Input routing, keybindings and pointer confinement.
- DMA-BUF export for capture and remote display.

## Scene graph

The core is a flat, ordered list of surfaces with a small set of layer types:

| Layer | Z | Input | Examples |
| --- | --- | --- | --- |
| Background | 0 | no | wallpaper, gradients |
| Windows | 10–49 | yes | browser, terminal, media |
| Floating | 50–79 | yes | dialogs, media mini-player |
| Overlays | 80–99 | modal | palette, chat drawer, device switcher |
| Chrome | 100 | yes | status bar, notifications |

Each surface carries:

```rust
pub struct Surface {
    pub id: SurfaceId,
    pub rect: Rect,          // logical coordinates
    pub layer: u8,
    pub buffer: BufferHandle,
    pub opaque: bool,
    pub damage: Region,      // accumulated since last present
}
```

The scene is sorted once per commit; z-order is stable and does not depend on
insertion time. Overlay focus is a stack, not a global modal flag.

## Damage tracking

Every mutation contributes to a damage region:

- surface move/resize → old and new rectangles
- buffer commit → the surface's reported damage
- text append in chat → the changed line only
- palette keystroke → input row plus the rows that changed

At present time the region is merged (rectangle union + coalescing) and clipped to
the output. If the merged region exceeds 60% of the output, the renderer upgrades to
a full redraw. This prevents pathological many-small-rect cases from costing more
than one pass.

The Rust core implements this in `os/compositor/crates/aether-scene` and is tested
without a GPU:

- moving a surface damages both old and new rects,
- merging adjacent rects reduces the count,
- clipping never produces negative extents,
- overflow falls back to full damage.

## Frame pacing

The pacer targets a deadline, not “as fast as possible”:

```text
frame N deadline = last_present + refresh_interval
if now > deadline - safety_margin: skip animation ticks, present latest state
else: schedule the next frame
```

Rules:

1. **Never queue frames.** A missed frame is dropped and superseded.
2. **Input wins.** If input arrives during a frame, the current frame is abandoned
   at the next safe point and a new one starts.
3. **Animations are time-based**, not frame-count-based, so a 60 Hz fallback does
   not run twice as fast.
4. **Idle means idle.** With no damage, no frame is submitted and no timer runs.

At 120 Hz the interval is 8.33 ms; the default safety margin is 1.2 ms. The pacer
reports achieved and missed frames to the status surface.

## Tiling layout

The default layout is a three-column master stack:

```text
┌───────────────┬───────────────┬───────────────┐
│               │               │               │
│    master     │     stack     │    stack      │
│               │               │               │
└───────────────┴───────────────┴───────────────┘
```

- Master width is a ratio (default 0.5), adjustable with `⌘H/L`.
- Windows are assigned to workspaces; workspaces map to outputs.
- Floating is per-surface and used only for overlays and dialogs.
- Gaps are 8 px outer, 6 px inner, drawn by the compositor, not by clients.

The layout engine is pure: `layout(surfaces, viewport, config) -> Vec<Rect>`. It is
tested with invariants (no overlap, full coverage of the work area, stable ordering)
rather than pixel snapshots.

## Input

Input is read from libinput, not from a display server:

1. Normalise to logical coordinates (fractional scale aware).
2. Route to the focused overlay first, then the focused window, then global keys.
3. Global keys are a compiled table, not a scripting language.
4. Pointer confinement and relative motion are explicit per-surface flags.

Key handling never blocks on a client. A client that does not respond to a
keyboard event within one frame loses focus to the shell's fallback handler.

## Overlays

The palette and chat are **shell layers**, not client windows. This means:

- No extra Wayland round trip to open them.
- They can render in the same draw pass as the status bar.
- Their state lives in the shell process, so input latency is one hop.
- They cannot outlive the shell or be captured as separate windows unless the user
  asks for it.

The trade-off is that a shell crash takes the overlays with it. That is acceptable
because the shell is small, audited and restarts in under 300 ms.

## Capture and DMA-BUF

Capture is implemented as a protocol the shell serves, not a screen-scraper:

- The shell exports the output buffer as a DMA-BUF handle.
- The encoder imports it without copying.
- Damage metadata travels with the frame so the encoder can use intra-refresh.
- Only one capture client is allowed at a time; the status bar shows a recording
  indicator that cannot be hidden.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| GPU hang | i915 reset; shell restarts; clients are reconnected or killed |
| Renderer OOM | Surface is dropped; shell shows a recovery surface |
| Client unresponsive | Surface marked “not responding”; close offered |
| Missed deadline | Frame dropped; animation ticks skipped; counter incremented |
| Output disconnect | Windows migrate to the remaining output; layout recomputed |
| Overlay infinite loop | Watchdog aborts the overlay task; shell stays live |

## What is not implemented yet

- The Smithay integration layer (adorner, udev backend, libinput seat).
- Real text shaping (cosmic-text or swash) and the font atlas.
- Fractional scaling polish and per-surface scale factors.
- HDR and colour management.
- The virtual seat used by remote display.

These are P2/P3 work. The pure core (geometry, damage, layout, pacing) is
implemented and tested now so the integration has a stable foundation.

## Performance budget

| Metric | Budget |
| --- | --- |
| Idle CPU (no damage) | 0% |
| Shell RSS | 30–50 MB |
| Frame time, 1440p, palette open | < 4 ms CPU + GPU |
| Input → damage commit | < 2 ms |
| Shell cold start | < 300 ms |

Measurements replace these budgets in P2.
