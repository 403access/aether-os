---
id: 0005-shared-memory-ipc
title: ADR-0005 — Shared-memory SPSC IPC
sidebar_position: 6
---

# ADR-0005 — Shared-memory SPSC IPC

**Status:** Accepted · **Date:** 2026-11-11

## Context

The shell, indexer, audio policy and agents exchange high-frequency state. Unix
sockets plus JSON add copies, allocation and serialization latency. The machine has
four cores; context switching and cache pollution are expensive.

## Decision

Use fixed-layout records over a single-producer/single-consumer ring mapped from
`memfd_create`. One 64-byte header, 16-byte record headers, an 8-byte capability
token hash, and a wrap marker. Backpressure is explicit and per channel.

## Consequences

- Near-zero serialization cost and predictable memory.
- Exactly one producer and one consumer per ring; richer topologies need multiple
  rings or a broker.
- Versioning is per message kind; unknown kinds are ignored.
- Debugging requires tooling (hexdump, ring inspector), not a socket proxy.
- The TypeScript implementation mirrors the Rust layout and is tested across threads.

## Alternatives

- **Unix sockets + JSON** — simple, portable, but allocation and parsing on every
  keystroke.
- **D-Bus** — heavy, asynchronous, adds a daemon and policy layer.
- **io_uring** — excellent for I/O, not for in-memory pub/sub.
- **A full MPMC queue** — more complexity for a topology we do not have.
