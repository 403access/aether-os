---
id: 0004-isolation-without-pku
title: ADR-0004 — Isolation without PKU
sidebar_position: 5
---

# ADR-0004 — Isolation without PKU

**Status:** Accepted · **Date:** 2026-11-11

## Context

The original design proposed Intel Memory Protection Keys: one key per agent, switch
with `WRPKRU` in ~10 ns. The reference CPU is an i7-7700K (Kaby Lake), which **does
not support PKU**. PKU also only protects memory; it does not filter syscalls.

## Decision

Untrusted agents run in separate processes with Landlock, seccomp-BPF, per-agent
UIDs and `no_new_privs`. Hostile agents run in a microVM with VFIO. In-process
execution is reserved for explicitly trusted code. PKU is not part of the v1.0
design.

## Consequences

- Isolation costs a context switch and address-space bookkeeping instead of an
  instruction.
- Filesystem and syscall restrictions are stronger than PKU alone.
- Memory separation relies on page tables, which is well understood.
- Cross-agent sharing requires explicit channels and capabilities.
- On future hardware with PKU, an additional in-process tier can be added without
  changing the default policy.

## Alternatives

- **Keep the PKU design** — impossible on the target; would be a false claim.
- **Containers only** — heavier than Landlock processes and weaker than microVMs for
  hostile code.
- **MicroVM for everything** — too expensive in memory and boot time for ordinary
  workers.
- **Rust `unsafe` discipline alone** — not a security boundary against generated
  code.
