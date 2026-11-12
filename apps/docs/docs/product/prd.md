---
id: prd
title: Product requirements
sidebar_position: 1
description: What Aether OS is, who it is for, and what v1.0 must do.
---

# Product requirements

## Vision

A workstation operating system that treats the machine as a **fixed target**.
Every layer is chosen for one CPU, one GPU, one user, and then compiled and tuned
for it. The result is a desktop that starts instantly, idles under 200 MB, answers
search in single-digit milliseconds, and can run fleets of isolated coding agents
without the UI losing a frame.

## Problem

A power user running local AI agents on a 4-core workstation fights three things:

1. **Background tax.** Desktop environments, indexers and Electron apps consume
   the headroom agents need.
2. **Latency stacks.** Input travels through compositor → toolkit → app → IPC →
   back. Every hop adds jitter.
3. **Trust ambiguity.** Agents and tools share the user's session, filesystem and
   credentials. Isolation, when present, is bolted on with containers that cost
   more than they protect.

## Goals

| ID | Goal | Measure |
| --- | --- | --- |
| G1 | Boot to an interactive shell fast | < 1.5 s userspace on the reference machine |
| G2 | Stay out of the way at idle | ≤ 200 MB RAM, < 0.5% CPU, zero polling |
| G3 | Make search a reflex | palette p95 < 10 ms at 10k documents |
| G4 | Never drop a frame at 120 Hz | frame budget 8.3 ms, p99 < 8.3 ms |
| G5 | Run agents safely | 6 concurrent workers, zero cross-agent reads in red-team tests |
| G6 | Make collaboration explicit | every cross-agent byte passes a capability check |
| G7 | Keep the machine legible | every subsystem documented with failure modes |

## Non-goals

- Running on hardware other than the reference machine.
- Multi-user, remote login or enterprise fleet management.
- Windows/macOS application compatibility.
- Being a general-purpose distribution or an installer for third parties.
- Cloud sync, telemetry, accounts or telemetry-adjacent “analytics”.
- A plugin ecosystem for the shell. Extensions are compiled in or isolated as
  agents.

## Functional requirements

### F1 — Shell and display

- Wayland compositor written in Rust (Smithay), Vulkan/wgpu renderer.
- Tiling layout with keyboard-first navigation; floating surfaces for overlays.
- Damage-tracked scene graph; one draw pass for shell chrome.
- 120 Hz pacing with a deadline scheduler; graceful degradation to 60 Hz.
- Output hotplug, fractional scaling, per-output refresh.

### F2 — Command palette

- Single shortcut (`⌘K`/`Super+K`) opens a floating palette layer.
- Searches applications, files, file content, commands, agents and settings.
- Mode prefixes: `>` commands, `/` files, `@` agents, `~` settings.
- Ranking: BM25F + exact/prefix/fuzzy + recency + usage.
- Keyboard only: arrows, page keys, home/end, enter, escape.
- Result set must update without blocking the frame; stale queries discarded.

### F3 — AI chat and agents

- Chat surface streams tokens from a local DeepSeek endpoint over a Unix socket.
- Markdown subset: fenced code, inline code, bold, italic, links.
- Agents are spawned through `aetherd`; isolation is planned by policy, not chosen
  by the agent.
- Agent limits: CPU weight, memory cap, wall-clock timeout, capability set.
- Collaboration only through broker-issued channels.

### F4 — Files

- Two-pane explorer with independent roots and per-row status.
- Operations: open, copy, move, rename, delete, reveal.
- Search filters rows instantly; content search delegates to the index daemon.
- Status classes are semantic and always include a text label.

### F5 — Audio

- Enumerate and switch every input and output, including Bluetooth.
- Per-application routing and volume.
- Device changes must not require restarting applications.
- Low-latency path for media and calls; no resampling by default.

### F6 — Browser and media

- WebKitGTK browser shell with tabs, minimal permissions, per-tab process
  isolation and no extension surface.
- Media player wrapping libmpv with hardware decode and subtitle support.
- Both run as sandboxed clients of the shell, not privileged components.

### F7 — Remote display (post-v1.0)

- Stream the desktop to a browser with WebRTC + WebCodecs.
- Low-latency encode settings, zero CPU pixel copies.
- Input forwarded over data channels into a virtual seat.

## Constraints

- **CPU:** 4 physical cores / 8 threads. Concurrency budgets assume one core is
  reserved for the shell, indexer and audio.
- **No PKU.** Kaby Lake lacks Memory Protection Keys; isolation must not depend on
  WRPKRU.
- **GPU:** HD 630 (QuickSync) or a discrete GPU. Hardware encode/decode is a
  requirement for remote display and media.
- **No portability layer.** `-march=native`, AVX2 baseline, no generic kernels.
- **Single writer.** XFS root, no copy-on-write snapshots on the hot path.

## Release criteria for v1.0

1. Cold boot to a usable shell on the reference machine, measured.
2. Palette, chat, explorer, browser and media player usable daily.
3. Six concurrent agents with enforced isolation and a verifiable audit chain.
4. Audio routing covers analog, USB and Bluetooth with no restarts.
5. All budgets in the performance ledger are either met or explicitly waived in an
   ADR with data.
6. A new contributor can build and test the repo from a clean checkout using only
   the documentation.

## Success metrics

See [Metrics](/product/metrics). The short version: **time-to-first-result**,
**frame stability**, **idle footprint**, and **isolation incidents**. Downloads
and sign-ups are not metrics; there is no account system.
