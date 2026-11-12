---
id: overview
title: Architecture overview
sidebar_position: 1
description: The four-layer model, the process boundary, trust boundaries and the data paths that matter.
---

# Architecture overview

Aether is four layers, one supervisor, one compositor, and an explicit trust
boundary around every agent. Nothing else runs at idle.

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ aether-shell (Rust · Smithay · wgpu)                                    │
│   compositor · tiling layout · scene graph · palette/chat/status layers │
├─────────────────────────────────────────────────────────────────────────┤
│ clients: palette · chat · explorer · media (libmpv) · browser (WebKit)  │
├─────────────────────────────────────────────────────────────────────────┤
│ aetherd        supervisor, capability broker, admission control         │
│ aether-index   in-memory BM25F index, inotify ingestion, snapshots      │
│ aether-ipc     memfd SPSC rings, record framing, backpressure           │
├─────────────────────────────────────────────────────────────────────────┤
│ PipeWire · WirePlumber · XFS (+ ext4 scratch) · systemd · udev          │
├─────────────────────────────────────────────────────────────────────────┤
│ NixOS base · custom Kaby Lake kernel · initrd · boot                    │
└─────────────────────────────────────────────────────────────────────────┘
```

## Process model

| Process | Language | Lifetime | Privilege | Why separate |
| --- | --- | --- | --- | --- |
| `aether-shell` | Rust | session | user | Owns the display and input; the only UI trust anchor |
| `aetherd` | Rust | session | user (CAP for microVM) | Broker: policy, capabilities, audit |
| `aether-index` | Rust | session | user, Landlock | Reads the filesystem; must not be able to write |
| `aether-ipc` | library | — | — | Shared protocol, not a daemon |
| Palette/chat | Rust (compositor layers) | session | user | Same scene graph, no separate window |
| Explorer | Rust or web layer | session | user | Dense list surface; can be a shell layer |
| Browser | WebKitGTK | per tab | sandboxed | Untrusted web content |
| Media | libmpv client | per file | sandboxed | Untrusted media codecs |
| Agents | per plan | bounded | **untrusted** | Never share the shell address space |

## Trust boundaries

1. **Kernel ↔ userspace.** Every capability check happens in user space first; the
   kernel enforces filesystem and syscall restrictions as the second line.
2. **Shell ↔ clients.** A client never receives a raw pointer to shell memory.
   Surfaces are transferred as buffers with explicit lifetimes.
3. **Agent ↔ agent.** No shared address space unless the planner allows in-process
   trusted code. Untrusted agents are separate processes with separate UIDs.
4. **Agent ↔ host.** Landlock allowlists, seccomp syscall allowlists,
   `no_new_privs`, rlimits, timeouts, and no ambient network access.
5. **Remote client ↔ host.** WebRTC DTLS + a bearer token bound to a session; input
   is validated as scancodes, never as commands.

## The four data paths

### 1. Palette query

```text
keystroke → aether-shell input → palette engine
          → aether-index (in-process or ring query)
          → ranked hits → scene graph → damage → present
```

The fast path has no serialization: the palette and index live in the same address
space in the shell build, and in the daemon build they exchange fixed-layout
records over the ring. Target: p95 < 10 ms at 10k documents.

### 2. Chat token stream

```text
local DeepSeek endpoint (Unix socket)
   → aether-chat reader → UTF-8 validation → markdown segments
   → scene graph text layer → damage only the tail
```

Tokens are appended to a rope; the text layer invalidates only the changed line.
The chat surface never re-lays-out the whole transcript while streaming.

### 3. Agent handoff

```text
worker A → capability check → SPSC ring (memfd) → worker B
                    ↑
              aetherd audit chain
```

No JSON serialization on the hot path. A message is a 16-byte header plus payload;
the capability token is a 64-bit hash checked before the record is accepted.

### 4. Remote frame

```text
compositor DMA-BUF → VA-API encode → str0m (WebRTC) → browser
                                                 → WebCodecs → canvas
```

There is no CPU pixel copy in the intended path. Input returns over a data channel
and is injected into a virtual seat after validation. See
[remote display](/architecture/remote-display).

## Failure model

Aether prefers **fail-closed and degrade visibly** over fail-open:

| Failure | Behaviour |
| --- | --- |
| Index daemon dies | Palette falls back to app/command providers; status bar shows index stale |
| Audio daemon dies | Existing streams continue via PipeWire; route changes disabled |
| Agent exceeds memory | Cgroup OOM kills the agent; audit records the event |
| Browser tab crashes | Tab shows crash state; shell unaffected |
| Compositor frame misses | Frame is dropped, never queued; latency stays bounded |
| Remote encoder stalls | Client shows degraded quality badge; input remains live |
| Capability revoked mid-flight | Next operation returns `revoked`; audit records denial |

## What is deliberately centralised

- **Display and input** live in one process. Splitting them would add IPC to the
  latency path for no security gain.
- **Policy** lives in `aetherd`. Agents never decide their own isolation tier.
- **The audit chain** is append-only and hash-chained. There is no “delete log”
  capability.

## What is deliberately distributed

- **Process isolation** for untrusted code.
- **Web content** in WebKit's own process model.
- **Media decoding** in libmpv, out of the shell.
- **Index ingestion** in a sandboxed daemon that can read but not write.

## Repository mapping

| Concept | Code |
| --- | --- |
| Domain types | `packages/protocol` |
| Search | `packages/indexer` |
| Ring + capabilities | `packages/ipc` |
| Policy + scheduling + collaboration | `packages/agents` |
| Palette state machine | `packages/palette` |
| UI components | `packages/ui` |
| Compositor core | `os/compositor` |
| Reference OS configuration | `os/nixos` |
| Shared Nix base | `os/common/base.nix` |
| Development VM (separate target) | `os/vm` |
| Boundary enforcement | `tests/separation.test.ts` |

Read next: [hardware target](/architecture/hardware-target) — every capacity
decision in this document is derived from it.
