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
P1 NixOS bring-up   ██████░░░░░░  in progress
P2 Compositor MVP   ███░░░░░░░░░  in progress
P3 Shell surfaces   ░░░░░░░░░░░░
P4 Audio            ░░░░░░░░░░░░
P5 Agents & chat    ░░░░░░░░░░░░
P6 Applications     ░░░░░░░░░░░░
P7 Remote display   ░░░░░░░░░░░░
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
- ✅ Flake evaluation (`nix flake check --no-build`) passes for the installed system
  and the installer on x86_64-linux.
- ⏳ Boot the installed system on the reference machine and record the numbers.
- ⏳ Trim the kernel with `localmodconfig` on that machine and commit the delta.
- ✅ Console session on tty1 (autologin). The compositor starts a surface at P2.
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
- ✅ Workspace shortcuts (`Super+1/2/3`), window moves, close, labelled
  indicators, a clickable workspace picker, an interactive command palette
  (typed query, filtering, workspace/window providers, row clicks, actions), a
  draggable master/stack split, Alt+drag move, Alt+right-drag corner resize,
  edge snapping, half-snap on release, `Super+M` maximize with restore, hover
  cursor feedback, a `New terminal` command, XDG `.desktop` app discovery and a
  bounded filename index with `OpenFile`. Clipboard and primary-selection
  copy/paste work between clients via focus forwarding. Picker, divider, palette
  input/commands, desktop/file launch, clipboard, window management and cursor
  mapping are verified by xdotool smokes. Idle frames are skipped; client
  commits use surface/buffer damage when available and fall back to the window
  rect.
  Palette app/file/external-launcher providers refresh on open; agent sessions
  can be published through the external manifest. Fullscreen uses xdg
  fullscreen state and hides shell chrome. A single 1280x800@60 output is
  advertised with xdg-output geometry. `zwp_text_input_manager_v3` is advertised
  with focus following the keyboard. `aether-cursor` draws backend-neutral
  software cursors in-frame. A Bun index service supplies content-ranked file
  results over a Unix socket, watches roots for live updates, falls back to the
  bounded scan when absent, and is packaged as a NixOS user service.
- ✅ Target preflight (`scripts/target-check.sh`) checks UEFI, i915, DRM/KMS,
  connectors, input and memory with fixture coverage.
- ⏳ Live agent-session IPC, native inotify indexing, desktop-entry SVG
  icons/theme inheritance/DBus, clipboard persistence/DnD, IME
  engine/input-method service, client cursor surfaces,
  udev/DRM.
- ⏳ udev/DRM backend, libinput seat, wgpu renderer, output hotplug.
- Scene graph with damage tracking; shell chrome in one draw pass.
- Tiling layout, focus, keybindings; one workspace per output.
- `os/compositor` pacing core wired to the 120 Hz deadline.
- Cursor, fractional scaling, screenshot protocol.

**Gate.** Sustained 120 Hz on a 1440p output with the palette open; no frames queued;
input → damage commit < 2 ms.

**Dependencies.** P1. **Risk:** GPU driver quirks, text rendering performance.

## P3 — Shell surfaces

**Goal.** Palette, chat and explorer become compositor layers.

Deliverables:

- `aether-index` daemon with inotify ingestion over the ring.
- Palette layer using `@aether/palette` logic (ported or shared via FFI).
- Chat drawer with streaming markdown; explorer as a virtualised list.
- Design-system parity with the reference explorer; screenshot comparison.
- Keyboard map, focus stack, status bar.

**Gate.** Palette p95 < 10 ms at 10k documents; explorer scrolls 10k rows at 120 Hz;
no dropped frames while streaming tokens.

**Dependencies.** P2. **Risk:** text shaping, IME, clipboard integration.

## P4 — Audio

**Goal.** Every input and output route is first-class.

Deliverables:

- PipeWire + WirePlumber policy; analog, USB, Bluetooth.
- Device overlay, per-app routing, input meters.
- Latency presets and xrun instrumentation.
- Capability check for microphone access.

**Gate.** Route switch < 100 ms; zero xruns in a 10-minute 64-frame stress test;
agent mic access denied without a capability.

**Dependencies.** P2 (overlay primitives). **Risk:** Bluetooth codec negotiation.

## P5 — Agents & chat

**Goal.** DeepSeek chat and isolated workers under one supervisor.

Deliverables:

- `aetherd`: planner integration, admission control, capability issuance, audit.
- Landlock + seccomp profiles per language runtime.
- Chat UI with tool-call traces and cancellation.
- Red-team suite: six agents attempting cross-reads and escapes.

**Gate.** Six concurrent agents; zero cross-agent reads in the suite; audit chain
verifies after a forced kill.

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

## P7 — Remote display

**Goal.** The desktop in a browser tab at interactive latency.

Deliverables:

- DMA-BUF capture → VA-API encode → str0m → WebCodecs canvas.
- Virtual seat input over a data channel.
- Signaling service, pairing code, DTLS.
- Fallback ladder (WebSocket + fMP4, JPEG tiles).
- AF_XDP only if measurement justifies it.

**Gate.** Glass-to-glass < 40 ms LAN; no CPU pixel copies; p99 frame interval
< 1.5× target.

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
