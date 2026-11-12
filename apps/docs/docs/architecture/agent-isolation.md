---
id: agent-isolation
title: Agent isolation
sidebar_position: 8
description: Threat model, the three tiers, Landlock and seccomp enforcement, microVMs, and the PKU gap.
---

# Agent isolation

Aether runs coding and research agents. Agents execute generated code, read
repositories, and sometimes handle untrusted input. The isolation model is the
security core of the OS.

## Threat model

| Adversary | Capability | Goal |
| --- | --- | --- |
| Prompt-injected agent | Runs model-chosen tool calls | Read secrets, exfiltrate, persist |
| Compromised dependency in an agent image | Arbitrary code in the agent | Escalate to host |
| Malicious project file | Content that steers an agent | Trick the agent into a bad call |
| Hostile tenant (future) | Untrusted workload | Escape its VM |
| Local attacker | Unprivileged process | Read another agent's state |

Assumptions:

- The operator is trusted.
- The kernel and hardware are not compromised.
- The broker (`aetherd`) is trusted; it is small and audited.
- Model output is untrusted input. Always.

## The PKU gap

The original design used Intel Memory Protection Keys: allocate a protection key per
agent, switch domains with `WRPKRU` in ~10 ns, no syscall. **Kaby Lake has no PKU.**
Aether cannot implement that design on the reference machine.

What we do instead:

| Trust | Tier | Enforcement | Cross-agent memory |
| --- | --- | --- | --- |
| `trusted` | `in-process` | Rust ownership + review | N/A (chosen) |
| `untrusted` | `process-landlock` | Landlock + seccomp + UID + `no_new_privs` | Page tables |
| `hostile` | `microvm` | Firecracker + VFIO | Separate guest |

On hardware with PKU, the planner may add a PKU tier as defence in depth, but
untrusted agents still default to separate processes. PKU is not a syscall filter;
it does not stop an agent from calling `open()` on a path it can access.

## Tier 1: process + Landlock

The workhorse tier. One process per agent:

```text
fork/exec → setuid(agent-<id>) → setgroups([]) → prctl(NO_NEW_PRIVS)
         → landlock_create_ruleset (fs allowlist)
         → seccomp-BPF (syscall allowlist)
         → cgroup v2 (memory.max, cpu.weight, pids.max)
         → exec agent
```

**Filesystem.** Landlock rules are generated from capabilities:

```text
fs.read  fs:/work/aether          → read + execute on that subtree
fs.write fs:/work/aether/scratch  → read/write on the scratch subtree only
```

No path outside the allowlist is visible to the agent, even if it guesses the path.

**Syscalls.** A per-language profile (Rust/Python/Node) allows the minimum known
set. `execve` is allowed once (the agent binary), then blocked. `socket` is blocked
unless the agent holds `net`. `ptrace`, `bpf`, `perf_event_open`, `mount`, `unshare`
and `keyctl` are always blocked.

**Identity.** Each agent gets a UID from a reserved range. Even a Landlock bypass
fails to read another agent's files because the UID owns nothing else.

**Resources.** cgroup v2 enforces memory, CPU weight, process count and (optionally)
I/O. A timeout sends `SIGTERM`, then `SIGKILL` after a grace period.

## Tier 2: microVM

For hostile code, supply-chain-untrusted images, or workloads that need a different
kernel:

- Firecracker microVM with a minimal guest kernel.
- VFIO passthrough only if the workload needs a device (GPU, NIC).
- No shared filesystem; inputs are copied into a virtio-fs share, outputs are
  copied out through the broker.
- Boot from a snapshot; memory is pre-faulted to avoid page-fault jitter.
- Network is off by default; a virtio-net device is attached only with a `net`
  capability and an explicit egress policy.

VT-x and VT-d are required. If IOMMU is unavailable, the planner blocks the tier
rather than falling back to emulation.

## The planner

`planIsolation(spec, hardware)` is pure and returns a plan:

```ts
type IsolationPlan = {
  agentId: string;
  tier: IsolationTier;
  enforcement: "pku" | "landlock-seccomp" | "microvm-vfio" | "none";
  rationale: string[];
  warnings: string[];
  privileged: boolean;
  blocked: boolean;
  blockers: string[];
};
```

Rules:

1. Hostile → always microVM. Requires an image and VT-x/VT-d.
2. Untrusted requesting `in-process` → escalated to `process-landlock`, with a
   warning. No exceptions.
3. Untrusted may not request `agent.spawn`. No self-replication.
4. Missing Landlock → the process tier is blocked, not silently weakened.
5. Missing IOMMU or image → the microVM tier is blocked.
6. Every plan includes warnings for missing timeouts, oversized memory and
   whole-filesystem write grants.

The planner is tested against the reference hardware profile. If `pku` is flipped to
`true`, tests still pass (the policy does not depend on it); if `landlock` is
`false`, the process tier is blocked.

## Admission control

Isolation is necessary but not sufficient; six agents on four cores can starve the
shell. `AdmissionController` enforces:

| Resource | Capacity on the reference machine |
| --- | --- |
| CPU points (100 = one core) | 300 (3 usable cores) |
| Memory | configured, e.g. 8 GiB |
| Agents | 6 |

Requests that exceed capacity are queued, not overcommitted. The scheduler admits
the highest-priority agent that fits when a slot frees, avoiding head-of-line
blocking.

## Capability grant

An agent's requested capabilities are just requests. The broker issues tokens:

```ts
const cap = table.issue({
  subject: "agent-7f3a",
  scope: "fs.read",
  resource: "fs:/work/aether",
  ttlMs: 120_000,
  delegable: false,
});
```

Rules:

- Scope cannot be widened by delegation.
- Resource patterns can only narrow (`fs:/work/aether/src`).
- Expiry is capped by the parent's expiry.
- Revocation cascades to children.
- The token is presented on every operation; there is no ambient authority.

## Audit

Every spawn, grant, denial, revocation and exit appends to a hash chain:

```json
{
  "seq": 42,
  "at": 1762900000000,
  "actor": "aetherd",
  "action": "capability.denied",
  "subject": "agent-7f3a",
  "detail": "fs.read fs:/home/oliver/.ssh",
  "prevHash": "…",
  "hash": "…"
}
```

The daemon uses SHA-256; the browser demo uses 64-bit FNV so the same code runs
everywhere. Editing an entry breaks verification for every subsequent entry.

## What an agent still can do

Honesty matters:

- An agent with `net` and `infer` can send prompts to the model. Prompt injection
  remains a semantic risk the OS cannot solve.
- A trusted in-process agent can read everything the shell can.
- A microVM with VFIO GPU access controls the GPU; that is a large attack surface.
- Side channels (timing, cache) are not addressed by this design.

The mitigations are policy and audit, not magic.

## Verification plan

| Test | Phase |
| --- | --- |
| Planner unit tests (blocked/escalated plans) | ✅ now |
| Capability table tests (expiry, delegation, cascade) | ✅ now |
| Cross-thread ring isolation test | ✅ now |
| Real Landlock profile under a test harness | P1 |
| Seccomp profile fuzzing | P5 |
| Red-team suite: six agents attempting cross-read | P5 |
| MicroVM escape test corpus | P8 |

## Related

- [Collaboration](/architecture/collaboration) — the only sanctioned cross-agent path.
- [Security threat model](/ops/security-threat-model) — system-wide view.
- [ADR-0004](/adr/0004-isolation-without-pku) — the decision to abandon PKU.
