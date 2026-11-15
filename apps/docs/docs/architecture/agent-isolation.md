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

**Syscalls.** The shipped profile is a seccomp-BPF deny-list, not a per-language
allowlist yet. It removes the syscalls that escape the process boundary:
`ptrace`, `bpf`, `perf_event_open`, `userfaultfd`, `mount`, `pivot_root`,
`unshare`, `setns`, `keyctl`, module loading, kexec, `open_by_handle_at`,
`process_vm_*` and io_uring. The socket family is denied unless the agent holds
`net`. `execve` remains available because the sandbox applies before the first
exec; Landlock's execute allowlist is what limits subsequent executables. A
per-language allowlist is future work.

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

## Implementation: `aether-sandbox`

The process tier is implemented in `os/compositor/crates/aether-sandbox`:

```bash
aether-sandbox check
aether-sandbox plan --read /work/aether --write /work/aether/scratch
aether-sandbox run --id build --label "Build agent" \
  --read /work/aether --write /work/aether/scratch --log /work/aether/build.log \
  -- /usr/bin/python3 /work/aether/agent.py
```

What happens between fork and exec:

1. `setgroups([])` then `setgid`/`setuid` to a deterministic UID in 4001–4095
   (the production supervisor must run with root/CAP_SETUID).
2. `prctl(PR_SET_NO_NEW_PRIVS, 1)`.
3. `RLIMIT_AS` (memory), `RLIMIT_NPROC` (256) and `RLIMIT_CPU` (timeout).
4. Landlock read/execute rules for runtime paths and agent inputs; read/write
   rules only for the scratch subtree.
5. seccomp-BPF deny-list, including the socket family unless `--net`.
6. `exec` the agent binary directly.

There is no wrapper process and no shell, so the per-agent cost is one ruleset
installation, not an extra context switch or proxy. `--no-uid` exists for CI and
development; production must omit it. cgroup v2 limits (`memory.max`,
`cpu.weight`, `pids.max`) are applied by `spawn_in_cgroup` when the supervisor
passes a writable cgroup root.

The implementation is packaged as `.#aether-sandbox` on the target and live
images. Linux integration tests prove an allowed file is readable, an
unallowlisted secret is denied, and an out-of-scratch write is denied.

### Agent sandboxes surface

`aether-sandbox run` writes one record per child to `/run/aether/sandboxes`
(shared root registry; override: `AETHER_SANDBOX_REGISTRY`) before it waits, and
removes it on exit. A per-sandbox Unix socket lets the unprivileged shell request
a stop even though the child runs under a reserved UID. `Super+B` renders that registry with the live
process table: label, id, trust, tier, PID, UID, CPU/RSS, enforcement, limits,
capabilities, read/write paths, warnings and log path. `K` stops the selected
sandbox, `Shift+K` force-stops it, `L` opens its log, and `C` prunes records left
by a killed launcher. A record is advisory; a missing PID is shown as `stale`,
never silently treated as running.

The registry is not a supervisor. It records what the CLI actually started; the
future `aetherd` remains responsible for admission, capability issuance and
cross-agent audit.

![Agent sandboxes surface with two running agents and a stale record](/img/smithay-agent-sandboxes.png)

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
| Real Landlock profile under a test harness | ✅ `aether-sandbox` tests |
| Seccomp denies `unshare` / network syscalls | ✅ now (`aether-sandbox`) |
| Seccomp profile fuzzing | P5 |
| Red-team suite: six agents attempting cross-read | P5 |
| MicroVM escape test corpus | P8 |

## Related

- [Collaboration](/architecture/collaboration) — the only sanctioned cross-agent path.
- [Security threat model](/ops/security-threat-model) — system-wide view.
- [ADR-0004](/adr/0004-isolation-without-pku) — the decision to abandon PKU.
