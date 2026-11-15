---
id: memory-footprint
title: Memory footprint and idle cost
sidebar_position: 16
description: A code-level audit of what the shell and its daemons keep resident at idle, what grows during use, and how to measure it.
---

# Memory footprint and idle cost

Aether's product promise is a **≤ 200 MB idle footprint, < 0.5% idle CPU**
([metrics](/product/metrics)). The repository has no measurement for either, and
the 200 MB number does not yet say *what* it measures. This page closes the first
half of that gap: it audits the code for what is resident at idle, separates
unavoidable buffers from caches that never shrink, and defines a reproducible
harness. [Memory scenarios](/roadmap/memory-scenarios) covers what to implement.

**Labels used here**

| Label | Meaning |
| --- | --- |
| **Measured** | Produced on this development host by a committed command |
| **Modelled** | Arithmetic from the code (sizes, rectangle areas, per-entry bytes) |
| **Unmeasured** | Depends on the target machine or on process versions; must be sampled |

The target is still unmeasured. Every number about the 7700K session below is a
**model**, not a result, and must be replaced by `scripts/memory-report.sh` output
before it appears in the ledger, the README or the site.

Line references were checked at the audit revision; symbol names are the durable
pointer when the code moves.

## Summary

- **Idle time does not create memory; it keeps what use created.** The compositor
  blocks correctly with zero frames, but nothing evicts the labels and panels
  rendered before the user stopped.
- **Two caches were unbounded.** Every unique text label and every geometry-keyed
  chrome panel used to be retained forever, as a CPU bitmap and a GPU texture.
  They are now byte-bounded LRUs (F1, F2).
- **The status bar was the multiplier.** While anything animated, it sampled once
  per second and produced a new label, a new pill and new cache entries — with no
  keyboard input (F1). Bounding the caches makes that history collectable.
- **The largest GPU item is invisible to PSS.** Two 1080p scan-out buffers are
  ~16 MiB of kernel GEM that `smem` does not see (F4).
- **The index daemon is wired** to the shell: the session exports its socket and
  the shell queries it. Its query path no longer case-folds every title per
  comparison, and the worker no longer wakes five times a second (F5, F8).
- **There was no gate.** `scripts/memory-report.sh` now measures attributed PSS +
  resident DRM, machine use, and idle growth.

## The four things "idle RAM" can mean

| Surface | How it is measured | What it includes | Weakness |
| --- | --- | --- | --- |
| Machine used | `MemTotal - MemAvailable` (`/proc/meminfo`) | kernel, slab, all services, page cache not counted as available | includes code we do not control; noisy across kernels |
| Process RSS | `VmRSS` in `/proc/[pid]/status` | shared pages counted once per process | double counts shared libraries |
| Process PSS | `Pss` in `/proc/[pid]/smaps_rollup` | shared pages split by sharing | still misses kernel-owned GPU objects |
| GPU / GEM | `drm-resident-*` in `/proc/[pid]/fdinfo/*` (i915) | scan-out and texture buffers owned by the process's DRM file | per-file accounting; shared objects can appear more than once |

`smem` (the tool the ledger used to name) reports PSS. It **does not see i915 GEM
objects**: compositor scan-out buffers and texture uploads are kernel shmem, not
pages mapped into the process. A 1920×1080 session therefore has ~16 MiB of
resident GPU memory that a PSS-only measurement calls free. The harness in this
commit reports both PSS and `fdinfo` DRM, and the gate can use either surface.

Recommended definition, used by the harness:

- **Attributed footprint** = Σ PSS + Σ resident DRM of the session processes
  (`aether-shell*`, `aether-index`, PipeWire/WirePlumber, IME, user session).
  This is the number the project can design against; target ≤ 200 MiB.
- **Machine footprint** = `MemTotal - MemAvailable`. This is the literal
  "whole machine at idle" number and includes the kernel and system services. It
  is a useful sanity check and a poor engineering budget.

The two are reported side by side until the ledger picks one.

## Measured anchors

