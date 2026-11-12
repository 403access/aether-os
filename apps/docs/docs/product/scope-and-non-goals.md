---
id: scope-and-non-goals
title: Scope and non-goals
sidebar_position: 3
description: What ships in v1.0, what waits, and what is explicitly refused.
---

# Scope and non-goals

Scope discipline is the product. A single-target OS fails when it starts to
resemble a general-purpose distribution.

## In scope for v1.0

| Area | Included |
| --- | --- |
| Base | NixOS minimal, custom Kaby Lake kernel, XFS root, systemd, PipeWire |
| Display | Smithay compositor, tiling layout, 120 Hz pacing, floating overlays |
| Shell | Palette, chat, explorer, status bar, notification toasts |
| Search | Apps, files, content, commands, agents, settings; in-memory BM25 + fuzzy |
| Agents | Isolation planner, admission control, capability broker, audit chain |
| Apps | WebKitGTK browser shell, libmpv player, two-pane explorer |
| Audio | Input/output enumeration, switching, per-app volume, Bluetooth |
| Ops | Repro builds, CI, performance gate, local audit export |
| Docs | This site, ADRs, budgets and measurement methods |

## Deferred

| Feature | Why it waits | Earliest phase |
| --- | --- | --- |
| Remote display (WebRTC/WebCodecs) | Needs a stable compositor and capture path | P7 |
| AF_XDP transport | Only justified if measurement shows kernel networking is the bottleneck | P7 |
| MicroVM agent tier | Useful before hostile workloads; expensive to operate | P8 |
| PGO + ThinLTO pipeline | Baseline must exist before profile-guided tuning | P8 |
| Multi-output VRR tuning | Requires hardware matrix and measurement | P8 |
| Snapshot/restore of agent VMs | Depends on microVM tier | P8 |

## Explicit non-goals

- **Portability.** No generic kernel, no fallback renderer, no “works on most
  laptops” guarantee. A different CPU is a fork.
- **Multi-user.** One seat, one user, one home directory. No login manager
  beyond a lock screen.
- **Enterprise management.** No MDM, SSO, policy server or remote wipe.
- **Cloud sync.** NixOS generations are local. The archive pane may mount a
  network filesystem, but Aether does not ship a sync service.
- **Telemetry.** Zero. Crash reports are local files the user chooses to share.
- **Third-party app store.** Applications are installed declaratively or compiled
  into the image. Agents are the extension mechanism.
- **Windows/macOS compatibility.** No Wine, no VMs for legacy apps in v1.0.
- **Accessibility parity with GNOME.** Keyboard-first and high-contrast themes are
  required; screen-reader integration is a known gap, not a shipped claim.
- **Battery life on laptops.** The reference machine is a desktop.
- **Gaming.** No Steam, no Proton, no controller stack in v1.0.

## Scope rules for contributors

1. If a feature only exists to support another machine, it is out of scope.
2. If a feature adds a background process, it needs a measured justification and
   an idle-cost budget.
3. If a feature adds a runtime dependency to the shell, it needs an ADR.
4. If a claim cannot be measured this phase, it must be labelled a target in the
   docs and the marketing copy.
5. If an agent needs a new capability, the capability is proposed as a scope with a
   resource pattern, not as a boolean.

## The “would we delete this?” test

Before adding a layer, ask: **if this disappeared, would the machine get faster
and simpler?** If yes, it does not belong. The exceptions are the Linux kernel,
the browser engine and PipeWire — replacing them is a different project with a
different risk profile.
