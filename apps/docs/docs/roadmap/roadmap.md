---
id: roadmap
title: Roadmap
sidebar_position: 1
description: Phases P0–P8 with deliverables, exit gates, dependencies and design-source guidance.
---

# Roadmap

Nine phases. Each has a **gate** — a measurement or behavioural proof — and a list of
deliverables. A phase is done when the gate passes, not when the code merges.

```text
P0 Foundations      ████████████  done
P1 NixOS bring-up   ██████████░░  in progress
P2 Compositor MVP   ███████████░  in progress
P3 Shell surfaces   ██████████░░  gate pending
P4 Audio            ██████████░░  code-complete, gate pending
P5 Agents & chat    ░░░░░░░░░░░░
P6 Applications     ░░░░░░░░░░░░
P7 Remote display   ██████████░░  code-complete, gate pending
P8 Hardening        ░░░░░░░░░░░░
```

## P0 — Foundations · done

**Goal.** A repository that contains the product surfaces, the tested core and the OS
configuration, with CI and documentation.

Deliverables:

- Monorepo: Bun workspaces, strict TypeScript, Rust workspace.
- `protocol`, `indexer`, `ipc`, `agents`, `palette`, `ui`, `bench`.
- Water design system vendored from the Data Cloud Compare project.
- Docusaurus docs and a marketing site with a live palette and explorer demo.
- Unit, DOM, cross-thread integration, Playwright e2e, Rust tests, perf gate.
- NixOS flake, hardware/kernel/filesystem modules, installer ISO, disko layout and install guide.

**Gate.** `bun run verify`, Playwright and `cargo test` green in CI; benchmark
baseline committed; docs build with no broken links.

## P1 — NixOS bring-up · in progress

**Goal.** Boot the reference machine into a deterministic base with the custom kernel.

Deliverables:

- ✅ Pin nixpkgs and the Rust toolchain; flake exposes `nixosConfigurations.kestrel`
  and `nixosConfigurations.installer`.
- ✅ Kernel fragment: Kaby Lake options, Landlock, seccomp, VFIO, 1 kHz tick.
- ✅ XFS root, ext4 scratch, tmpfs index; mount options from the spec.
- ✅ disko GPT layout, live installer ISO, `scripts/install.sh` and an end-to-end
  [install guide](/getting-started/install).
- ✅ Full Aether live ISO output (`.#live-iso`, x86_64): compositor, shell and audio
  from USB with no persistence; the minimal installer remains `.#iso`.
- ✅ Flake evaluation (`nix flake check --no-build`) passes for the installed system
  and the installer on x86_64-linux.
- ⏳ Boot the installed system on the reference machine and record the numbers.
- ⏳ Trim the kernel with `localmodconfig` on that machine and commit the delta.
- ✅ Console autologin on tty1; `aether-session` on tty7 starts the native DRM
  compositor with cage as the fallback ([ADR-0017](/adr/0017-native-drm-first)).
- ✅ A separate aarch64 development VM boots the same base under HVF in ~10 s
  (test harness only; not the reference target).

**Gate.** Cold boot to a shell in < 1.5 s userspace (three consecutive runs), idle
RAM and CPU recorded, `landlock` present in `/sys/kernel/security/lsm`.

**Status notes.** Evaluation is verified; an actual boot has not been performed in
this repository’s CI (no x86_64 builder). Building the ISO requires an x86_64 Linux
host with Nix; the install guide’s Option A uses the official NixOS ISO and does
not require building one.

**Dependencies.** None. **Risk:** missing firmware, storage driver, thermal throttling.

## P2 — Compositor MVP · in progress

**Goal.** A Smithay shell that owns the scene graph, tiling and frame pacing.

Deliverables:

- ✅ winit/EGL adapter presents `FramePlan` (solid surfaces) under Xvfb; standalone
  Linux crate so the core stays toolkit-free.
- ✅ xdg-shell/shm clients map into shell windows, tile and are configured to
  their rects; keyboard/pointer focus, buttons and axis; two terminals verified.
