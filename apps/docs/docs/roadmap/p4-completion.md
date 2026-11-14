---
id: p4-completion
title: P4 completion plan
sidebar_position: 4
description: Audio phase tasks, acceptance criteria, evidence and commit boundaries.
---

# P4 completion plan

**Goal.** Every input and output route is first-class: PipeWire/WirePlumber policy,
a compositor audio overlay, latency presets, xrun instrumentation and a microphone
capability gate.

P3 is closed ([P3 completion](/roadmap/p3-completion)); this page is the executable
plan for P4. Each task is a separate commit and updates this ledger.

## Gate

- Route switch **< 100 ms** across analog, USB and Bluetooth.
- **Zero xruns** in a 10-minute 64-frame stress test on the reference machine.
- Agent microphone access is **denied without an `audio.capture` capability**.

The first two are target measurements. Dev-host and VM checks prove the code path
and record the harness; they are not quoted as target results.

## Tasks

| ID | Task | Acceptance | Evidence |
| --- | --- | --- | --- |
| P4-T1 | `aether-audio` core: route model, `pw-dump`/`wpctl` adapter, switching and latency presets | Fixtures parse outputs/inputs and transports; switch calls the adapter; presets map to PipeWire metadata; unit tests pass | crate tests + `artifacts/p4/audio-list.json` |
| P4-T2 | Compositor audio overlay: outputs/inputs, active route, presets, input meters, `Super+A` | Overlay opens/closes, selection and activation switch the default route, preset chips cycle; fixture smoke captures frames | `artifacts/smithay/audio.png`, `audio-preset.png`, `artifacts/p4/audio-smoke.log` |
| P4-T3 | Microphone capability gate: `audio.capture` scope, grant/deny policy, audit | No capability → denied; explicit capability → allowed; denial is audited; protocol rejects malformed scopes | `packages/agents` tests, `aether-audio` tests |
| P4-T4 | NixOS policy: PipeWire quantum profiles, WirePlumber defaults, Bluetooth codecs, packaged daemon/CLI | Config evaluates; target enables Bluetooth and the Aether audio profile; docs describe the routes | `nix flake check --no-build` log, NixOS eval |
| P4-T5 | Measurement harness: route-switch benchmark and 10-minute 64-frame xrun stress | Harness emits NDJSON; a dry run works without audio hardware; target run is documented as pending | `scripts/audio-stress.sh`, `artifacts/p4/audio-stress-*.ndjson` |
| P4-T6 | Verification and reconciliation: tests, smokes, claims and docs | All checks pass; claims/evidence and roadmap carry the real labels; no budget is presented as measured | ledger + CI run |

## Sequencing

T1 → T2 → T3 → T4 → T5 → T6. T2 consumes the T1 model; T3 is independent after the
route model exists; T4 can land alongside T2; T5 exercises T1/T4; T6 is last.

## Verification

| Check | Result |
| --- | --- |
| `aether-audio` unit tests | pending |
| Shell audio-overlay tests | pending |
| Audio fixture smoke | pending |
| Agent mic capability tests | pending |
| `bun run verify` | pending |
| `nix flake check --no-build` | pending |
| Route switch on target | budget, not measured |
| 10-minute 64-frame xruns | budget, not measured |

## Non-goals

- A custom audio server; PipeWire owns the graph.
- A UI for building PipeWire graphs.
- Bluetooth codec implementation; BlueZ/WirePlumber negotiate.
- Per-app DSP chains; filters remain PipeWire modules.

## Execution ledger

| Task | Status | Commit |
| --- | --- | --- |
| Plan | ✅ | this document |
| P4-T1 | ⏳ | |
| P4-T2 | ⏳ | |
| P4-T3 | ⏳ | |
| P4-T4 | ⏳ | |
| P4-T5 | ⏳ | |
| P4-T6 | ⏳ | |
