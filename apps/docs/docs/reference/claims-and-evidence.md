---
id: claims-and-evidence
title: Claims and evidence
sidebar_position: 1
description: Every performance claim, labelled measured, budgeted or speculative — including the ones from the original design conversation.
---

# Claims and evidence

The original architecture conversation produced a lot of numbers. Some are
measured, some are budgets, and some are speculative. This page is the audit.

**Rule:** a number is only a claim if it appears in this table with a label and a
source. Everything else is a target or an aspiration.

## Labels

| Label | Meaning |
| --- | --- |
| **Measured** | Produced by a committed harness on a named host |
| **Budget** | An engineering target with a gate; not yet achieved |
| **Speculative** | An estimate from the design discussion; not validated |
| **Refused** | A number we will not repeat because the design does not support it |

## Idle footprint

| Claim | Label | Evidence |
| --- | --- | --- |
| 130–200 MB idle RAM | **Budget** | Installer and VM exist; no measured target idle footprint yet; P1 measures |
| 110–150 MB with all optimisations | **Speculative** | MicroVM/PGO combination not built |
| 10–20 MB saved by shared-memory IPC | **Speculative** | Ring measured; whole-system saving not |
| GNOME idles 1.2–2.5 GB | **Speculative** | Depends on distro/session; not measured here |
| < 0.1% idle CPU | **Budget** | P1 gate |

## Latency

| Claim | Label | Evidence |
| --- | --- | --- |
| Index query p50 1.65 ms / p95 2.23 ms at 5k docs | **Measured** | Apple M3, Bun 1.4, `bench/baseline.json` |
| Palette p50 0.78 ms at 1k items | **Measured** | Same harness |
| IPC round-trip 15–30 ns | **Refused** | A ring push+pop is ~220 ns on the dev host; 15 ns ignores framing and cache effects |
| 100× faster IPC than sockets | **Speculative** | Depends on payload; no controlled comparison committed |
| Network latency 15–45 µs with AF_XDP | **Speculative** | Not implemented; hardware-dependent |
| 50× lower jitter with AF_XDP | **Speculative** | Not measured; claim withdrawn until tested |
| Glass-to-glass < 40 ms | **Budget** | P7 gate |
| Input → paint < 16 ms | **Budget** | P2 gate |

## Concurrency

| Claim | Label | Evidence |
| --- | --- | --- |
| 25–40 concurrent agents | **Refused** | Assumes 16+ cores; the target has 4 |
| 4–6 concurrent CPU-bound agents | **Budget** | Admission control enforces 6; P5 measures |
| 8× agent density vs. a desktop | **Speculative** | No controlled benchmark |
| 6 agents max | **Enforced** | `packages/agents` scheduler tests |

## Compiler and isolation

| Claim | Label | Evidence |
| --- | --- | --- |
| PGO + ThinLTO adds 10–25% | **Speculative** | Standard range; not measured for this codebase |
| `target-cpu=native` adds 5–15% | **Speculative** | Not measured yet |
| PKU gives 10 ns isolation | **Refused** | Kaby Lake has no PKU; not applicable |
| Context switch ~1–2 µs | **Budget** | Measured in P5, not assumed |
| MicroVM adds 5–15 MB | **Speculative** | Firecracker overhead, not measured here |

## Boot and storage

| Claim | Label | Evidence |
| --- | --- | --- |
| < 50 ms boot | **Refused** | That is a microVM snapshot restore, not a cold desktop boot |
| < 1.5 s userspace boot on the 7700K | **Budget** | P1 gate |
| ~10 s multi-user boot in the aarch64 VM | **Measured (dev host)** | Apple M3, HVF, 4 vCPU; not the target |
| XFS is fastest for this workload | **Qualified** | Reasoned from design; not benchmarked on this machine |
| Btrfs/ZFS rejected | **Decision** | ADR-0003; not a performance claim |

## What “measured on the dev host” means

The development host is an Apple M3 with Bun 1.4. It is **not** the reference
machine. The committed baseline is useful for detecting regressions, and it is
**not** evidence about i7-7700K performance. Any absolute claim must be repeated on
the target before it appears in marketing copy.

## How numbers are added

1. Measure with the committed harness.
2. Record host, toolchain and raw JSON.
3. Update the ledger and this table.
4. If the number is a target, add the gate that will prove it.
5. If the design changes, re-label or remove the claim. Deleting a claim is a
   success, not a loss.

## Claims we will not make

- “Blazing fast”, “instant”, “zero latency” without a number and a method.
- “More secure” without a threat model and a test.
- “Works everywhere” or “portable”.
- “Production ready”.
- “AI-powered” as a substitute for a capability description.

The value of the project is that the numbers are inspectable. Protecting that is
more important than any single benchmark.