- ✅ Workspace shortcuts (`Super+1/2/3`, `Super+Alt+1/2/3` per output),
  window moves, close, labelled indicators, a clickable workspace picker, an
  interactive command palette (typed query, filtering, workspace/window
  providers, row clicks, actions), equal-column tiling, Alt+drag move,
  Alt+right-drag corner resize, edge snapping, half-snap on release,
  `Super+M` maximize with restore, hover cursor feedback, a `New terminal`
  command, XDG `.desktop` app discovery and a bounded filename index with
  `OpenFile`. Clipboard and primary-selection copy/paste work between clients
  via focus forwarding. Picker, palette input/commands, desktop/file
  launch, clipboard, window management, DnD and cursor mapping are verified by
  xdotool smokes. Idle frames are skipped; client commits use surface/buffer
  damage when available.
- ✅ Native `aether-shell-drm`: libseat session, KMS page-flips, libinput,
  udev hotplug, per-output workspaces and multi-output composition, with cage as
  the fallback session ([ADR-0017](/adr/0017-native-drm-first)). Validated on a
  virtio-gpu VM.
- ✅ Live agent-session IPC, native inotify indexing, clipboard persistence,
  client-to-client DnD, IME candidate popups, client cursor surfaces and
  per-output workspace indicators/palette.
- ✅ Renderer is GLES/EGL (wgpu deferred, [ADR-0016](/adr/0016-gles-vs-wgpu)).
- ⏳ Fractional scaling, screenshot/capture protocol, per-output status data
  (one shared sample today), and the measured 120 Hz gate metrics on the
  reference machine (see [P2 completion](/roadmap/p2-completion)).

**Gate.** Sustained 120 Hz on a 1440p output with the palette open; no frames queued;
input → damage commit < 2 ms.

**Status.** Code-complete except the deferred tail; the gate is unmeasured (0 of 3).
Off-target evidence exists for input → damage (dev host, roughly two orders of
magnitude under the 2 ms budget) and queue depth (virtio-gpu VM); sustained 120 Hz
needs the reference machine. Runbook and ledger:
[P2 completion](/roadmap/p2-completion).

**Dependencies.** P1. **Risk:** GPU driver quirks, text rendering performance.

## P3 — Shell surfaces · ✅

**Goal.** Palette, chat and explorer become compositor layers.

Deliverables:

- ✅ `aether-index` daemon with inotify ingestion, socket queries and an event ring.
- ✅ Palette layer: TS/Rust parity fixture, prefix modes, subsequence scoring,
  10k p95 1.11 ms.
- ✅ Chat drawer streaming markdown; explorer virtualised at 10k rows
  (scroll p95 0.004 ms).
- ✅ Design-system parity: shared token constants, `test:design`, screenshot
  pixel probes.
- ✅ Central keyboard map, MRU focus stack and status bar.

**Gate.** Palette p95 < 10 ms at 10k documents; explorer scrolls 10k rows at 120 Hz;
no dropped frames while streaming tokens.

Dev-host measurements: palette 1.11 ms p95 at 10k, explorer scroll 0.004 ms p95,
chat 52 presented frames in 3 s. The 120 Hz and target-machine halves remain
budgets until the reference run.

**Dependencies.** P2. **Risk:** text shaping, IME, clipboard integration.

## P4 — Audio · in progress

**Goal.** Every input and output route is first-class.

Deliverables:

- ✅ PipeWire + WirePlumber policy; analog, USB, Bluetooth with LDAC/aptX/AAC preferences.
- ✅ Device overlay, scriptable per-app routing (`aether-audio apps` / `move-app`),
  input-meter surface with fixture feed (live peaks wire with P5).
- ✅ Latency presets (1024/256/128) and a 64-frame xrun harness.
- ✅ `audio.capture` capability gate for microphone access, audited.

**Gate.** Route switch < 100 ms; zero xruns in a 10-minute 64-frame stress test;
agent mic access denied without a capability.

Execution plan and commit ledger: [P4 completion](/roadmap/p4-completion). The
runtime/overlay work is code-complete; the route-latency and xrun gates remain
target measurements and are labelled as budgets until run on the 7700K. The
fixture smoke proves switching, presets and two-way overlay state; the capability
suite proves microphone denial. Live input peaks are a P5 supervisor wiring item.

**Dependencies.** P2 (overlay primitives). **Risk:** Bluetooth codec negotiation.

