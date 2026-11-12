---
id: observability
title: Observability
sidebar_position: 2
description: Local-only metrics, logs, traces and the audit chain. There is no telemetry.
---

# Observability

Aether has no telemetry. Observability means the user can see what the machine is
doing, locally, without a network connection.

## Principles

1. **Local by default.** Metrics live in memory and local files.
2. **Bounded.** Counters and ring buffers, not unbounded logs.
3. **Visible.** If the system is doing expensive work, the status bar says so.
4. **Exportable on demand.** The user can dump a report; nothing is uploaded.
5. **No PII in logs.** Paths are the user's business; logs are mode 0600.

## Signals

| Signal | Source | Retention |
| --- | --- | --- |
| Frame time / dropped frames | compositor pacer | ring buffer, last 10 min |
| Input latency | input timestamp → damage | last 1k samples |
| Index size / query latency | `aether-index` | counters + histogram |
| Agent usage (CPU, memory, time) | cgroups | per-agent lifetime |
| Capability denials | `aetherd` audit | persistent, chained |
| Audio xruns | PipeWire | counters |
| Boot timing | systemd | last 10 boots |
| Crash dumps | shell/daemons | local, opt-in export |

## The status bar

Always visible, minimal:

```text
aether@kestrel   2/6 agents   14% CPU   1.8 GiB   ♪ Speakers · Analog   7.2 ms
```

It shows only live values. No graphs, no sparklines, no “health score”.

## Admin overlay

`⌘⇧I` opens a local overlay with:

- Frame budget graph (last 10 minutes).
- Per-agent CPU/memory and isolation tier.
- Index coverage and staleness.
- Audio graph summary.
- Capability audit tail (last 50 entries, chain status).

The overlay is shell-owned and cannot be captured by remote display without an
explicit indicator.

## Logging

| Stream | Destination | Level |
| --- | --- | --- |
| systemd journal | `/var/log/journal` (bounded) | info |
| `aetherd` audit | `/var/lib/aether/audit.jsonl` | append-only |
| shell | journal, `aether-shell` tag | warn |
| agents | per-agent file, mode 0600 | info |
| browser | disposable profile | warn |

Rules:

- No log line contains a bearer token, API key or file contents.
- Paths are logged only for operations the user initiated or an agent was denied.
- Journal is capped (500 MB) and rotated.
- Audit is never rotated automatically; the user archives it.

## Audit chain

The audit log is the only persistent security signal. Each entry commits to the
previous hash, so deletion or edits are detectable.

```bash
aether audit verify           # exits non-zero on a broken chain
aether audit export --since 2026-11-01 > audit.json
```

Verification is available without the daemon (the format is documented in
[collaboration](/architecture/collaboration#audit)).

## Performance overlays

For development, not shipped in the default UI:

```bash
AEther_TRACE=frame,index,audio aether-shell
```

Traces are written to `~/.cache/aether/traces/` as JSON lines, suitable for
`jq`/`perfetto` import. They are opt-in and never enabled in release builds.

## What is not collected

- No usage analytics, page views, feature flags or “anonymous statistics”.
- No device fingerprint.
- No crash upload.
- No remote logging.
- No network calls from the shell except the configured inference endpoint.

This is a product constraint. A PR that adds a network call to a telemetry endpoint
is rejected, not debated.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Audit file full | Writes fail closed; agents cannot start without audit |
| Journal full | Rotation; oldest entries dropped |
| Clock jumps | Audit timestamps may go backwards; chain still verifies |
| Trace overhead | Disabled by default; compile-time feature in release |
| Status bar wedged | Shell watchdog restarts it; counters survive in shared memory |

## Verification

| Test | Phase |
| --- | --- |
| Audit chain tamper detection | ✅ `packages/agents/src/audit.test.ts` |
| Capability denial is recorded | ✅ collaboration tests |
| No telemetry network calls | P1 (strace the session for 10 min) |
| Journal cap enforced | P1 |
| Overlay cannot be captured silently | P7 |
