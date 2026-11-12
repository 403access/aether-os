---
id: metrics
title: Metrics
sidebar_position: 5
description: The four numbers that matter, how they are measured, and what is deliberately not measured.
---

# Metrics

Aether has no analytics pipeline. Metrics exist as **local measurements and CI
gates**, not as a dashboard in someone else's cloud.

## North-star metrics

| Metric | Target | Measurement | Owner |
| --- | --- | --- | --- |
| Time to first result | < 120 ms from `⌘K` to painted results | Input timestamp → damage commit, 1k warm queries | Shell |
| Frame stability | p99 < 8.3 ms at 120 Hz | Compositor frame trace, 10-minute idle + load | Compositor |
| Idle footprint | ≤ 200 MB RAM, < 0.5% CPU | `smem`/`procps` snapshot 60 s after boot | Platform |
| Isolation incidents | 0 in red-team suite | Capability fuzz + cross-agent read attempts | aetherd |

The first three are **speed**. The fourth is the reason the system exists: a fast
machine that lets an agent read your SSH keys is not a win.

## Supporting metrics

| Metric | Target | Where gated |
| --- | --- | --- |
| Index build, 10k docs | < 250 ms | `bench --check` |
| Index query p95, 10k docs | < 10 ms | `bench --check` |
| Palette p50, 1k items | < 2 ms | `bench --check` |
| Ring throughput | > 2 M msgs/s | `bench --check` |
| Capability verify | > 2 M ops/s | `bench --check` |
| Userspace boot | < 1.5 s cold | P1 boot test |
| Audio route switch | < 100 ms | P4 harness |
| Glass-to-glass remote | < 40 ms LAN | P7 harness |

## What is measured now

The current baseline was recorded on the development host (Apple M3, Bun 1.4) and
committed to `bench/baseline.json`. CI compares **relative** changes against that
baseline with a 25% tolerance. Absolute numbers on the i7-7700K are a P1
deliverable.

Reference measurements at v0.1:

| Metric | Value |
| --- | --- |
| Index build, 5k docs | 80 ms |
| Index query p50, 5k docs | 1.65 ms |
| Index query p95, 5k docs | 2.23 ms |
| SPSC ring throughput | 4.5 M msgs/s |
| Capability verify | 9.2 M ops/s |
| Palette p50, 1k items | 0.78 ms |

These are **reference-host measurements**, not promises about the target machine.
They exist to catch regressions and to keep the optimisation work honest.

## What is deliberately not measured

- **Engagement.** There is no session length target. The best session is short.
- **Feature adoption.** One user. Adoption is “does the operator use it”.
- **Telemetry-derived crash rates.** Crashes are local logs.
- **Download counts.** The project is source-distributed.

## Measurement rules

1. **Same machine, same governor.** Record CPU model, clocks, kernel and toolchain
   with every number.
2. **Warm and cold.** Report both; never quote only the flattering one.
3. **p50 and p95.** A mean hides the tail that users feel.
4. **No synthetic workload without a trace.** Benchmarks must resemble a real
   interaction (typing a query, scrolling a file list, switching audio).
5. **Store the raw samples.** `bench-results/` keeps JSON; summaries are derived.
6. **Regressions block merges.** A 25% regression requires either a fix or an ADR
   explaining why the new cost is acceptable.

## Budget ledger

The canonical table lives in [the performance ledger](/architecture/performance-engineering#the-budget-ledger)
and in the marketing site. Each row has a gate; a number without a gate is a wish.

## Reporting to the user

The status bar shows only live, local values: agent count, CPU, memory, audio route,
frame time. No graphs by default; graphs are available in the admin overlay when
the user asks for them.

This is the entire metrics strategy. If a future feature needs analytics, it needs
an ADR that explains why local measurement is insufficient.
