---
id: p2-completion
title: P2 completion plan
sidebar_position: 2
description: Sequenced, actionable tasks from the current tree to the P2 gate, with technical descriptions, touch points and acceptance criteria.
---

# P2 completion plan

**Snapshot:** `3addb30`, 2026-11-14. The tree moves quickly; re-verify the baseline
before starting and update line references as they drift.

This page turns the P2 audit into work. Every task has a technical description, the
files it touches, a measurable "done when", and the evidence it must leave behind.
A task is complete when its acceptance criterion is demonstrated, not when the diff
merges — the same rule as the phase gates.

## Baseline at snapshot

| Area | State | Evidence |
| --- | --- | --- |
| Native DRM + libinput backend | Exists, single process | `aether-shell-drm/src/main.rs` (769 lines): libseat, libinput, `DrmCompositor`, multi-device, EDID preferred mode, multi-output origins |
| Session | Native first, cage fallback | `os/nixos/services.nix:63`, `flake.nix` `aether-session` |
| Wayland frontend | Shared, backend-neutral | `aether-shell-wayland` (2,691 lines): protocol, input policy, `compose()`, `take_damage()` |
| Feature surface | Effectively complete | 23 xdotool smokes; palette/app/file/index/agent providers, clipboard + persistence, fullscreen, IME, DnD, cursors |
| Tests | 89 in default workspace; 8 in standalone crates | `cargo test`; `aether-shell-wayland` 5, `aether-shell-drm` 3 |
| CI | Linux backend job is manual; 2 of 23 smokes | `.github/workflows/smithay.yml` |
| **Gate** | **0 of 3 measured** | See below |

### Why the gate is still open

These are the concrete blockers, each a task below:

1. The DRM loop polls every 16 ms (`main.rs:231`), capping presentation at 62.5 Hz.
2. `ComposedFrame` carries no damage (`render.rs:205`), any damage marks every output
   dirty (`main.rs:234–235`), and the frontend's damage is then discarded
   (`let _ = app.take_damage();`, `main.rs:646`).
3. There is no frame instrumentation: no present interval, dropped-frame or
   input→damage timestamp.
4. Chrome text is rasterized and uploaded during every `compose()` (`render.rs:508–512`),
   even when unchanged.
5. The palette index query blocks the compositor loop with a 250 ms socket timeout
   (`aether-shell-wayland/src/app.rs:834`, called from the key handler at `:1388`).

## The gate

> Sustained 120 Hz on a 1440p output with the palette open; no frames queued;
> input → damage commit < 2 ms.

| Metric | Target | How it will be captured |
| --- | --- | --- |
| Sustained refresh | 120 Hz, p99 interval < 1.5 × 8.33 ms | Present timestamps per output, 10-minute palette-open run |
| Frames queued | 0 | `DrmCompositor` in-flight slot count per output |
| Input → damage | < 2 ms, p95 and max | Event timestamp → damage set committed |

**Sizes** are relative: **XS** contained patch; **S** one file plus a test; **M** a
subsystem across 2–3 files; **L** new backend-scale work. No calendar estimates —
the repository's velocity makes them noise.

---

## G — Gate workstream

### G1 · Deadline-driven present loop

**Why.** `event_loop.dispatch(Some(Duration::from_millis(16)))` is a fixed poll
timeout. At 120 Hz the target interval is 8.33 ms, so the loop cannot present fast
enough no matter how fast rendering is.

**Touch.** `aether-shell-drm/src/main.rs` (loop, `render_all`, `frame_submitted`),
`aether-pacing` (consumer only), optionally `aether-shell-wayland` if the pacer moves
into shared state.

**Do.**
- Hold one `FramePacer` per output, initialised from the connector's refresh rate.
- Compute the dispatch timeout from the earliest `pacer.deadline()`; pass `None`
  when there is no damage and no scheduled frame, so idle costs no wakeups.
- Gate submission on `pacer.should_present(now_ns)`; call `pacer.on_present(now_ns)`
  after `queue_frame` succeeds, so a late frame re-anchors instead of queueing.
- Keep the existing `--frames N` exit for deterministic tests.

**Done when.** On a 120 Hz-capable output, 60 s of palette interaction reports a p99
present interval < 12.5 ms and zero busy frames while idle; `missed_deadlines` is
reported and non-zero only when frames are actually late.

**Record.** `artifacts/perf/p2/pacing.json` with per-output interval samples.

### G2 · Carry damage through the frame plan

