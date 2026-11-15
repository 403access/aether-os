---
id: memory-scenarios
title: Memory scenarios
sidebar_position: 3
description: Ten implementable ways to hold the idle-memory budget, with effort, impact, risks and gates.
---

# Memory scenarios

[The audit](/architecture/memory-footprint) finds that idle memory is not a
property of *time*: it is what the session accumulated while it was being used.
The compositor blocks correctly at idle; the problem is that labels, panels and
backgrounds are cached forever, and the 200 MB budget has no gate.

This page is the implementation menu. Nothing here is committed code yet. Each
scenario states what it changes, why it matters, the expected effect, the risk it
creates, and the measurement that proves it worked.

## Summary

| ID | Scenario | Primary effect | Expected impact | Effort | Risk |
| --- | --- | --- | --- | --- | --- |
| S0 | Define and instrument the budget | Makes the claim testable | unblocks everything | S | none |
| S1 | Bounded, byte-accounted text/chrome caches | Removes unbounded growth | session delta → ~0; cap 20–30 MiB | M | low |
| S2 | Glyph atlas + layout cache | Removes per-label bitmaps | 5–10× less text memory, fewer draws | L | medium |
| S3 | One background per output | Stops full-screen style accumulation | saves 8–24 MiB per stale style | S | low |
| S4 | Compact the file index | Wired and allocation-light queries | −30–50% index RSS before BM25 | M | low |
| S5 | Measure and gate the process floor | IME is opt-in by default | 0 MiB default; 25–45 MiB when enabled | S | product |
| S6 | Idle reclamation (trim after use) | Returns freed memory | approaches boot footprint | M | medium |
| S7 | zram safety valve (optional) | Prevents OOM instead of a leak | no steady-state saving | S | latency |
| S8 | Release scan-out buffers on blank | Frees GPU memory while dark | 16–28 MiB at FHD/QHD | M | resume |
| S9 | Cache-bytes observability | Makes growth visible | no direct saving | S | none |

`S` ≈ half a day, `M` ≈ 1–2 days, `L` ≈ 3–5 days for one maintainer who knows the
tree. Impact figures are **modelled** from the audit; replace them with harness
output when implemented.

## Recommended order

1. **S0** — without a definition and a gate, every later claim is unverifiable.
2. **S1** — the largest and cheapest win; the only finding that can add hundreds
   of megabytes to an otherwise idle session.
3. **S3 + S9** — small changes that close the full-screen background case and make
   cache growth legible in the task manager.
4. **S5** — the IME is opt-in now; measure the enabled delta on the target and
   record the session floor.
5. **S2** — do it only if S1's cap is still routinely hit, or when real shaping
   lands. The atlas is the right long-term design but a bigger change.
6. **S4** — wiring and query allocation hygiene landed; finish the representation
   before full-text/BM25 indexing makes the index several times larger.
7. **S6, S8** — refinement after the caches are bounded; reclamation on top of an
   unbounded cache just hides the problem until it is touched again.
8. **S7** — a safety valve, never a substitute for S1.

## S0 — Define and instrument the budget

**Why.** "≤ 200 MB idle RAM" has not been measurable: the ledger used to name
`smem`, which was never installed; `free -m` is the documented fallback; and no
measurement distinguishes boot state from post-use state. Two engineers could
pass the claim with two different numbers.

**How.**

- Adopt the two surfaces from the audit: **attributed** (Σ PSS + resident DRM of
  session processes) as the engineering budget, **machine** (`MemTotal -
  MemAvailable`) as the reported cross-check.
- Add `scripts/memory-report.sh --seconds 60 --gate attributed --check` to the
  release checklist and to the P1 target run. Keep the JSON in `artifacts/perf/`.