| Anchor | Value | Label | Command |
| --- | --- | --- | --- |
| Software shell core + one 1920×1200 frame | 11.4 MiB peak RSS | **Measured (dev host)** | `cd os/compositor && cargo build --release -p aether-preview && /usr/bin/time -l ./target/release/aether-shell-preview` (macOS; GNU `time -v` on Linux) |
| That frame's RGBA canvas | 8.8 MiB | Modelled | 1920 × 1200 × 4 B |
| Shell core without the canvas | ~2.6 MiB | Derived | Peak RSS minus canvas |
| Wayland/Smithay shell binary | 158 MiB debug ELF | Measured (dev host) | file size, not resident; included only to reject it as a proxy |
| Session PSS / DRM / idle growth on the 7700K | not recorded | **Unmeasured** | `scripts/memory-report.sh` |

The anchor matters because it rules out the cheap explanation: the shell's own
state (scene graph, window list, palette model) is **not** what threatens the
budget. Buffers and caches are.

## Where idle memory actually goes

At idle the DRM loop blocks in `calloop` with no timeout
(`os/compositor/aether-shell-drm/src/main.rs:676`), the index daemon blocks in
`inotify`, and the compositor composes nothing. The resident set is therefore
the union of:

| Component | Mechanism | Expected at idle | Label |
| --- | --- | --- | --- |
| Shell binary + heap | Rust, font, cursor theme, Wayland state, scene | ~10–25 MiB | Unmeasured |
| Scan-out swapchain | 2 buffers per output × mode × 4 B | 15.8 MiB at FHD; 28.1 MiB at QHD | Modelled |
| Background bitmap | one full-output RGBA per active output size, replaced on style change | +7.9 MiB per output (CPU) + 7.9 MiB (GPU) | Modelled |
| Text / chrome caches | byte-bounded LRUs, see F1/F2 | 0 MiB on a fresh boot; ≤ ~20 MiB CPU after use | Modelled |
| `aether-index` | 20k filename documents, inotify watches, 1 MiB ring file | ~12–18 MiB, now serving the palette (F5) | Modelled |
| Shell file list | 2,000-file synchronous scan when the socket is not set | ~0.6–1 MiB | Modelled |
| PipeWire + WirePlumber | realtime graph, device objects | ~15–30 MiB | Unmeasured |
| IME (fcitx5) | enabled by default in `configuration.nix` | ~25–45 MiB with addons | Unmeasured |
| systemd, udev, D-Bus, NetworkManager, bluetoothd | base OS | ~60–110 MiB | Unmeasured |
| journald runtime journal | tmpfs, `RuntimeMaxUse=128M` | cap 128 MiB; actual size unmeasured | Unmeasured |
| Kernel, slab, driver state | kernel | ~50–100 MiB | Unmeasured |
| **Session sum** (shell + index + audio + IME) | | **~60–120 MiB before caches** | Modelled |

Two consequences follow.