**Why.** The frontend computes damage and then throws it away. Marking all outputs
dirty on any damage means every keystroke redraws the whole 1440p scene — the largest
single threat to the gate.

**Touch.** `aether-shell-wayland/src/render.rs` (`ComposedFrame`, `compose`),
`app.rs` (`take_damage`, `needs_frame`), `aether-shell-drm/src/main.rs`
(`mark_all_dirty`, `render_all`).

**Do.**
- Add `damage: DamageSet<i32, Physical>` to `ComposedFrame`, populated from the
  frontend's per-surface damage, clipped to the output rect.
- Replace `mark_all_dirty()` on `needs_frame()` with a per-output damaged set:
  compose only outputs whose rect intersects damage.
- Clear only the damaged region instead of the full output where the renderer allows.
- Delete the `let _ = app.take_damage()` discard; drain damage exactly once, in the
  presenter, and assert in debug that the drained set is a superset of what was drawn.
- Keep the conservative full-window fallback for `Damage::Buffer` with a non-normal
  transform.

**Done when.** With one client typing, frame logs show partial damage on the active
output only, no full-output clears, and no composition of unaffected outputs; idle
(no clients, no input) presents zero frames.

**Record.** `artifacts/perf/p2/damage.log` from `AETHER_FRAME_LOG`.

### G3 · Frame instrumentation

**Why.** None of the three gate numbers can be produced today.

**Touch.** `aether-shell-wayland/src/app.rs` (input timestamps), `render.rs`
(damage committed), `aether-shell-drm/src/main.rs` (present, flip complete, queue
depth), `aether-pacing` (`PacingReport` serialisation).

**Do.**
- Stamp each input event on arrival; stamp damage committed at the end of the
  handler; stamp present at `queue_frame` and completion at the vblank callback.
- Track in-flight slots per output; expose a `queue_depth` counter.
- Emit newline-delimited JSON when `AETHER_FRAME_STATS=<path>` is set; include
  output id, mode, interval, missed deadlines, queue depth, input→damage, and the
  damage area.
- Summarise p50/p95/p99 with a small script under `scripts/`.

**Done when.** A single run produces the three gate metrics as p50/p95/p99 plus
raw samples, on both the winit host (off-target) and the DRM backend (on-target).

**Record.** `artifacts/perf/p2/frames.ndjson` + generated summary.

### G4 · Output model from the connector

**Why.** The DRM path selects the EDID preferred mode, but `OutputInfo` does not
carry refresh and nothing asserts that the pacer and the advertised Wayland mode
agree. The winit host still hardcodes 1280×800@60.

**Touch.** `aether-shell-wayland/src/app.rs` (`OutputInfo`), `aether-shell-drm/src/main.rs`
(`add_output`), `aether-shell-smithay/src/client.rs` (host output).

**Do.**
- Add `refresh_hz` (and `physical_size_mm`) to `OutputInfo`; derive it from
  `mode.vrefresh()` on DRM and from the winit monitor on the host.
- Assert at startup that every pacer's Hz equals its output's advertised refresh.
- Fail loudly when no mode ≥ 120 Hz exists, so a 60 Hz panel is a visible failure,
  not a silent gate miss.

**Done when.** On the reference monitor the shell advertises 1440p@120 (or the
highest available) in `wl_output` and paces to the same rate; a 60 Hz host run logs
the shortfall explicitly.

### G5 · Reference measurement run

**Why.** The gate is a measurement on the 7700K; nothing else closes P2.

**Touch.** `scripts/target-check.sh`, `apps/docs/docs/reference/claims-and-evidence.md`,
`apps/docs/docs/product/metrics.md`, `artifacts/perf/p2/`.

**Do.**
- Preflight, cold boot, confirm `landlock` in `/sys/kernel/security/lsm`.
- Run the palette-open workload (typed query loop + window switching) for 10 minutes.
- Capture: boot to shell, idle RAM/CPU at 60 s, sustained refresh, p99 interval,
  input→damage, queue depth, and the GPU/connector mode used.
- Store raw samples; update the claims page with measured vs budgeted labels and
  the exact machine, governor and kernel.

**Done when.** All three gate metrics appear as measured, with raw samples committed,
or an ADR waives one with data.

### G6 · Prove nothing is queued

**Why.** "No frames queued" is a separate promise from "fast frames".

**Touch.** `aether-shell-drm/src/main.rs`, `aether-shell-wayland` render path.

