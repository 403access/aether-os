---
id: risks
title: Risks and mitigations
sidebar_position: 2
description: The risk register for building a single-target OS on four cores.
---

# Risks and mitigations

Scored qualitatively. Review at every phase gate; add a row when a decision creates
a new failure mode.

## Risk register

| ID | Risk | Likelihood | Impact | Mitigation | Phase |
| --- | --- | --- | --- | --- | --- |
| R1 | Kernel configuration misses a required driver and the machine does not boot | Medium | High | Keep a known-good generation; `nixos-rebuild build-vm`; live USB | P1 |
| R2 | GPU driver (i915/HD 630) cannot sustain 120 Hz with the compositor | Medium | High | P2 measures early; fallback to 60 Hz with documented penalty | P2 |
| R3 | Text shaping/rendering is slower than budget and makes the shell feel heavy | Medium | Medium | Choose cosmic-text/swash, cache atlases, measure glyph cost | P2 |
| R4 | Landlock/seccomp profiles break real agent workloads | High | Medium | Start permissive per runtime, tighten with test corpus; audit denials | P5 |
| R5 | WebKitGTK integration consumes more memory than the idle budget allows | Medium | Medium | Browser is not part of idle state; measure per-tab RSS and document | P6 |
| R6 | VA-API zero-copy path is unavailable on the chosen GPU | Medium | Medium | Software encode fallback with latency badge; NVENC path if dGPU | P7 |
| R7 | Four cores cannot sustain six agents plus UI | High | Medium | Admission control enforces 6; show queueing; measure burst behaviour | P5 |
| R8 | Remote display latency exceeds 40 ms on Wi-Fi | High | Low | Recommend wired/VPN; adaptive bitrate; fallback ladder | P7 |
| R9 | Compiler tuning yields < 5% and adds build complexity | Medium | Low | P8 measures; revert if below threshold | P8 |
| R10 | Scope creep toward portability or multi-user | High | High | PRD non-goals; ADR required to change scope | all |
| R11 | Design drift between shell, docs and marketing | Medium | Medium | Single token source; parity tests; vendored design system | P3 |
| R12 | Speculative numbers leak into marketing as facts | Medium | High | Claims & evidence page; review checklist; budgets labelled | all |
| R13 | Single maintainer bus factor | High | High | Documentation-first; reproducible builds; ADRs; tested core | all |
| R14 | Supply-chain compromise in a dependency | Low | High | Pinned inputs, minimal deps, no postinstall scripts, sandboxed agents | all |
| R15 | MicroVM complexity consumes the performance budget | Medium | Medium | Defer to P8; measure VM tax before shipping | P8 |

## Assumption failures

These are the assumptions that, if wrong, invalidate the architecture. Each has a
test or a measurement that would reveal it.

| Assumption | If false | Detection |
| --- | --- | --- |
| Kaby Lake has no PKU | Isolation plan would be unnecessarily conservative | `lscpu` flags; profile test |
| Landlock is available | Process tier is blocked | `/sys/kernel/security/lsm` check |
| VT-d works | MicroVM tier is blocked | `dmesg` IOMMU; planner blocker |
| QuickSync encodes H.264/HEVC | Remote display falls back to software | `vainfo`; P7 harness |
| 4 cores suffice for UI + 6 agents | Fewer agents, queueing | Scheduler telemetry; red-team load |
| In-memory index fits in RAM | Snapshot eviction, slower queries | `stats()`; memory sampling |
| WebKitGTK works on Wayland | Browser fallback to XWayland (not desired) | P6 spike |

## Off-ramps

A gate failure does not always mean “work harder”. Documented off-ramps:

- **P2 frame budget fails at 120 Hz** → ship 90 Hz or 60 Hz, reduce effects, revisit
  renderer path. The OS still works.
- **P5 isolation breaks workloads** → keep the process tier but restrict which
  languages are supported until profiles stabilize.
- **P7 latency fails** → ship tier 2 (WebSocket + fMP4) and label the latency
  honestly. Do not claim sub-40 ms.
- **P8 PGO gains < 5%** → drop PGO, keep ThinLTO, document the measurement.

An off-ramp is a product decision, not a failure. It must be recorded in an ADR.

## Review cadence

| Moment | Question |
| --- | --- |
| Phase kickoff | Which risks does this phase retire? |
| Phase gate | Did the gate pass? Which risks remain? |
| Every ADR | Does this create a new risk row? |
| Every release | Are any mitigations now unnecessary? |
