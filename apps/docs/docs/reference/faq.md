---
id: faq
title: FAQ
sidebar_position: 3
description: Straight answers about feasibility, hardware, the stack and what is actually built.
---

# FAQ

## Is this a real OS?

It is a real project with a real architecture, a tested core, an installable
NixOS base and a Rust compositor core. v0.1 ships an installer ISO, a disko disk
layout, an install script and a separate aarch64 VM that boots on this Mac. The
compositor UI and daemons are P2/P3. It is an **installable base**, not the
finished desktop; the [intro](/) status table is the honest inventory.

## Can I try it without the i7-7700K?

Yes, as a development VM: `./scripts/vm-build.sh && ./scripts/vm-run.sh`. It is a
separate aarch64 target and not a supported install. See
[boot the development VM](/getting-started/virtual-machine).

## Why not just use Omarchy / Arch + Hyprland?

Omarchy is a curated, portable Arch desktop. Aether is the opposite bet: one
machine, no portability layer, a custom compositor and a custom agent-isolation
model. The interesting work is the integration, not the desktop shell. If you want a
ready-made keyboard-driven Arch desktop, Omarchy is the better choice. If you want
to own the display, IPC and trust boundary, Aether is the project.

## Why Rust?

Rust and C/C++ compile to the same class of machine code. Rust wins here because:

- Memory safety without a GC, so no latency spikes.
- Fearless concurrency for the compositor, indexer and audio paths.
- A strong ecosystem for Wayland (Smithay), rendering (wgpu), and async I/O.

The browser is still C++ (WebKit), the kernel is C, and mpv is C. Rust is the glue
and the hot path where we control it, not a religion.

## Why Bun, TypeScript and React for the web surfaces?

The web surfaces are not the OS. They need to ship fast, share types with the
domain model, and be testable. Bun gives one toolchain for install/test/build; React
gives the component library; Docusaurus gives the docs. The OS itself is Rust, Nix
and C.

## Will it run on my laptop?

No. The kernel, compiler flags and capacity budgets assume the i7-7700K. A different
CPU is a fork, not a configuration flag. The hardware dossier explains exactly what
would change.

## How many agents can it run?

Six concurrent CPU-bound workers is the enforced ceiling on 4 cores / 8 threads,
with one core reserved for the shell and audio. The “25–40 agents” figure from the
original discussion assumed a much larger machine. See
[hardware target](/architecture/hardware-target#capacity-math).

## How are agents isolated without PKU?

Kaby Lake has no Memory Protection Keys. Untrusted agents run in a **separate
process** with Landlock, seccomp-BPF, a per-agent UID and `no_new_privs`. Hostile
code runs in a microVM. The cost is a context switch; the benefit is that the
isolation exists. See [agent isolation](/architecture/agent-isolation).

## Does that mean the 10 ns isolation claim is wrong?

For this CPU, yes. We do not repeat it. On hardware with PKU, an in-process tier is
possible as defence in depth, but it still would not filter syscalls.

## Where does the AI run?

The chat surface talks to a local DeepSeek endpoint over a Unix socket (or the
hosted API, if configured). Aether does not ship model weights. The OS guarantees
the transport and the capability model, not the model.

## Can I access the desktop from a browser?

That is P7 and not implemented. The intended pipeline is DMA-BUF → hardware encode →
WebRTC → WebCodecs, with a documented fallback ladder. The current repo has no
streaming code; claiming otherwise would violate the evidence rule.

## Why XFS?

Extent allocation, dynamic inodes and strong parallel I/O without copy-on-write
write amplification. ext4 is used for scratch, Btrfs and ZFS are rejected because
their memory and write behaviour conflict with the idle and latency budgets.

## Why is the idle RAM budget 200 MB?

It is a budget derived from the component estimates, not a measurement. P1 replaces
it with a measured value. The point is to have a number small enough to force
discipline.

## What about accessibility?

Keyboard-first, visible focus, reduced motion and contrast are implemented in the
web surfaces. Screen-reader support (AT-SPI) is **not** implemented and is listed as
a known gap. We do not claim accessibility parity with GNOME.

## Is the design system really reused?

Yes. The Water tokens, components and water effects are vendored from the Data Cloud
Compare project (`/Users/olivermolnar/Developer/jottacloud-comparison`), whose
explorer runs at `http://localhost:8787/`. The explorer was rebuilt as `FilePane` in
React. The provenance and divergence policy are in
[design system and explorer source](/roadmap/design-system-and-explorer).

## Can I contribute?

Yes, within scope. Read [development](/contributing/development) and
[testing](/contributing/testing). The fastest useful contributions are benchmarks on
the reference hardware, seccomp/Landlock profiles, and docs corrections.

## Why so much documentation?

Because a single-maintainer OS has a bus factor of one. The documentation is the
continuity plan: decisions, budgets, failure modes and gates are written down so the
project survives a context switch.