## P5 — Agents & chat

**Goal.** DeepSeek chat and isolated workers under one supervisor.

Deliverables:

- ✅ `aether-sandbox`: kernel-enforced process tier (Landlock, seccomp, UID,
  rlimits, optional cgroup v2), packaged and tested on Linux.
- ⏳ `aetherd`: supervisor wiring, planner integration, admission control,
  capability issuance and audit.
- Landlock + seccomp profiles per language runtime.
- Chat UI with tool-call traces and cancellation.
- Red-team suite: six agents attempting cross-reads and escapes.

**Gate.** Six concurrent agents; zero cross-agent reads in the suite; audit chain
verifies after a forced kill.

The sandbox runtime is landed; the supervisor and red-team suite still gate P5.

**Dependencies.** P3, P4 (for audio-capable agents). **Risk:** seccomp profiles
breaking legitimate workloads; model latency dominating UX.

## P6 — Applications

**Goal.** Browser, media and file management usable daily.

Deliverables:

- WebKitGTK shell with per-tab processes and the permission model.
- libmpv frontend with hardware decode and zero-copy handoff.
- Explorer operations: copy, move, rename, delete, reveal.
- Crash isolation and recovery surfaces.

**Gate.** 4K60 video < 6% CPU; a tab crash leaves the shell at 120 Hz; `~/.ssh` is
unreadable from a `file://` page.

**Dependencies.** P2, P3. **Risk:** WebKit integration, GPU decode on HD 630.

## P7 — Remote display · in progress

**Goal.** The desktop in a browser tab at interactive latency.

Deliverables:

- ✅ DMA-BUF capture model, zero-copy audit and a real low-latency H.264 encoder
  with a DMA-BUF → VA-API/NVENC → software ladder.
- ✅ str0m WebRTC transport, DTLS fingerprints, one-shot pairing codes, session
  tokens and an audited signaling exchange.
- ✅ Virtual-seat input over a data channel: scancodes only, coalesced motion and
  a rate limit.
- ✅ `@aether/remote` browser client: WebCodecs decode, canvas sink, tier
  selection and a WebSocket/JPEG fallback packet model.
- ✅ Compositor remote overlay (`Super+R`), recording indicator and status chip.
- ✅ Opt-in NixOS service with local-only listeners and VA-API packages (Linux
  evaluation pending; no Nix on the development host).
- ⏳ Bind capture to the compositor's exported DMA-BUFs and run the glass-to-glass
  measurement on the 7700K.
- ⏳ AF_XDP only if the measurement shows the kernel stack dominates.

**Gate.** Glass-to-glass < 40 ms LAN; no CPU pixel copies; p99 frame interval
< 1.5× target.

**Status.** Code-complete except the target capture binding and measurement. The
transport, encoder, client and fallback model are tested on the dev host; the
gate numbers remain **budgets**. Execution plan and ledger:
[P7 completion](/roadmap/p7-completion).

**Dependencies.** P2, P6 (for real workloads). **Risk:** encoder/driver matrix,
WebCodecs support differences.

## P8 — Hardening

**Goal.** Compiler and isolation work that only pays once the system exists.

Deliverables:

- PGO pipeline with recorded workloads; ThinLTO everywhere.
- Firecracker + VFIO microVM tier; snapshot/restore.
- Threat-model review, capability fuzzing, kernel attack-surface audit.
- Accessibility gaps assessed; AT-SPI prototype if viable.

**Gate.** 10–25% throughput gain from PGO on recorded workloads, or the change is
reverted; microVM escape corpus passes; audit of residual risks signed off.

**Dependencies.** P5, P7. **Risk:** diminishing returns, complexity budget.

## Cross-cutting work

These are not phases; they run continuously:

| Track | Cadence |
| --- | --- |
| Benchmark baseline refresh | Every release |
| Documentation accuracy | Every PR that changes behaviour |
| Design-system parity | Every UI change |
| Security review | Every new capability or syscall |
| Dependency updates | Monthly, pinned and tested |

## How work is chosen

1. Does it unblock a gate? Do it.
2. Does it remove measured cost? Do it.
3. Does it add a process, dependency or capability without a gate? Write an ADR
   first.
4. Everything else waits.
