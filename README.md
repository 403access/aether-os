# Aether OS

A **single-target** Linux workstation OS built for exactly one machine: an Intel
Core i7-7700K (Kaby Lake, 4c/8t) with a discrete or integrated GPU. No
portability layer, no generic desktop environment, no telemetry. Every subsystem
is chosen for deterministic latency and a small resident footprint, then tuned
at compile time for this CPU.

The visual/UX thesis: **one fast surface** — a compositor that also renders the
command palette, AI chat, and system chrome in a single scene graph, backed by an
in-memory index that answers in single-digit milliseconds.

```text
  ┌──────────────────────────────────────────────────────────────┐
  │  aether-shell (Rust): compositor + scene graph + chrome      │
  │  wgpu/Vulkan · damage-tracked · 120 Hz frame pacing          │
  ├──────────────────────────────────────────────────────────────┤
  │  palette · chat · files · media · browser (thin native apps) │
  ├──────────────────────────────────────────────────────────────┤
  │  aetherd: supervisor, capability broker, agent scheduler     │
  │  aether-ipc: memfd SPSC rings · aether-index: BM25 + fuzzy    │
  ├──────────────────────────────────────────────────────────────┤
  │  PipeWire/WirePlumber · XFS root · NixOS + custom 7700K kernel│
  └──────────────────────────────────────────────────────────────┘
```

## What exists today

This repository is the **installable base release (v0.1 "Kestrel")**. It is a
working monorepo: the product surfaces, design system, indexing engine, IPC
protocol, capability model, bootable NixOS system, live installer ISO, disk layout
and install guide all exist as real, tested code. The compositor UI and daemons are
the next phase; first boot lands in a console, not the finished desktop.

| Area | State | Where |
| --- | --- | --- |
| Marketing site + live palette demo | ✅ built, 11 e2e specs | `apps/web` |
| Documentation site | ✅ built | `apps/docs` |
| Search/index engine (BM25, fuzzy, prefix) | ✅ 59 tests | `packages/indexer` |
| Shared-memory SPSC ring + capabilities | ✅ 28 tests (incl. cross-thread) | `packages/ipc` |
| Agent isolation policy + scheduling | ✅ 39 tests | `packages/agents` |
| Command palette state machine | ✅ 27 tests | `packages/palette` |
| React UI kit | ✅ 17 DOM tests | `packages/ui` |
| Benchmark harness + regression gate | ✅ baseline committed | `packages/bench` |
| NixOS system + installer ISO | ✅ evaluates (`nix flake check`); not booted in CI | `os/nixos` |
| disko disk layout + install script | ✅ written, evaluation-checked | `os/nixos/disko.nix`, `scripts/install.sh` |
| Compositor scene graph + frame scheduler | ✅ 35 Rust tests | `os/compositor` |
| Compositor UI (`aether-shell`) | ⏳ roadmap P2 | — |
| WebRTC remote display | ⏳ roadmap P7 | — |

> **On numbers:** the docs distinguish *measured*, *budgeted*, and *speculative*
> figures. Idle-RAM and latency numbers in the architecture discussion are
> **budgets and targets**, not results — see
> `apps/docs/docs/reference/claims-and-evidence.md`.

## Quickstart

