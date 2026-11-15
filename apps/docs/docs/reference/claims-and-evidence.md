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
| 130–200 MB idle RAM | **Budget** | Installer and VM exist; harness committed (`scripts/memory-report.sh`); no measured target idle footprint yet; P1 measures |
| Attributed session footprint ≤ 200 MB | **Budget** | Σ PSS + resident DRM of session processes, 60 s after boot and after a fixed workload; definition in [memory footprint](/architecture/memory-footprint) |
| Idle growth ≤ 1 MB / 10 min | **Budget** | Two harness samples with no input; retention is covered by the post-workload attributed run |
| 11.4 MiB shell core + one 1920×1200 software frame | **Measured (dev host)** | `cd os/compositor && cargo build --release -p aether-preview`; `/usr/bin/time -l` maximum RSS 12,009,472 B; includes the 8.8 MiB canvas |
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
| Glass-to-glass < 40 ms | **Budget** | P7 gate; harness + client committed, target run pending |
| Input → paint < 16 ms | **Budget** | P2 gate |

## Compositor (P2)

| Claim | Label | Evidence |
| --- | --- | --- |
| Input → damage p50 ≈ 24 µs, p99 ≈ 190 µs | **Measured (dev host)** | Xvfb/winit palette run, `evidence/p2/frames.ndjson`, `scripts/frame-stats.py` |
| Chrome text rasterizes once per unique label | **Measured (dev host)** | `AETHER_TEXT_STATS` counter stable across static frames |
| Present interval p50 ≈ 55 ms | **Not a performance claim** | Xvfb + xdotool sleeps; only proves the delta loop runs |
| Sustained 120 Hz, p99 < 12.5 ms | **Budget** | P2 gate, reference machine; not measured here |
| Queue depth ≤ 1 | **Budget** | P2 gate; tracked by G6, VM instrumentation |
| Input → damage < 2 ms | **Budget** | P2 gate; dev-host proxy is measured above |

## Shell surfaces (P3)

| Claim | Label | Evidence |
| --- | --- | --- |
| Palette p95 1.11 ms at 10k files | **Measured (dev host)** | Rust release test, `evidence/p3-palette.log` |
| Explorer scroll p95 0.004 ms at 10k rows | **Measured (dev host)** | Rust release test, `evidence/p3-explorer.log` |
| Chat keeps presenting while tokens stream | **Measured (dev host)** | 52 frames in 3 s, `evidence/p3-chat.ndjson` |
| Palette p95 < 10 ms on the 7700K | **Budget** | P3 gate; dev-host proxy is 1.11 ms |
| Explorer 10k rows at 120 Hz | **Budget** | P3 gate; needs reference machine |
| No dropped frames while streaming on target | **Budget** | P3 gate; dev-host smoke only |

## Audio (P4)

| Claim | Label | Evidence |
| --- | --- | --- |
| Microphone denied without `audio.capture` | **Measured (unit test)** | `packages/agents/src/audio.test.ts`; legacy `audio` scope also denied |
| Audio overlay switches routes/presets | **Measured (fixture)** | `smoke-audio-xvfb.sh`; 5 fixture routes, switch + preset asserted |
| Route switch < 100 ms | **Budget** | `scripts/audio-stress.sh`; target run pending |
| Zero xruns, 10-minute 64-frame stress | **Budget** | `scripts/audio-stress.sh`; target run pending |
| Input meter matches `pw-top` within 5% | **Not claimed** | Surface is fixture-driven; live peak sampling lands with P5 |

## Agent sandbox (P5)

| Claim | Label | Evidence |
| --- | --- | --- |
| Landlock denies a file outside the allowlist | **Measured (Linux unit test)** | `aether-sandbox` integration test |
| An out-of-scratch write is denied | **Measured (Linux unit test)** | `aether-sandbox` integration test |
| Seccomp denies `unshare` inside the sandbox | **Measured (Linux integration test)** | `aether-sandbox` runs `unshare` under the filter |
| Seccomp network syscalls are denied unless `net` | **Measured (filter test)** | `denied_syscalls` assertions; syscall fuzzing is P5 |
| Sandbox overhead | **Not claimed** | No helper process; rules install once between fork and exec, but no target benchmark yet |

## Remote display (P7)

| Claim | Label | Evidence |
| --- | --- | --- |
| Software H.264 encodes 320×180 at p99 ≈2.2–2.6 ms | **Measured (dev host)** | `aether-remote bench`, `artifacts/p7/bench.ndjson`; not the target |
| Synthetic pipeline interval p99 ≈19.4–20.5 ms at 60 fps | **Measured (dev host)** | 1.5× budget is 25 ms; target run pending |
| DMA-BUF descriptor records zero copied bytes | **Measured (unit test)** | `aether-remote` capture tests + `artifacts/p7/capture.json` |
| str0m loopback establishes DTLS and opens the input channel | **Measured (integration test)** | `aether-remote` transport test; input echoes; `artifacts/p7/webrtc.json` |
| Pairing code is one-shot, expiring, and the audit chain verifies | **Measured (unit test)** | `aether-remote` session tests |
| Tier ladder selects WebRTC / WS-AU / JPEG / VNC | **Measured (unit test)** | `packages/remote` tests |
| Glass-to-glass < 40 ms on LAN | **Budget** | P7 gate; needs the reference machine capture path |
| No CPU pixel copies on target | **Budget** | Audit + descriptors exist; compositor DMA-BUF import pending |
| Input round trip < 20 ms on target | **Budget** | Loopback input channel tested; target timing pending |

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
| ~10–12 s multi-user boot in the aarch64 VM | **Measured (dev host)** | Apple M3, HVF, 4 vCPU; not the target |
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