- Record the result in the [budget ledger](/architecture/performance-engineering#the-budget-ledger)
  and [claims and evidence](/reference/claims-and-evidence), replacing "target"
  with "measured (target host)" only when the 7700K run exists.
- Add a second gate for **retention**: run the harness before and after a scripted
  10-minute workload; the post-use delta must stay under the same budget.
- Document the workload: palette typing, explorer scroll, chat stream, task
  manager open, video playback, theme switching. Without a fixed workload the
  number is not comparable.

**Gate.** `attributed ≤ 200 MiB` at boot and after the workload; idle growth
`≤ 1 MiB` over 10 minutes with no input.

**Risk.** The whole-machine number will likely exceed 200 MiB and force an honest
conversation. That is the point; hiding it behind `free -m` is worse.

## S1 — Bounded, byte-accounted caches

**Status.** Core implemented: `crates/aether-cache` provides `BudgetCache`, and
the text, icon, panel, round and top caches use it. Remaining from this scenario:
the pinned working set and the idle trim (S6).

**Why.** F1 and F2 are the only findings that make a *quieter* session heavier.
The caches were keyed by content and geometry and never evicted. A user who
watches one hour of windowed video (status bar visible) can add hundreds of
megabytes (audit, "Growth arithmetic") without touching the keyboard.

**How.**

1. Introduce one small `CacheBudget` utility in `aether-shell-wayland` (or a new
   `aether-cache` crate):
   - tracks entries, bytes and a monotonic generation counter;
   - evicts least-recently-used entries on insert when over budget;
   - computes bytes from `MemoryRenderBuffer` dimensions (`width × height × 4`)
     plus a fixed key overhead;
   - supports `trim_to(bytes)` for S6.
2. Wrap `text_cache` with a cap (for example 8–12 MiB) and record the byte total.
   Hit/miss is already explicit at `render/compose.rs:352`.
3. Give the chrome caches caps per class: panels 8–12 MiB, rounded fills 4 MiB,
   tops 2 MiB, icons 1 MiB. Keep `chrome_background` out of the generic cache;
   see S3.
4. Make eviction **generation based**, not “clear everything on every frame”:
   the palette and status bar re-render the same labels continuously and must keep
   hitting the cache while visible.
5. Keep a small “pinned” set for labels drawn on every frame (workspace names,
   static keycaps) so eviction does not cause per-frame rasterization.

**Expected impact.** Text and chrome stop growing after the working set is
reached. The steady-state cap is 20–30 MiB including GPU twins; the report's
one-hour video case drops from ~350 MiB to a flat cache. Re-rasterization on a
miss is already the normal path and costs sub-millisecond at these sizes.

**Risk.** A cap that is too small turns every frame into a rasterization storm
(CPU, not memory). The existing `AETHER_TEXT_STATS` counter is the guard: assert
rasterizations per static frame stay at zero. Do not cap below the working set of
the largest overlay (task manager, ~40 labels) with headroom.

**Gate.** After the scripted workload and ten idle minutes: attributed footprint
≤ budget, growth ≤ 1 MiB. Instrumentation: cache bytes and evictions exposed
through S9.

## S2 — Glyph atlas and text layout cache

**Why.** Even bounded, per-label bitmaps are an inefficient representation: every
unique string allocates a CPU buffer and a GPU texture, and the compositor issues
one draw per label. The same glyph is rasterized and uploaded hundreds of times
across labels. The architecture doc already lists "real text shaping and the font
atlas" as P2/P3 work.

**How.**

- Rasterize each `(glyph, px, color?)` once into an atlas texture, with a shelf or
  skyline packer. Colour can stay in a per-vertex tint rather than in the atlas
  key; that removes colour from the key space.
- Store shaped runs (glyph ids, advances, offsets) in an LRU keyed by string, px
  and style — a few hundred bytes per run instead of a full RGBA bitmap.
- Render text as textured quads through `TextureBuffer` / a custom
  `RenderElement`, replacing `MemoryRenderBuffer` per label.
- Keep `TextBitmap` for the software preview and the fallback bitmap font
  (ADR-0013); do not fork the layout code.

**Expected impact.** Text memory becomes the atlas (1–4 MiB) plus run metadata;
the per-label CPU/GPU twin disappears. Draw calls fall, which also helps the
frame-time budget. This is the design that makes a large palette and a dense
explorer cheap.

**Risk.** Medium-high: new render element, atlas invalidation on scale/theme
change, and a rewrite of label composition. Do not start before S1; S1 buys most
of the memory win for a fraction of the risk. If shaping (cosmic-text/swash) lands
first, build the atlas around its output rather than duplicating layout.

**Gate.** Same footprint gate, plus `text_rasterizations` per static frame = 0
and no frame-time regression against the P2 baseline.

## S3 — One background per output, not per style

**Status.** Implemented: the cache is keyed by output size and replaces the
buffer when the style changes; the byte budget evicts resize history.

**Why.** A full-output background is 7.9 MiB at 1080p (CPU) and another 7.9 MiB as
a GPU texture. The cache key used to include every theme field, so cycling modes,
tones, backgrounds, caustics, effects and speeds accumulated one full-screen pair
per combination visited.

**How.**

- Keep exactly one background buffer per output. On a theme change, replace it and
  drop the previous `Arc` (the GPU texture is released with the buffer).
- Do not key the cached background on style at all. Style changes are rare and
  already damage the full output.
- If theme previews are ever added, render them at thumbnail size, not full
  output.
- Consider drawing the gradient directly (solid/gradient shader) and reserving the
  bitmap for caustics/effects; that removes the steady-state 7.9 MiB entirely.

**Expected impact.** Removes an 8–24 MiB class of stale full-screen buffers and
the transient 16 MiB peak while a new style rasterizes (source pixmap plus copy).

**Risk.** Low. A theme switch already causes a full repaint; the only cost is
re-rasterizing on every switch instead of reusing a stale bitmap.

**Gate.** Attributed footprint after switching through every theme stays flat.
`chrome_background` reports at most one entry per output.

## S4 — Compact the file index

**Status.** Partial: `aether-session` exports the index socket and the shell uses
the daemon; titles are case-folded once and the palette selects top-k instead of
sorting every match. Remaining: the on-disk representation and connection reuse.

**Why.** The daemon is connected and small (~12–18 MiB modelled) because it
stores only filenames. It is also the component most likely to grow: the
documentation describes BM25/inverted indexing, which multiplies memory by the
number of unique tokens. Every query currently rebuilds allocations that could be
pruned at insert.

**How.**

- Keep the socket export in `aether-session`; the 2,000-file local scan is only
  the fallback when no service is configured.
- Store one record per document: an interned or arena-owned path plus a small
  flags/enum. Derive `title` (basename) and `subtitle` (parent + extension) at
  query time or cache them only for matched rows.
- Drop `haystack`: the path is already lowercased once; keep one lowercase copy,
  not a second full path.
- Case-fold the title once per document (done) so `score()` and the comparator do
  not allocate. Select top-k with `select_nth_unstable_by` before sorting the
  page (done).
- Reuse one connection per shell worker (or a small pool) instead of
  connect-per-query + thread-per-connection; this removes thread/arena churn.
- Keep the hard document limit and expose the current document count and
  approximate bytes in the hello/status line.

**Expected impact.** The cached titles and top-k remove the per-keystroke
allocation churn now; compacting the record is a few MiB today and the difference
between a bounded and an unbounded future when postings lists arrive.

**Risk.** Low; behaviour is covered by the existing index tests. The main trap is
re-deriving `title`/`subtitle` per hit and shifting the allocation to the response
path — cache them only after top-k selection.

**Gate.** `aether-index` PSS at 20k docs, before/after, plus palette p95 unchanged
on the existing bench. Record both in the claims table.

## S5 — Measure and gate the process floor

**Status.** Implemented as an opt-in: `aether.inputMethod.enable` defaults to
`false` in `os/nixos/input-method.nix`. Remaining: run the harness with it off and
on and record the delta as the session floor.

**Why.** The session's fixed cost is not only the shell. fcitx5 used to be
enabled by default, PipeWire/WirePlumber are required, and systemd/journald/udev/
D-Bus/NetworkManager come with the base. Without numbers, the 200 MB budget is a
guess about services the compositor does not own.

**How.**

- Keep the `text-input`/IME protocol support; it is independent of fcitx5.
- For CJK/IME input, add `aether.inputMethod.enable = true;` to
  `configuration.nix` and rebuild. One line, documented in the first-boot page.
- Run the harness with the option off and on; record both. The off run is the
  session floor.
- Audit `systemd --user` units and D-Bus activation at 60 s; remove anything that
  is not part of the product (the repository already disables printing, avahi,
  fwupd, SSH).
- Keep PipeWire and WirePlumber: audio is a commitment and the memory buys policy.

**Expected impact.** 0 MiB by default; 25–45 MiB when an operator opts in, and an
honest floor number the rest of the budget can be planned against.

**Risk.** Product/UX: an operator who needs an IME must be able to enable it in
one documented step. Do not silently remove input methods.

**Gate.** The attributed footprint with the IME off and on; the difference is
recorded in the ledger as the "session floor".

## S6 — Idle reclamation after use

**Why.** Even with caps, a session that has used the palette, explorer, chat and
task manager carries a warm working set. Dropping cold entries after a few idle
minutes returns the machine to its boot state without hurting interactive
latency.

**How.**

- Track last-use generation per cache entry (S1 already does).
- On a 2–5 minute idle timer, call `trim_to(warm_budget)` rather than clearing:
  keep the labels the current surface needs, drop everything else.
- Optionally drop the cached background and let the next frame re-rasterize it;
  a theme switch or a resume from blank already pays that cost.
- Call glibc `malloc_trim(0)` after trimming (Linux-only FFI) so freed arenas are
  returned to the kernel. Consider `MALLOC_ARENA_MAX=2` in the session unit to
  limit per-thread arena growth; measure before keeping it.
- Never trim on the input path: schedule the trim from the event loop when no
  frame is due.

**Expected impact.** Post-workload attributed footprint approaches the boot
footprint; makes the S0 "after use" gate achievable without relying on the cap
alone.

**Risk.** Medium: a badly timed trim can re-rasterize on the first keypress. Gate
input→damage p95 and the first-frame rasterization count after a trim.

**Gate.** Post-idle growth after the workload ≤ 1 MiB, input→damage unchanged, and
no new rasterization on the first static frame after resume.

## S7 — zram as a safety valve, not a fix

**Why.** The image currently has `zramSwap.enable = false`, no swap devices, and
`zswap.enabled=0` with `vm.swappiness=10`. A leak of the size F1 permits ends in
the OOM killer. On a 16 GiB machine, a small compressed swap device is cheap
insurance.

**How.** Enable a 1–2 GiB zram device with a conservative priority and test
suspend/reboot behaviour; keep `swappiness` low. Do not enable zswap on top of
zram. Record the decision as an ADR, because it changes the "no swap" posture.

**Expected impact.** No steady-state memory saving. Converts a hard OOM into
reclaim latency in the case where a cache regression slips through.

**Risk.** Latency: swapping a compositor is visible. This is why it is a valve,
not a budget strategy; S1 must land first.

**Gate.** zram inactive (no pages) during the idle workload and during normal use;
activation during a synthetic memory-pressure test only.

## S8 — Release scan-out buffers while the screen is off

**Why.** The swapchain keeps two buffers resident (15.8 MiB at 1080p, 28.1 MiB at
1440p). When the output is blanked or the session is inactive, those pages are
pure waste in the idle footprint.

**How.** On DPMS off / session pause, call the DrmOutput's buffer reset path and
drop the swapchain; on resume, mark full damage and let it re-allocate. Verify
with `fdinfo` that `drm-resident-system` falls while blank.

**Expected impact.** 16–28 MiB while the machine is idle with the display off.
This is the dominant GPU item in a dark room.

**Risk.** Medium: resume latency and the first frame after wake. Gate the
wake-to-first-present time and do not reset while a frame is in flight.

**Gate.** `drm-resident` drops to a small constant while blank, and wake-to-paint
stays within the input→paint budget.

## S9 — Observability: make cache bytes visible

**Status.** Partial: `AETHER_TEXT_STATS=1` now prints text cache bytes, entries,
evictions and per-class chrome bytes. The task-manager panel and the benchmark
JSON fields are still open.

**Why.** A memory regression is invisible until someone runs an external harness.
The task manager already shows per-process RSS; the shell knows its own cache
sizes.

**How.**

- Track bytes per cache (S1 gives this for free) and expose the total in the task
  manager Resources tab and optionally in `AETHER_TEXT_STATS` output.
- Add `cache_bytes`, `cache_entries`, `evictions` and `rasterizations` to the
  status JSON the benchmark already writes (`AETHER_FRAME_STATS`).
- Include them in the `memory-report.sh` JSON so a run correlates external PSS
  with internal cache accounting.

**Expected impact.** No direct saving, but it turns "why is the shell 300 MB after
a video?" into a number in the same panel that shows CPU.

**Risk.** Low; the task manager sample path is already once per second.

**Gate.** The counters move when the palette is typed in and are flat when idle.

## What not to do

- **Do not add jemalloc/mimalloc as a memory fix.** The performance page explicitly
  defers custom allocators until profiling shows allocator time. Allocation churn
  in the index is better fixed by not allocating.
- **Do not drop PipeWire, WirePlumber or the status bar** to fit a number. They
  are product commitments; measure and budget, do not amputate.
- **Do not enable swap/zram to hide F1.** It moves a leak to a latency problem.
- **Do not replace the filename index with full-text BM25 before S4.** The memory
  model changes by a multiple; compact the representation first.
- **Do not make the text cache global-and-cleared-on-idle.** Clearing the working
  set on every transition produces a rasterization storm on the next frame; bound
  and trim instead.
- **Do not quote the modelled numbers in this page or the audit as results.** They
  are there to prioritise work. The harness output is the result.

## Acceptance criteria

A release is allowed to claim the idle budget when all of the following are
recorded on the reference machine:

1. `scripts/memory-report.sh --seconds 60 --gate attributed --check` passes on a
   fresh boot with the documented process floor.
2. The same command passes after the scripted ten-minute workload and ten idle
   minutes, with `growth ≤ 1 MiB`.
3. A one-hour animated-content run (video or the chat demo) leaves the cache
   within its configured cap and does not change the attributed footprint by more
   than 1 MiB after trim.
4. The task manager reports cache bytes, and they are flat while idle.
5. The numbers are in the ledger and the claims page, labelled with the host and
   date.

Until then the ledger row stays **budget**, and every page that quotes 200 MB says
so.
