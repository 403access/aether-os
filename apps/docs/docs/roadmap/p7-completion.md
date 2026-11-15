---
id: p7-completion
title: P7 completion plan
sidebar_position: 5
description: Remote-display tasks, acceptance criteria, evidence and commit boundaries.
---

# P7 completion plan

**Goal.** The desktop in a browser tab at interactive latency: DMA-BUF capture,
low-latency encode, WebRTC/DTLS transport, a WebCodecs canvas, a virtual-seat input
path and a fallback ladder.

P4 is code-complete ([P4 completion](/roadmap/p4-completion)); this page is the
executable plan for P7. Each task is a separate commit and updates this ledger.

## Gate

- **Glass-to-glass < 40 ms** on a wired LAN, measured with a timestamped overlay.
- **No CPU pixel copies** in the encode path (`perf` + DMA-BUF traces).
- **p99 frame interval < 1.5×** the negotiated frame target.

The compositor capture binding and the glass-to-glass measurement are target-only.
This repository has no x86_64 reference machine, so the plan ships the pipeline,
the harness and the browser client, and labels the gate as a **budget** until the
run happens on the 7700K. Nothing below presents a dev-host number as the gate.

## Tasks

| ID | Task | Acceptance | Evidence |
| --- | --- | --- | --- |
| P7-T1 | `aether-remote` capture model: DMA-BUF planes, formats, damage, CPU-copy audit, synthetic source | Planes validate; a DMA-BUF frame records zero copies; a software frame is counted and labelled | crate tests + `artifacts/p7/capture.json` |
| P7-T2 | Low-latency H.264 encode and capability ladder | Real Annex-B H.264 from RGBA; baseline profile, no B-frames, no lookahead; hardware selected when advertised otherwise software | crate tests + `artifacts/p7/encode.ndjson` |
| P7-T3 | Session security and signaling: pairing codes, tokens, DTLS fingerprints, SDP messages, audit chain | Code is one-shot and expires; token is bound to code/session; malformed/old-version messages are rejected; audit verifies | crate tests + `artifacts/p7/pair.json` |
| P7-T4 | Virtual seat input and WebRTC transport | Scancodes are range-checked, motion coalesced and rate-limited; two str0m peers reach DTLS-connected state and open the input channel | crate tests + `artifacts/p7/webrtc.json` |
| P7-T5 | Pipeline, rate control and harness | Last-frame-wins slot never queues; p50/p99 latency ledger; `aether-remote bench` emits NDJSON; no-queue and interval budgets asserted | `artifacts/p7/bench.ndjson`, `aether-remote bench` exit code |
| P7-T6 | Browser client `@aether/remote`: capability probe, tier selection, WebCodecs/canvas adapter, input batching, stats | Capability matrix selects the right tier; client state machine survives fallback and reconnect; input batches coalesce motion | `packages/remote` tests + `artifacts/p7/client.json` |
| P7-T7 | Web viewer surface and screenshots | The viewer shows pairing, tier, latency and the recording indicator; Playwright covers pairing + streaming states | `e2e/remote.spec.ts`, `remote-viewer.png` |
| P7-T8 | Compositor integration: remote overlay, status chip, `Super+R`, preview renderer | Overlay opens/closes, actions cycle input/clipboard, status reports the active session; preview captures both states | `aether-shell` tests, `artifacts/smithay/remote.png`, `remote-idle.png` |
| P7-T9 | NixOS service, firewall and docs verification | Config evaluates; service is opt-in and local-only; claims/roadmap/README/CHANGELOG agree | `nix flake check --no-build` log, docs build |

## Sequencing

T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9. T2 consumes T1's frames; T4 consumes
T3's session; T5 ties them together; T6/T7 are the browser half; T8 exposes the
session in the shell; T9 packages and reconciles.

## Non-goals

- A public relay or TURN service. The session is LAN/VPN-local by default.
- Agent-started sessions. `remote.host` is a host capability, never delegated.
- Clipboard by default. It is opt-in per session and audited.
- AF_XDP unless the committed latency harness shows the kernel stack dominates.
- A second capture implementation for X11; the compositor is the capture source.

## Verification

| Check | Result |
| --- | --- |
| `aether-remote` unit/integration tests | pending |
| str0m loopback handshake | pending |
| `packages/remote` tests | pending |
| Web viewer e2e | pending |
| Shell remote-overlay tests | pending |
| `bun run verify` | pending |
| `cargo test --manifest-path os/compositor/Cargo.toml` | pending |
| Glass-to-glass < 40 ms on target | **budget**; harness committed, target run pending |
| No CPU pixel copies on target | **budget**; audit + DMA-BUF plan committed, target run pending |
| p99 frame interval < 1.5× target | **budget**; dev-host harness records, target run pending |

## Execution ledger

| Task | Status | Commit |
| --- | --- | --- |
| Plan | ✅ this document | roadmap and sidebar |
| P7-T1 | ⏳ | capture model |
| P7-T2 | ⏳ | encode |
| P7-T3 | ⏳ | session and signaling |
| P7-T4 | ⏳ | input and WebRTC |
| P7-T5 | ⏳ | pipeline and harness |
| P7-T6 | ⏳ | browser client |
| P7-T7 | ⏳ | web viewer |
| P7-T8 | ⏳ | compositor integration |
| P7-T9 | ⏳ | packaging and reconciliation |
