---
id: performance-engineering
title: Performance engineering
sidebar_position: 15
description: How Aether measures, optimises and gates performance — without turning targets into claims.
---

# Performance engineering

Performance is a maintained property, not a sprint. This page defines how work is
chosen, measured and rejected.

## Optimisation order

1. **Remove the work.** Fewer processes, fewer copies, fewer frames.
2. **Move the work.** Batch, cache, precompute, lazy-load.
3. **Change the algorithm.** BM25 top-k, better data structures.
4. **Change the representation.** Compact records, cache-friendly layouts.
5. **Compiler tuning.** `target-cpu=native`, LTO, PGO.
6. **Hardware tricks.** DMA-BUF, AF_XDP, VFIO.

Most projects start at 5. Aether starts at 1 and only descends when measurement
demands it.

## The budget ledger

| Metric | Budget | Measurement | Status |
| --- | --- | --- | --- |
| Idle RAM | ≤ 200 MB | 60 s after boot, `smem` | target |
| Idle CPU | < 0.5% | 60 s mean + p95 | target |
| Frame time | 8.3 ms @ 120 Hz | compositor trace | target |
| Input → paint | < 16 ms | evdev → damage commit | target |
| Index build, 10k docs | < 250 ms | bench | target |
| Index p95, 10k docs | < 10 ms | bench | target |
| Palette p50, 1k items | < 2 ms | bench | measured 0.78 ms (dev host) |
| Ring throughput | > 2 M msgs/s | bench | measured 4.5 M (dev host) |
| Capability verify | > 2 M ops/s | bench | measured 9.2 M (dev host) |
| Userspace boot | < 1.5 s | systemd-analyze, cold | target |
| Agent ceiling | 6 | admission tests | enforced now |
| Glass-to-glass | < 40 ms LAN | P7 harness | target |

A row becomes “measured” only when the harness and the reference machine both exist.
Until then it is a target, and the marketing site says so.

## Benchmark harness

```bash
bun run bench            # measure, write bench-results/*.json
bun run bench --check    # compare with bench/baseline.json, exit 1 on regression
bun run bench --update-baseline
```

Metrics:

| Metric | Direction | What it catches |
| --- | --- | --- |
| `index.build` | lower | Tokenizer/index regressions |
| `index.query.p50` | lower | Common-case latency |
| `index.query.p95` | lower | Tail latency |
| `ipc.ring.throughput` | higher | Record overhead |
| `capability.verify` | higher | Broker hot path |
| `palette.query.p50` | lower | Ranking/grouping regressions |

Tolerance is 25% on the reference machine. CI runs the harness with a 3× smoke
tolerance because GitHub's runners are slower than the development host; a 3×
regression still catches an order-of-magnitude mistake without failing on clock
differences. The exact 25% gate is a release step on the target hardware.

Two real bugs found by this harness during v0.1:

1. **Highlights computed for every candidate.** Query p50 was 6.9 ms; compute after
   top-k and it fell to 1.65 ms.
2. **Palette scored matches twice.** Removing the duplicate fuzzy pass cut p50 by
   23%.

Both are documented in commit messages and are examples of why the baseline exists.

## Compiler tuning

| Technique | When | Expected gain | Risk |
| --- | --- | --- | --- |
| `-C target-cpu=native` | always | 5–15% in hot loops | None on a single target |
| ThinLTO | release builds | 3–10% | Longer builds |
| `codegen-units=1` | release builds | 2–5% | Much longer builds |
| PGO | after baselines | 10–25% | Profile staleness |
| BOLT | after PGO | 2–8% | Toolchain complexity |

PGO workflow:

```bash
# 1. instrument
RUSTFLAGS="-Cprofile-generate=/tmp/pgo" cargo build --release
# 2. run a representative workload: palette typing, scrolling, audio switch
# 3. merge and rebuild
llvm-profdata merge -o /tmp/pgo/merged.profdata /tmp/pgo
RUSTFLAGS="-Cprofile-use=/tmp/pgo/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --release
```

The workload is a recorded trace, not a synthetic loop. If the profile cannot be
reproduced in CI, the gain is not claimed.

## What not to optimise yet

- **AF_XDP.** Only if the kernel network stack is measured to dominate remote
  latency.
- **Top-k heap for search.** Only if p95 exceeds 10 ms at 10k docs.
- **Custom allocators.** Only if `perf` shows allocator time in the top three.
- **Lock-free every data structure.** SPSC is enough; MPMC introduces complexity
  without a measured need.
- **Kernel PGO.** P8 at the earliest.
- **MicroVM for every agent.** Context switches are cheaper than VM exits for
  ordinary work; microVMs are for hostile code.

## Measurement hygiene

1. Record the machine: CPU, governor, kernel, toolchain, Bun version.
2. Warm up before measuring; report cold separately.
3. Pin to a core (`taskset`) when comparing microbenchmarks.
4. Close the browser. Seriously.
5. Use p50 and p95; never quote a mean alone.
6. Keep raw JSON. Summaries are derived and can be recomputed.
7. Compare on the same host; cross-host comparisons are anecdotal.

## CPU-specific notes (i7-7700K)

- 4 cores / 8 threads: the scheduler reserves one core for the shell and audio.
- AVX2 is available; AVX-512 is not. SIMD code should target AVX2.
- 8 MB L3: keep the hot index working set under a few megabytes where possible.
- Turbo is 4.5 GHz but sustained loads will clock lower; budgets assume 4.2 GHz.
- No PKU: isolation costs a context switch, not an instruction. Budget accordingly.

## The rule

> If a number is not in the ledger with a gate, it does not go in the README,
> the site, or a design review.

This is enforced socially and, for the benchmarked metrics, technically.