1. **The shell has room, but not much.** A 30–50 MiB shell budget
   ([compositor](/architecture/compositor#performance-budget)) plus index and audio
   is plausible. The IME and the base OS are not free.
2. **The 200 MB budget only works as a session/attributed number.** Whole-machine
   `MemTotal - MemAvailable` on a minimal NixOS with journald, NetworkManager and
   PipeWire is unlikely to sit under 200 MiB before the shell starts. Either the
   ledger says "attributed", or the number changes. That is a product decision,
   not an optimisation.

## Findings

> **Status.** F1–F3 are addressed by the cache-budget change: `BudgetCache`
> caps text at 8 MiB and icons/panels/rounds/tops at 1 + 6 + 2 + 1 MiB of CPU
> bytes (GPU textures mirror them), and keeps one background per output size
> under a 48 MiB ceiling. The findings below
> record the unbounded state that motivated the budgets; they remain the review
> checklist for new caches. S1's pinned set and idle trim are not implemented.

### F1 — The text cache was unbounded and grew with every changing label

`App.text_cache` is a `HashMap<(String, u32, [u8; 4], bool), MemoryRenderBuffer>`
(`os/compositor/aether-shell-wayland/src/app/mod.rs`, field `App::text_cache`).
Labels were inserted on miss and **never removed or evicted**
(`os/compositor/aether-shell-wayland/src/render/compose.rs`). Each value owns the
CPU RGBA bitmap *and* a cached GPU texture in Smithay's
`MemoryRenderBufferInner` (`textures: HashMap<ErasedContextId, …>`), so one entry
costs roughly `width × height × 4` bytes twice. The cache is now an 8 MiB
`BudgetCache`: the live overlay set stays resident, history is evicted.

The cache is keyed by the exact rendered string, so every value that changes
creates a new entry; the budget decides how long it survives:

| Surface | What changes | Labels per change |
| --- | --- | --- |
| Status bar (`status.rs:74`) | CPU %, used MB, frame time, audio volume | 1 label + 1 pill panel, **once per second of frame activity** |
| Palette (`palette.rs:52`) | query echo, titles, details | up to ~22 per keystroke |
| Explorer (`explorer.rs`) | file names and paths while scrolling | ~2 per visible row per scroll step |
| Chat (`chat.rs`) | every streamed token line | 1–2 per line |
| Task manager (`task_manager.rs`) | process names, commands, CPU/mem columns | ~40 per one-second refresh |

The status bar is the worst case because it is always visible and its text contains
`frame_ms` and CPU/memory counters. While *something else* animates — video, a
streaming chat, a client repainting — a frame is due every second, `poll_status`
samples (`app/status.rs:54`), the label changes, and a new bitmap plus a new
status-pill panel (keyed by width) are created. **No user input is required.**
They are now evictable like any other entry.

### F2 — Chrome caches were unbounded and keyed by geometry

`chrome_icons`, `chrome_panels`, `chrome_rounds` and `chrome_tops` were plain
`HashMap`s with no eviction
(`os/compositor/aether-shell-wayland/src/render/chrome.rs`). Keys include width,
height, radius, fill and border colour. They are now byte-bounded per class
(1/6/2/1 MiB).

- Chat bubbles are cached by their content-derived height, so every distinct
  message length is a new panel bitmap.
- The status pill width follows the status string, so F1 also creates a new panel
  per status update.
- Theme changes alter every colour, so a full palette of panels is regenerated per
  theme.

### F3 — The background cache stored a full-output bitmap per style

`chrome_background` used to key on `(width, height, BackgroundStyle)`
(`app/mod.rs`, field `App::chrome_background`). `BackgroundStyle` is the
cross-product of mode (4), background (2), tone (3), caustics (2), effects (2) and
speed (3) — up to 288 distinct styles (`render/chrome/primitives.rs:12`). A user
cycling themes could accumulate dozens of 7.9 MiB CPU buffers plus their GPU
twins. The cache is now keyed by output size and replaces the buffer when the
style changes, so one style per active output is resident.

### F4 — GPU buffers are invisible to the current measurement story

Smithay's `Swapchain` allocates up to four buffers per output on demand and reuses
them. With queue depth 1 the steady state is two: 15.8 MiB at 1920×1080, 28.1 MiB
at 2560×1440, in kernel GEM. `smem`/PSS does not see them. A PSS-only report can
show a "pass" while the machine is 16–60 MiB heavier. The harness adds
`drm-resident-*` from `fdinfo` to close this gap.

### F5 — The index is compact, but it re-allocated on every query

A `Document` is `{ PathBuf, title, subtitle, lower_title, haystack }`
(`os/index/aether-index/src/index.rs`). With allocator headers that is roughly
400–500 B per document at 20k entries, i.e. ~8–10 MiB of documents, plus inotify
watch paths and a 1 MiB ring file. `lower_title` is the one addition made by this
audit: ~1 MiB at 20k documents.

- **Wiring.** `aether-session` exports
  `AETHER_INDEX_SOCKET="$XDG_RUNTIME_DIR/aether-index.sock"` (`flake.nix`), so
  `App::new` spawns `IndexWorker` and the palette queries the daemon instead of
  the bounded 2,000-file fallback. The fallback remains for a session with no
  service configured. The earlier audit revision found the socket was not
  exported; that is fixed.
- **Query allocations (fixed).** `score()` used to lowercase the title for
  **every** document on **every** query, so a keystroke allocated ~20k short
  strings; the sort comparator lowercased both titles per comparison, ~n·log₂n
  more. Titles are now case-folded once at insert, and the palette selects the
  top-k with `select_nth_unstable_by` before sorting only the page. This trades
  ~1 MiB of retained lowercase titles for the removal of the per-keystroke churn
  that inflated RSS high-water and glibc arenas.

The daemon still accepts a **new connection and a new thread per query**
(`main.rs`, and the shell connects per request in `index_worker.rs`). Thread
stacks are virtual and small resident, but per-thread allocator arenas are
retained by glibc. It is survivable for one user; a persistent connection is the
next step if queries become hotter. Nor is this the memory profile of the BM25
index the documentation describes, which would be several times larger than this
filename-only table. Fix the representation before that feature lands (S4).

### F6 — The session process floor includes an IME nobody measured

`i18n.inputMethod.enable = true; type = "fcitx5";`
(`os/nixos/configuration.nix:62`) and `p11-kit`/D-Bus activation make fcitx5 part
of the session. If it autostarts, it is the largest optional process in the
session. There is no measurement of it and no opt-out path documented. PipeWire
and WirePlumber are justified; a default IME is a UX assumption that deserves a
number.

### F7 — There is no idle-memory gate, and the quoted tool is not installed

- The ledger used to name `smem`, but `smem` was never installed
  (`configuration.nix:68`); the first-boot checklist substitutes `free -m`. The
  new harness is self-contained, but nothing yet runs it on the target.
- Nothing measures growth over time, so an unbounded cache passes every existing
  test.
- `AETHER_TEXT_STATS` proves a static frame rasterizes each unique label once
  (`reference/claims-and-evidence.md:53`), which is correct and orthogonal: the
  problem is that *unique labels never stop accumulating*.

### F8 — The index worker used to wake five times a second

The shell's index worker waited on a condvar with a 200 ms timeout in a loop
(`index_worker.rs`). The compositor genuinely blocked with no timer, but this
thread woke 5×/s for the life of the session once the socket was wired. It now
blocks until a request arrives or the worker is dropped, so idle wakeups are zero
in every session. The 30 ms debounce after a request is active-work only.

### F9 — The status bar forks `wpctl` once per sampled frame

`StatusSnapshot::sample` calls `wpctl get-volume` (`status.rs:76`) on every status
sample. During sustained activity this is a fork/exec + libpipewire initialisation
per second — transient memory churn and a latency spike in the frame that happens
to sample it. Not an idle-leak, but it is measurable and avoidable (cache the
volume, update on PipeWire events, or read the device object directly).

## Growth arithmetic

Use these as order-of-magnitude checks, not as predictions. Assume Inter at the
sizes in `aether-text/src/font.rs` (11 px small, 13 px body, ~16 px line height)
and a bitmap of `chars × 6–7 px`.

| Label | Characters | Pixels | CPU bytes | With GPU texture |
| --- | --- | --- | --- | --- |
| Status line | 50 | 300 × 14 | 16.4 KiB | 32.8 KiB |
| Status pill | 340 × 26 | — | 34.5 KiB | 69.0 KiB |
| Palette title | 30 | 195 × 16 | 12.2 KiB | 24.4 KiB |
| Palette detail | 50 | 275 × 14 | 15.0 KiB | 30.0 KiB |
| Explorer row (name + path) | 25 + 60 | 162 × 16 + 330 × 14 | 28.1 KiB | 56.2 KiB |
| Chat line + bubble | 70 | 455 × 16 + 470 × 40 | 101.8 KiB | 203.6 KiB |

Then:

- **One hour of windowed animated content with the status bar visible** ≈
  3,600 status updates × ~100 KiB ≈ **350 MiB**, before pill-panel variants are
  counted. Fullscreen hides the bar and stops sampling; windowed video or a
  streaming chat does not.
  This is the difference between "idle RAM is fine" and a session that grows by
  hundreds of megabytes while the user watches a video.
- **One palette search** of eight keystrokes, each changing the result set ≈ 8 ×
  ~330 KiB ≈ **2.6 MiB**.
- **Scrolling 5,000 explorer rows** in 100 viewport steps ≈ 100 × ~1.1 MiB ≈
  **110 MiB**.
- **A task-manager session left open for ten minutes** ≈ 600 refreshes ×
  ≥0.6 MiB ≈ **≥360 MiB** (long command lines push this higher).
- **Index steady state** at 20k docs ≈ **8–10 MiB** of documents; the cached
  lowercase titles remove the per-query allocation churn rather than adding
  retention.

These are the numbers that explain why a shell can pass a 60-second idle check and
still be over budget an hour later. The budget must be checked **after use**, not
only after boot.

## Idle behaviour, precisely

What happens when nothing is happening:

- DRM backend: `poll_timeout()` returns `None` when there is no damage and no task
  manager deadline; `calloop` blocks; no frames, no CPU
  (`os/compositor/aether-shell-drm/src/main.rs:676-703`).
- Status bar: sampled **only inside `compose`**, i.e. only when a frame was due
  for another reason (`app/status.rs:54-63`). At true idle it does not run.
- Task manager: samples at most 1 Hz and only while open (`task_manager.rs:96`).
- Index daemon: blocking `inotify` read; no scans, no polling.
- Shell index worker: blocks on a condvar until a request; no timed wakeups (F8).
- Audio: no periodic sampling in `aether-audio`; status calls `wpctl` when a
  status sample runs.

So **idle time does not create memory**; it preserves whatever usage created.
That is why "idle footprint" must be defined as *after a representative session*.
A fresh-boot measurement is the easiest number to pass and the least useful.

## How to measure

The committed harness is `scripts/memory-report.sh` (Linux, no dependencies):

```bash
# 60 seconds of no input, human-readable
./scripts/memory-report.sh

# Gate a release candidate: 10 minutes idle, attributed budget
./scripts/memory-report.sh --seconds 600 --gate attributed --check

# Keep raw evidence with the run
./scripts/memory-report.sh --gate attributed \
  --json artifacts/perf/memory/idle-$(date +%Y%m%dT%H%M%S).json
```

Protocol:

1. Boot, sign in, wait 60 s so the session settles.
2. Run the harness **before** opening apps: this is the boot baseline.
3. Do a representative ten minutes: type in the palette, scroll the explorer,
   open chat, open the task manager, play a video, switch themes.
4. Close everything and run the harness again. The second run must be close to
   the first; the delta is the retention cost of the session.
5. Record the machine footprint at the same time; they answer different questions.
6. Store the JSON. Summaries are derived.

The harness also reports `Shmem`, `SReclaimable` and the scan-out/DRM resident
bytes, because a PSS-only report would let F4 pass unnoticed.

`AETHER_TEXT_STATS=1` now prints the text cache bytes, entries, evictions and the
per-class chrome bytes next to the rasterization counter. Compare those with the
harness: a flat process footprint and a flat cache byte count after the workload
is the pass condition.

## What is already fine

- **Chat history is bounded** at 4,000 lines, trimmed to 3,000
  (`aether-shell/src/model/chat.rs`, `chat_materialize`).
- **The scene graph is tiny**: a few hundred bytes per surface, no per-pixel
  storage (`aether-scene`).
- **The explorer is virtualised**: `matches: Vec<usize>` for at most 5,000 files is
  ~40 KiB; only the visible slice becomes labels (`model/explorer.rs`).
- **The index has a hard document limit** (`AETHER_INDEX_LIMIT=20000`), ignores
  vendor directories and is wired to the palette. Its steady state is small.
- **The shell's fallback file list is bounded** to 2,000 entries at depth 5, and
  it is a one-time synchronous scan at startup, not a watcher.
- **`/var/index` tmpfs is a cap, not a reservation** (tmpfs is lazy); it is also
  currently unused, because the ring lives in `XDG_RUNTIME_DIR`. It does not cost
  2 GiB at idle, but it is misleading documentation.
- **Idle CPU in the compositor is genuinely zero**; the gap is the index worker's
  timed wait and the status fork, both listed above.

## Open questions that need the target

1. Which processes are actually in the session at 60 s? (`systemctl --user
   list-units`, `ps -eo pid,user,rss,comm --sort=-rss`)
2. Does fcitx5 autostart, and with which addons?
3. How many scan-out buffers does i915 keep resident at 1080p and 1440p?
4. What is the whole-machine footprint after boot on 6.6+ with this kernel
   fragment, and how much of it is the journald runtime journal?
5. How large is the text cache after one hour of the representative workload?
   The harness reports growth; a debug counter for cache bytes would make it
   direct.

The next page turns these into implementation scenarios with effort, impact and
gates: [Memory scenarios](/roadmap/memory-scenarios).
