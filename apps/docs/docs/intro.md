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
  <strong>Status: v0.1 “Kestrel” — foundation release.</strong> The product
  surfaces, the tested core libraries and the full OS configuration exist in this
  repository. The kernel and compositor are validated configuration and a
  dependency-free Rust core, not yet a bootable image. See the{" "}
  <a href="/roadmap">roadmap</a> for the path to boot.
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
| NixOS flake, kernel and hardware modules | ⚠️ written, not booted | `os/nixos` |
| Compositor scene/layout/pacing core | ✅ Rust tests | `os/compositor` |
| Bootable ISO | ⏳ P1 | — |
| Remote display | ⏳ P7 | — |

## Reading paths

<div className="doc-grid">
  <div className="doc-card">
    <h3>I want the product vision</h3>
    <p className="doc-meta">15 minutes</p>
    <p>
      <a href="/product/prd">PRD</a> ·{" "}
      <a href="/product/personas-and-jobs">Personas &amp; jobs</a> ·{" "}
      <a href="/product/scope-and-non-goals">Scope &amp; non-goals</a> ·{" "}
      <a href="/product/ux-principles">UX principles</a>
    </p>
  </div>
  <div className="doc-card">
    <h3>I want to understand the system</h3>
    <p className="doc-meta">45 minutes</p>
    <p>
      <a href="/architecture/overview">Architecture overview</a> ·{" "}
      <a href="/architecture/hardware-target">Hardware target</a> ·{" "}
      <a href="/architecture/agent-isolation">Agent isolation</a> ·{" "}
      <a href="/architecture/compositor">Compositor</a>
    </p>
  </div>
  <div className="doc-card">
    <h3>I want to build it</h3>
    <p className="doc-meta">Start now</p>
    <p>
      <a href="/contributing/development">Development setup</a> ·{" "}
      <a href="/contributing/testing">Testing strategy</a> ·{" "}
      <a href="/roadmap">Roadmap</a> ·{" "}
      <a href="/adr/index">Decision records</a>
    </p>
  </div>
  <div className="doc-card">
    <h3>I want the unvarnished numbers</h3>
    <p className="doc-meta">Required reading</p>
    <p>
      <a href="/reference/claims-and-evidence">Claims &amp; evidence</a> ·{" "}
      <a href="/architecture/performance-engineering">Performance engineering</a> ·{" "}
      <a href="/product/metrics">Metrics</a>
    </p>
  </div>
</div>

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