Requires [Bun](https://bun.sh) ≥ 1.3, Node ≥ 20, and Rust (stable) for the
compositor core.

```bash
bun install
bun run dev:web          # marketing site  → http://localhost:4321
bun run dev:docs         # documentation   → http://localhost:4322/docs/

bun run test:unit        # 199 package tests (bun test)
bun run test:e2e         # 11 Playwright specs against the assembled build
bun run typecheck        # strict TS across the monorepo
bun run os:test          # 35 Rust compositor-core tests
bun run bench            # benchmarks; `bun run bench:check` gates regressions
bun run build            # assembles a single deployable tree in apps/web/dist
```

`bun run build` produces one artifact: the marketing site at `/` and the docs at
`/docs/`, which is exactly what the GitHub Pages workflow publishes.

## Install on the reference machine

Aether installs from any current NixOS minimal ISO, or from the included installer
ISO. The target machine must be UEFI x86_64 and **the whole target disk is wiped**.

```bash
# on a live NixOS ISO, in a clone of this repo:
#   edit os/nixos/disk.nix and set your NVMe device
sudo ./scripts/install.sh /dev/nvme0n1
```

Then reboot and sign in as `oliver` (initial password `aether`). Full guide,
including firmware settings, manual disko/install commands, recovery and a
hardware checklist:

- [`apps/docs/docs/getting-started/install.md`](apps/docs/docs/getting-started/install.md)
- [`apps/docs/docs/getting-started/first-boot.md`](apps/docs/docs/getting-started/first-boot.md)
- [`apps/docs/docs/getting-started/hardware.md`](apps/docs/docs/getting-started/hardware.md)

To build the live installer ISO (requires an x86_64 Linux host with Nix):

```bash
nix build .#iso
```

## Repository layout

```text
apps/web              marketing site, live palette demo (Vite + React)
apps/docs             Docusaurus: architecture, PRD, roadmap, ADRs, ops
packages/protocol     shared domain types + runtime validation
packages/indexer      tokenizer, inverted index, BM25, fuzzy + prefix search
packages/ipc          memfd-style SPSC ring buffer + capability table
packages/agents       isolation tiers, admission control, collaboration bus
packages/palette      headless command-palette state machine
packages/ui           React design system (palette, chat, chrome)
packages/bench        benchmark runner + regression gate (--check)
os/nixos              flake, kernel, hardware, disko layout, installer, services
os/compositor         Rust workspace: pure scene/layout/pacing core
scripts               assemble, publish, install, static server
e2e                   Playwright specs for web + docs
.github/workflows     CI, Pages deploy, perf gate
```

## The five commitments

1. **Single target.** `-C target-cpu=native`, `localmodconfig`, hardware facts
   encoded in config. Portability is a non-goal, not an accident.
2. **No idle tax.** No background indexer polling, no Electron, no Qt/GTK
   runtime in the shell. Idle budget: **≤ 200 MB RAM, < 0.5% CPU**.
3. **Explicit trust.** Agents are untrusted by default, isolated with
   Landlock + seccomp + per-agent UID (PKU is unavailable on Kaby Lake), and
   collaborate only through capability-gated shared memory.
4. **Measured, not asserted.** Every performance claim has a benchmark, a
   budget, and a CI gate — or is labelled speculative.
5. **Legible.** Every architectural choice has an ADR; every subsystem has a
   spec with non-goals and failure modes.

## Documentation

Start at `apps/docs/docs/intro.md`, or run the docs site. Highlights:

- [Install on the i7-7700K](apps/docs/docs/getting-started/install.md)
- [First boot checklist](apps/docs/docs/getting-started/first-boot.md)
- [Hardware notes](apps/docs/docs/getting-started/hardware.md)
- [Product requirements](apps/docs/docs/product/prd.md)
- [Architecture overview](apps/docs/docs/architecture/overview.md)
- [Hardware dossier: i7-7700K](apps/docs/docs/architecture/hardware-target.md)
- [Agent isolation without PKU](apps/docs/docs/architecture/agent-isolation.md)
- [Roadmap](apps/docs/docs/roadmap/roadmap.md)
- [Claims & evidence](apps/docs/docs/reference/claims-and-evidence.md)

The visual language and the explorer GUI are adopted from the Data Cloud Compare
project (`/Users/olivermolnar/Developer/jottacloud-comparison`, live reference at
`http://localhost:8787/`). The tokens and components are vendored in
`packages/design-system`; provenance and the adoption plan are recorded in
[design system and explorer source](apps/docs/docs/roadmap/design-system-and-explorer.md).

## Publishing to GitHub

GitHub CLI is not authenticated in this environment. After `gh auth login`:

```bash
./scripts/publish.sh            # creates the repo, pushes main, enables Pages
```

The Pages deployment builds both sites and serves docs under `/docs/`.

## License

MIT © Aether OS contributors.
