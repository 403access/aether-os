---
id: security-threat-model
title: Security threat model
sidebar_position: 3
description: Assets, adversaries, controls and residual risk for a single-user OS that runs generated code.
---

# Security threat model

Aether runs code chosen by a language model. That makes the agent boundary the most
important security property in the system.

## Assets

| Asset | Why it matters |
| --- | --- |
| SSH keys, API tokens, browser profile | Direct account compromise |
| Source repositories | Intellectual property; supply-chain pivot |
| Inference endpoint | Cost, data exfiltration |
| Audit log | Detection and accountability |
| Session/desktop | Input injection, screen capture |
| Boot integrity | Everything above |

## Adversaries

| Adversary | Access | Motivation |
| --- | --- | --- |
| Prompt-injected agent | Tool calls within its grants | Exfiltrate, persist, pivot |
| Compromised dependency | Code inside an agent or app | Escalate to host |
| Malicious website | Browser renderer | Break the sandbox |
| Malicious media file | Decoder | Code execution |
| Local unprivileged process | Shell in the same user | Read another agent's state |
| Network attacker | Remote display traffic | View/inject input |

Out of scope: physical attackers, compromised firmware, malicious kernel modules,
hardware side channels, and a malicious operator.

## Trust boundaries

```text
kernel  ──────────────  trusted
aetherd ──────────────  trusted, small, audited
aether-shell ─────────  trusted for display/input
browser/media ────────  untrusted, sandboxed
agents ───────────────  untrusted/hostile, planned
model output ─────────  untrusted input, always
```

## Controls by threat

| Threat | Control |
| --- | --- |
| Agent reads secrets | Landlock allowlist, per-agent UID, capability per path |
| Agent reads another agent | Separate address space + UID; no shared mapping unless brokered |
| Agent exfiltrates | `net` denied by default; egress policy for microVMs |
| Agent persists | Ephemeral home, no autostart, no cron, no systemd unit creation |
| Agent spawns agents | `agent.spawn` blocked for untrusted; broker-only |
| Agent escapes seccomp | Minimal allowlist; denied syscalls kill the process |
| Browser escapes | WebKit process model + bwrap + empty home + no device access |
| Media decoder exploit | Sandboxed client, read-only file, no network |
| Remote viewer | DTLS-SRTP + per-session token + pairing code; no public listener |
| Input injection | Scancodes only; validated; never shell commands |
| Audit tampering | Hash chain; append-only; verification tool |
| Supply chain | Pinned lockfiles, minimal dependencies, no postinstall scripts, SBOM later |

## Agent-specific analysis

### Confused deputy

The broker issues capabilities; an agent cannot ask for a capability it was not
granted, and cannot widen a delegated one. Resource patterns are hierarchical and
boundary-aware (`fs:/work/a` does not match `fs:/work/ab`).

### Capability leakage

Tokens are bound to a subject and checked on every operation. A token copied to
another process fails the subject check. Tokens expire and are revoked on channel
close. The ring stores a hash, not the token.

### Denial of service

- cgroup memory and pids limits.
- Admission control prevents CPU oversubscription.
- Timeouts terminate stalled agents.
- The shell reserves one core and is not in an agent's cgroup.

### Side channels

Not addressed: timing, cache, power. Mitigations would require partitioning L3 and
disabling SMT, which conflicts with the performance goal. This is a documented
residual risk.

### Prompt injection

The OS cannot distinguish a legitimate tool call from an injected one. Mitigations
are structural: least privilege, no ambient network, narrow filesystem grants,
timeouts, audit. A successful injection should be **bounded**, not impossible.

## Residual risks

| Risk | Severity | Accepted because |
| --- | --- | --- |
| Prompt injection causes a bad but permitted action | High | Semantic problem; mitigated by least privilege and audit |
| Timing/cache side channel between agents | Medium | Requires hardware partitioning that defeats the performance target |
| WebKit 0-day | High | Sandbox limits blast radius; engine updates are pinned and tested |
| GPU driver 0-day | High | GPU is required; microVM with VFIO isolates hostile workloads |
| Trusted in-process agent compromise | High | Trust is explicit; such agents are part of the TCB |
| Physical access | High | Out of scope; disk encryption and lock screen are the controls |

## Security review checklist

For every new capability, syscall, or daemon:

- [ ] What is the asset? What can this access?
- [ ] Is the grant least-privilege and resource-scoped?
- [ ] Does denial fail closed?
- [ ] Is the decision audited?
- [ ] Can it be revoked while running?
- [ ] Does it add a network listener? If so, why?
- [ ] Does it widen the kernel attack surface?
- [ ] Is there a test that proves the denial?

## Incident response

1. **Contain.** Kill the agent cgroup; revoke its capabilities; close its channels.
2. **Preserve.** Export the audit chain and the agent's workspace before deletion.
3. **Assess.** Which capabilities were used? Which paths were read?
4. **Rotate.** Keys and tokens the agent could reach.
5. **Record.** Add an entry to the risk register; adjust policy or profiles.
6. **Test.** Add a regression test reproducing the path.

There is no central incident team. The operator performs these steps; the tooling
exports everything needed.

## Phase gates

| Gate | Proof |
| --- | --- |
| P5 | Six agents, cross-read suite, zero successes |
| P5 | Seccomp profile corpus passes for Rust/Python/Node |
| P7 | Remote session requires a valid token; input is scancode-only |
| P8 | MicroVM escape corpus; capability fuzzing; audit verification |
