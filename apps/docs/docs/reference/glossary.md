---
id: glossary
title: Glossary
sidebar_position: 2
description: Terms used across the architecture, with the meaning Aether intends.
---

# Glossary

| Term | Meaning in Aether |
| --- | --- |
| **aetherd** | The supervisor/broker: isolation planning, admission control, capabilities, audit |
| **Agent** | A bounded process that executes model-directed work; untrusted by default |
| **Admission control** | Enforcing CPU/memory/agent-count budgets before starting a worker |
| **BM25F** | Field-weighted BM25 ranking used by the index |
| **Capability** | A token granting one scope on one resource to one subject |
| **Compositor** | The process that owns the display, input and scene graph |
| **Damage** | The region of the screen that changed and must be redrawn |
| **DMA-BUF** | Kernel buffer-sharing mechanism enabling zero-copy GPU paths |
| **Dock** | Not used. Window management is tiling; there is no dock |
| **Enforcement** | The mechanism behind a tier: PKU, Landlock/seccomp, or microVM |
| **Frecency** | Usage frequency plus recency; a ranking boost |
| **Gate** | A measurable condition that ends a phase |
| **Index** | The in-memory inverted index over apps, files, content and commands |
| **Isolation tier** | `in-process`, `process-landlock`, or `microvm` |
| **Landlock** | Linux LSM (5.13+) for unprivileged filesystem/syscall restrictions |
| **memfd** | Anonymous in-memory file used to share a ring between processes |
| **MicroVM** | Firecracker VM with a minimal guest kernel and optional VFIO passthrough |
| **Palette** | The modal search surface (`⌘K`) |
| **PGO** | Profile-guided optimisation |
| **PKU / MPK** | Intel Memory Protection Keys; **unavailable on Kaby Lake** |
| **Ring** | Single-producer/single-consumer shared-memory buffer |
| **seccomp-BPF** | Syscall allowlist enforced by the kernel |
| **SPSC** | Single producer, single consumer |
| **ThinLTO** | Link-time optimisation across crate boundaries |
| **Tier** | See isolation tier |
| **VFIO** | Device passthrough to a VM via the IOMMU |
| **Water** | The design language and its `--ds-*` tokens |
| **WebCodecs** | Browser API for low-level video decode/encode |
| **Zero-copy** | A path where pixel data is not touched by the CPU |

## Hardware terms

| Term | Meaning |
| --- | --- |
| Kaby Lake | Intel microarchitecture of the i7-7700K (2017) |
| AVX2 | 256-bit SIMD extension available on the target |
| QuickSync | Intel fixed-function video encode/decode |
| IOMMU / VT-d | Hardware DMA remapping required for safe VFIO |
| SMT | Simultaneous multithreading (8 threads on 4 cores) |

## Naming

- **Aether** — the project.
- **Kestrel** — release codename for v0.1.
- **Kestrel config** — the committed kernel config (`os/nixos/kernel/kestrel.config`).
- **Water** — the visual language.