**Do.**
- Instrument `NoFreeSlotsError` and in-flight slots per output.
- Assert ≤ 1 frame in flight per output during the 10-minute run.
- Add a stress smoke: force a slow compose (large palette, many windows) and verify
  frames are dropped and superseded rather than queued.

**Done when.** The 10-minute run reports max queue depth 1 and a non-zero dropped
count under injected slowness, with no unbounded latency growth.

---

## C — Compositor cost workstream

### C1 · Cache chrome text

**Why.** `compose()` rebuilds the element list and calls `TextBitmap::render` +
`MemoryRenderBuffer::from_slice` for every label on every frame (`render.rs:508–512`).
With the palette open that is dozens of rasterizations and texture uploads per frame.

**Touch.** `aether-shell-wayland/src/render.rs` (text buffer creation),
`app.rs` (cache storage), `aether-text`.

**Do.**
- Cache text buffers keyed by `(text, scale, colour)`; invalidate on query, workspace
  name or status change.
- Reuse a per-output element list, rebuilding only entries whose inputs changed.
- Keep a debug counter for rasterizations per frame.

**Done when.** Static palette frames perform zero text rasterizations and zero
buffer uploads; the counter is zero across 120 consecutive frames.

### C2 · Avoid full-output clears

**Why.** `frame.clear(...)` with the full damage rect paints 1440p every frame even
for a 3 px damage region.

**Touch.** `render.rs`, `aether-shell-drm` render path.

**Do.** Clear the damaged region only; fall back to a full clear on first frame,
mode change, or when damage covers more than half the output.

**Done when.** Frame logs report the cleared area equal to the damage area for small
updates.

### C3 · Occlusion (deferrable)

**Why.** Overdraw is the remaining predictable cost after C1/C2.

**Do.** Use Smithay's opaque-region subtraction across the z-ordered element list;
measure before and after. Ship only if it moves the p99.

**Done when.** A measured p99 improvement is recorded, or the task is closed with the
numbers that justified not doing it.

### C4 · Renderer decision (ADR, not a rewrite)

**Why.** The roadmap names wgpu; the implementation is GLES/EGL. That is a deviation
without a record, and it is not currently the bottleneck.

**Do.** Write an ADR comparing GLES and wgpu against the measured frame cost; keep
GLES unless the numbers justify the port.

**Done when.** ADR accepted and the roadmap line updated.

---

## L — Latency hygiene

### L1 · Non-blocking palette index query

**Why.** `refresh_index_results()` runs synchronously on the compositor thread for
every palette keystroke, with 250 ms socket timeouts (`app.rs:834`). A slow or hung
index service stalls input and directly violates the *input→damage* gate.

**Touch.** `aether-shell-wayland/src/app.rs` (query path), `apps/indexer-service`
(protocol unchanged), new worker module.

**Do.**
- Move the socket round-trip to a worker thread; return results through a channel
  drained by the compositor loop.
- Tag each request with a query generation; discard responses whose generation is
  stale (the palette engine already models stale queries).
- Coalesce keystrokes with a short debounce; never grow an unbounded queue.
- On timeout or disconnect, fall back to the bounded scan without blocking.

**Done when.** With an indexer that sleeps 200 ms per request, typing at 10 keystrokes/s
keeps input→damage < 2 ms and shows only results for the latest query; a smoke asserts
this with a deliberately slow service.

### L2 · Cap index result churn

**Do.** Limit returned rows to the visible count plus one; reuse `PaletteFile`
allocations; skip re-render when the result set is unchanged.

**Done when.** A repeated identical query causes no recomposition.

---

## V — Verification

### V1 · Put the real backend in CI

**Why.** `smithay.yml` is `workflow_dispatch` only, builds one manifest, and runs 2
of 23 smokes. The compositor's core is the least tested code in the repository.

**Do.**
- Trigger on PRs touching `os/compositor/**`.
- Build and clippy all three manifests (`smithay`, `wayland`, `drm`).
- Run `cargo test` for the two standalone crates.
- Run a curated smoke set on every PR (damage, idle, cursor, palette, fullscreen,
  provider, index, text-input, agent-ipc, clipboard-persist, actions); keep the rest
  nightly/manual and say so in the workflow header.

**Done when.** A PR that breaks `take_damage()` or `compose()` fails CI.

### V2 · Unit tests for the frontend

