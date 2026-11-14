---
id: hardware-target
title: Hardware target — i7-7700K
sidebar_position: 2
description: The reference machine, its capabilities, its limits, and what each fact changes in the design.
---

# Hardware target — i7-7700K

Aether is compiled for one CPU. This page is the dossier that every capacity
decision refers to; the canonical machine-readable copy is `KABYLAKE_7700K` in
`packages/protocol/src/index.ts`.

## The machine

| Property | Value | Consequence |
| --- | --- | --- |
| CPU | Intel Core i7-7700K | Kaby Lake, 14 nm |
| Cores / threads | 4 / 8 | 6-agent ceiling; one core reserved for the shell |
| Base / turbo | 4.2 / 4.5 GHz | Thermal headroom matters for sustained loads |
| L3 cache | 8 MB | Index working sets must stay cache-friendly |
| ISA | SSE4.2, AVX2, BMI2, FMA3 | `-C target-cpu=native`; no AVX-512 |
| **PKU** | **No** | The WRPKRU micro-sandbox is unavailable |
| VT-x / VT-d | Yes / Yes | MicroVMs and VFIO passthrough are possible |
| iGPU | Intel HD Graphics 630 | QuickSync encode/decode (VA-API) |
| Memory | 32–64 GB (assumed) | Admission budget is explicit, not “use what is free” |
| Storage | NVMe | XFS root, no copy-on-write penalty |

## The defining constraint: no PKU

Intel Memory Protection Keys arrived on Skylake-SP Xeons and reached mainstream
desktop CPUs later. Kaby Lake does not have them, so this design is impossible:

```rust
// Not available on the reference machine.
pkey_mprotect(agent_heap, len, PROT_READ | PROT_WRITE, pkey);
wrpkru(/* switch domain */);
```

The original architecture discussion proposed 10 ns agent isolation via `WRPKRU`.
On this CPU that would be a lie. Aether instead uses:

| Trust level | Isolation | Enforcement |
| --- | --- | --- |
| Trusted | In-process | Rust ownership + review |
| Untrusted | Separate process | Landlock + seccomp-BPF + `no_new_privs` + per-agent UID |
| Hostile | MicroVM | Firecracker + VFIO, VT-x/VT-d |

The cost is a context switch instead of an instruction. The gain is that the
isolation actually exists. See [agent isolation](/architecture/agent-isolation).

## Preflight

Run `scripts/target-check.sh` on the machine (or with `--root /mnt` before
install) to verify UEFI, i915, `/dev/dri`, connected KMS connectors, input
devices and memory. See [Target preflight](/ops/target-check).

## Capacity math

```text
4 physical cores
− 1 core reserved for shell, indexer, audio, compositor
= 3 cores for agents
= 300 CPU points (100 points = one core)

max concurrent agents = floor(4 × 1.5) = 6
```

Admission control enforces both dimensions independently: a fleet of six
500 mIB agents fits memory but may not fit CPU; a fleet of three 100-point agents
fits CPU but leaves no room for burst. The scheduler queues rather than
overcommitting. See `packages/agents/src/scheduler.ts`.

The earlier discussion’s “25–40 agents” figure assumed a 16-core machine. On this
target the honest figure is **4–6 concurrent CPU-bound workers**, plus lightweight
I/O-bound tasks. Aether documents the number it can enforce.

## Compiler and kernel flags

| Layer | Setting | Reason |
| --- | --- | --- |
| Rust | `-C target-cpu=native -C codegen-units=1` | AVX2 + Kaby Lake scheduling |
| Rust release | ThinLTO, `panic=abort` for daemons | Smaller code, fewer landing pads |
| C/C++ (browser) | `-march=native -O3` where supported | WebKit's hot loops |
| Kernel | `localmodconfig`, Kaby Lake options, no debug symbols | Boot time and attack surface |
| PGO | Recorded workloads: palette typing, scrolling, audio switch | P8, only if measurement pays |

## QuickSync and the GPU path

The HD 630 exposes VA-API encode/decode through iHD. For remote display:

- **H.264** baseline/high is the compatibility floor.
- **HEVC** and **AV1** are preferred when the client decodes them.
- Zero-copy requires DMA-BUF export from the compositor and import into the
  encoder; `vaExportSurfaceHandle` is the relevant path.
- The iGPU shares system memory, so “zero copy” means “no CPU touch”, not
  “no bandwidth cost”.

If a discrete NVIDIA GPU is fitted, NVENC replaces VA-API behind the same encoder
interface. The architecture does not branch on vendor in the compositor.

## What would change on different hardware

| Hardware change | Required change |
| --- | --- |
| CPU with PKU | Isolation planner may add an in-process PKU tier; policy stays default-deny |
| More cores | Admission budgets and the agent ceiling become configuration |
| AVX-512 | Rebuild with `target-cpu=native`; no code changes |
| No VT-d | MicroVM tier is blocked at plan time; hostile agents cannot run |
| No QuickSync | Remote display falls back to software encode with a documented latency penalty |
| Laptop / battery | Out of scope; power management is a fork |

The point of this table is not portability. It is to make the assumptions explicit
so a fork knows exactly what it is changing.

## Verification

The hardware facts are not comments; they are typed and tested:

```ts
export const KABYLAKE_7700K: HardwareProfile = {
  id: "kabylake-7700k",
  cpu: "Intel Core i7-7700K",
  microarchitecture: "Kaby Lake",
  cores: 4,
  threads: 8,
  pku: false,
  landlock: true,
  vtX: true,
  vtD: true,
  igpu: "Intel HD Graphics 630",
  // ...
};
```

`packages/protocol/src/validate.test.ts` asserts `pku === false`. If someone flips
it to `true`, the isolation tests fail. That is deliberate: a hardware assumption
that changes the security model should break the build.
