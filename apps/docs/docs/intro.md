---
id: intro
title: Aether OS
sidebar_position: 1
slug: /
description: "A single-target Linux workstation OS: Rust compositor, zero-copy IPC, capability-isolated agents."
---

# Aether OS

**A single-target Linux workstation OS built for exactly one machine** — an Intel
Core i7-7700K (Kaby Lake, 4 cores / 8 threads). Portability is a non-goal.
Determinism, latency and a small resident footprint are the goals.

<div className="doc-callout">
  <strong>Status: v0.1 “Kestrel” — installable base.</strong> The product
  surfaces, the tested core libraries and the full OS configuration exist in this
  repository. The NixOS system and a live installer ISO evaluate, and the install
  path for the reference machine is written end to end. The compositor UI and
  daemons are still P2/P3 work. See the{" "}
  <a href="/roadmap">roadmap</a> and the <a href="/getting-started/install">install guide</a>.
</div>

## The thesis

Conventional desktops are built for every machine, so they carry abstraction
layers, background services and UI toolkits that a single-user workstation does
not need. Aether removes them:

- **One compositor process owns the display.** The window manager, command
  palette, AI chat and system chrome are layers in the same scene graph — not a
  stack of Electron, Qt or GTK applications.
- **The search index lives in memory** and answers with field-weighted BM25,
  prefix expansion and bounded edit distance.
- **Agents are untrusted by default.** Kaby Lake has no PKU, so isolation is a
  Landlock process or a microVM — never a promise.
- **Every performance claim is a budget with a gate.** Numbers that cannot be
  measured yet are labelled targets, not results.

## What exists today

| Area | State | Where |
| --- | --- | --- |
| Marketing site + live palette + explorer demo | ✅ built, tested | `apps/web` |
| Documentation site | ✅ built | `apps/docs` |
| Domain model and boundary validation | ✅ 22 tests | `packages/protocol` |
| Search engine (BM25F, prefix, fuzzy, highlights) | ✅ 59 tests | `packages/indexer` |
| SPSC ring buffer + capability table | ✅ 28 tests incl. cross-thread | `packages/ipc` |
| Isolation planner, scheduler, collaboration bus, audit | ✅ 39 tests | `packages/agents` |
| Headless command palette | ✅ 27 tests | `packages/palette` |
| React UI kit on the Water design system | ✅ 17 DOM tests | `packages/ui` |
| Benchmark harness + regression gate | ✅ baseline committed | `packages/bench` |
| NixOS flake + installer ISO | ✅ evaluates; not booted in CI | `os/nixos` |
| disko layout + install script | ✅ written, evaluation-checked | `os/nixos/disko.nix`, `scripts/install.sh` |
| aarch64 development VM | ✅ build + booted headless (~10–12 s) | `os/vm` |
| Smithay winit backend | ✅ Linux-only; tiled xdg-shell clients under Xvfb | `os/compositor/aether-shell-smithay` |
| Compositor core + shell state + preview | ✅ 49 Rust tests | `os/compositor` |
| Compositor UI (`aether-shell`) | ⏳ P2 | — |
| Remote display | ⏳ P7 | — |

## Reading paths

| Path | Time | Start here |
| --- | --- | --- |
| Install it | 1–2 h | [Install on the 7700K](/getting-started/install) · [First boot](/getting-started/first-boot) · [Hardware notes](/getting-started/hardware) |
| Boot it virtually | 30 min | [Development VM](/getting-started/virtual-machine) — separate aarch64 target, no hardware changes |
| Product vision | 15 min | [PRD](/product/prd) · [Personas & jobs](/product/personas-and-jobs) · [Scope & non-goals](/product/scope-and-non-goals) · [UX principles](/product/ux-principles) |
| System design | 45 min | [Architecture overview](/architecture/overview) · [Hardware target](/architecture/hardware-target) · [Agent isolation](/architecture/agent-isolation) · [Compositor](/architecture/compositor) |
| Build it | now | [Development setup](/contributing/development) · [Testing](/contributing/testing) · [Roadmap](/roadmap) · [ADRs](/adr) |
| Unvarnished numbers | required | [Claims & evidence](/reference/claims-and-evidence) · [Performance engineering](/architecture/performance-engineering) · [Metrics](/product/metrics) |

## The five commitments

1. **Single target.** `-C target-cpu=native`, custom kernel configuration, hardware
   facts encoded in code. No generic fallbacks in the hot path.
2. **No idle tax.** No Electron, no Qt/GTK runtime in the shell, no polling
   indexer. Idle budget: **≤ 200 MB RAM, < 0.5% CPU**.
3. **Explicit trust.** Untrusted agents run in a separate address space with
   Landlock + seccomp; hostile code runs in a microVM. Collaboration is
   capability-gated.
4. **Measured, not asserted.** Each performance claim is measured, budgeted, or
   explicitly speculative. There is no fourth category.
5. **Legible.** Every architectural choice has an ADR; every subsystem has a spec
   with non-goals and failure modes.

## Repository map

```text
apps/web            marketing site, live palette and explorer demo (Vite + React)
apps/docs           this documentation (Docusaurus)
packages/protocol   domain types + runtime validation
packages/indexer    tokenizer, inverted index, BM25, fuzzy, filesystem ingestion
packages/ipc        memfd-style SPSC ring + capability table
packages/agents     isolation planner, admission control, collaboration bus, audit
packages/palette    headless command palette + search adapter
packages/ui         React components on the Water design system
packages/bench      benchmark harness + regression gate
os/nixos            flake, kernel, hardware, filesystem, services
os/compositor       Rust scene graph, tiling layout, frame pacing
e2e                 Playwright specs for both built sites
```

The documentation is the contract. If code and documentation disagree, the
disagreement is a bug with an owner.