**Do.** Cover `compose()` per output (visibility, z-order, chrome, cursor layers),
`take_damage()` semantics (full vs partial, drain-once), `add_output`/`remove_output`
geometry, mode/refresh derivation, and stale index-response discard.

**Done when.** `app.rs` and `render.rs` each have targeted tests; the standalone
test count moves from 8 to a level that would catch a damage regression.

### V3 · Headless native-backend smoke

**Why.** Page flips need a DRM device, but most of the host can still be exercised.

**Do.** Add a `--frames N` headless path that runs the DRM host against a virtual
connector (QEMU `virtio-gpu` in the development VM) and asserts frames presented,
queue depth ≤ 1, and clean shutdown. Document what only real hardware can prove.

**Done when.** The VM smoke runs in CI and catches host regressions before hardware.

---

## F — Feature tail (gate-independent)

| ID | Task | Done when |
| --- | --- | --- |
| F1 | Fractional scaling: per-output `Scale`, surface scale, damage conversion for non-integer scale | A 1.5× output renders crisp and damage stays correct |
| F2 | Screenshot/capture protocol (`wlr-screencopy` + DMA-BUF export) | A capture client receives a correct frame with damage metadata |
| F3 | Desktop-entry SVG icons, theme inheritance, DBus actions | Icon themes resolve; actions appear without a manifest |
| F4 | Clipboard DnD completion and type negotiation | Drag between two clients transfers text and files |
| F5 | Status bar surface (agents, CPU, memory, audio, frame time) | Live values, no graphs, no polling when hidden |

---

## D — Documentation and decisions

| ID | Task | Done when |
| --- | --- | --- |
| D1 | ADR: native DRM first, cage as fallback session policy | Accepted; `services.nix` comment links it |
| D2 | ADR: GLES vs wgpu (see C4) | Accepted; roadmap line updated |
| D3 | Rewrite the P2 roadmap bullet | It no longer claims a single 1280×800@60 output or lists landed items as ⏳ |
| D4 | Fix `smithay-backend.md` limitations | The "buffer damage is a full-window redraw" claim is removed |
| D5 | Claims & evidence for P2 | Every P2 number labelled measured, budgeted or speculative, with machine and kernel |

---

## Sequencing

**Batch 1 — unblocked, off-hardware.**
G2 → G1 → G3 → C1 → C2 → L1 → V1. These can all be developed and validated under
Xvfb, and together they make the gate measurable.

**Batch 2 — Linux build, no reference machine.**
V2 → V3 → G6 → F1/F2/F3. V3 is what lets a regression reach the 7700K safely.

**Batch 3 — reference machine.**
G4 → G5. G4 can be written earlier but is only meaningful with the real monitor.

**Continuous.** D1–D5 alongside the batch that touches them.

**Cheapest first move:** G2. It removes the largest per-frame cost, and it is the
prerequisite for both the 120 Hz loop and an honest input→damage number.

## Definition of done for P2

- The three gate metrics are measured on the reference machine and committed as raw
  samples.
- A CI or documented runbook reproduces the pacing and damage numbers.
- `roadmap.md`'s P2 section matches reality, with remaining items explicitly deferred.
- D1 and D2 are accepted; R2 (i915 at 120 Hz) is either closed or formally waived
  with data.
- `bun run verify`, all Rust tests, and the curated smokes pass from a clean checkout.

## Non-goals for this plan

- Rewriting the renderer.
- Removing cage before the native backend has survived real daily use.
- Fractional scaling, occlusion, capture and DnD as gate blockers — they are tail work.
- Any new dependency, daemon or capability that does not close a gate or remove a
  measured cost (see roadmap "How work is chosen").

## Beyond P2

P3–P8 remain as specified in the [roadmap](/roadmap/). The first actionable
P3 tasks, given what now exists:

| Phase | First tasks |
| --- | --- |
| P3 | Promote the Bun index service to `aether-index` over the IPC ring; move the palette into the compositor as a layer; explorer virtualisation at 10k rows; screenshot parity with the reference explorer |
| P4 | PipeWire/WirePlumber policy; device overlay on the shell's chrome primitives; route-switch harness |
| P5 | `aetherd` admission control and capability issuance; per-runtime Landlock/seccomp profiles; red-team suite |
| P6 | WebKitGTK shell with per-tab processes; libmpv frontend; file operations |
| P7 | DMA-BUF capture → VA-API → str0m → WebCodecs (post-v1.0) |
| P8 | PGO with recorded workloads; Firecracker tier; threat-model review |
