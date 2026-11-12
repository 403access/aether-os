---
id: collaboration
title: Agent collaboration
sidebar_position: 9
description: How isolated agents exchange state without violating isolation.
---

# Agent collaboration

Isolation without collaboration is a wall. Collaboration without isolation is a
breach. Aether uses **capability-gated shared-memory channels** opened by the broker.

## Model

```text
worker A ── capability(channel:c1) ──┐
                                     ├── SPSC ring (memfd) ── audit
worker B ── capability(channel:c1) ──┘
```

- The broker (`aetherd`) creates the channel and issues one capability per
  participant.
- Participants map the same memfd and exchange fixed-layout records.
- No JSON, no HTTP, no serialisation of the hot path.
- Closing the channel revokes both capabilities immediately.

An agent that did not receive a capability cannot address the channel, even if it
knows the id. It has no file descriptor and no mapping.

## Channel lifecycle

1. **Request.** A coordinator asks `aetherd` for a channel to a peer.
2. **Policy.** The broker verifies that the coordinator holds `agent.collab`, that
   the peer exists, and that the requested capacity/TTL fit the budget.
3. **Issue.** Two capabilities are minted: `channel:<id>` for each subject, scoped
   to `agent.collab`, non-delegable, with a TTL.
4. **Map.** Each agent receives the memfd and a token. Mapping is one syscall.
5. **Exchange.** Messages are pushed/poped over the ring.
6. **Close.** Either participant (or the broker) closes; revocation is immediate.

The reference policy implementation is `packages/agents/src/collaboration.ts`.

## Ring protocol

The shared layout is defined in [IPC](/architecture/overview#the-four-data-paths)
and implemented in `packages/ipc/src/ring.ts`:

```text
64-byte header (magic, version, capacity, head, tail, dropped)
16-byte record header (length, kind, capability token)
payload, padded to 8 bytes
0xFFFFFFFF wrap marker when a record cannot fit contiguously
```

Guarantees:

- Single producer, single consumer.
- `head` is written only by the producer; `tail` only by the consumer.
- A record is published only after its bytes are written.
- The consumer copies (or finishes reading) **before** advancing `tail`; this closes
  the only data race we found during development.
- Oversized records are rejected, never truncated.
- When the ring is full, `tryPush` fails and counts a backpressure event; the caller
  decides whether to retry, drop, or block.

The daemon maps the same layout from `memfd_create`; the browser demo and tests use
a `SharedArrayBuffer`.

## Message kinds

| Kind | Payload | Used by |
| --- | --- | --- |
| `1` indexUpdate | document id + changed fields | index → palette |
| `2` chatToken | UTF-8 fragment | chat → shell |
| `3` audioState | route + volume | audio → status |
| `4` frameReady | DMA-BUF handle + damage | capture → encoder |
| `5` agentHandoff | task id + result ref | agent → agent |
| `6` control | command + args | broker → daemon |
| `7` audit | chained entry | any → audit sink |

Payloads are versioned by kind. Unknown kinds are ignored, not crashed on. This is
how the shell can be upgraded while an agent still runs an older protocol.

## Backpressure policy

| Consumer state | Policy |
| --- | --- |
| Palette updates | Drop oldest; the UI only needs the latest state |
| Chat tokens | Never drop; block the producer with a bounded wait |
| Audio state | Latest-wins; replace in place |
| Frame ready | Drop oldest; a stale frame is worse than no frame |
| Agent handoff | Retry with exponential backoff, then fail the task |

The policy belongs to the channel, not the transport. The ring exposes counters so
the broker can audit drops.

## Capability semantics

- **Scope:** only `agent.collab` can address a channel.
- **Resource:** exactly `channel:<id>`. Hierarchical matching prevents
  `channel:1` from matching `channel:10`.
- **Subject:** the token is bound to one agent. A stolen token fails the subject
  check.
- **Delegation:** off. A channel is a two-party agreement; sub-agents get their own
  channel.
- **Revocation:** cascades to any child capabilities (there are none today) and is
  reflected in the audit chain.

Verification is a map lookup plus a few integer comparisons; the benchmark measures
9.2 M ops/s on the development host.

## Example

```ts
const bus = new CollaborationBus();
const { channel, tokens } = bus.open("coordinator", "worker-a", {
  capacity: 64 * 1024,
  ttlMs: 120_000,
});

bus.send(channel.id, "coordinator", tokens.sender.id, MESSAGE.agentHandoff, payload);
const result = bus.receive(channel.id, "worker-a", tokens.receiver.id);

bus.close(channel.id, "coordinator"); // both tokens revoked
```

A third agent presenting either token gets `denied`. After close, the same token
gets `revoked`.

## Audit

Every channel event is appended to the broker's hash chain:

```text
channel.open      coordinator → worker-a
channel.send      (optional, auditTraffic)
channel.receive   (optional)
channel.send.denied  agent-c attempted to use worker-a's token
channel.close      2 capabilities revoked
```

Denials are always recorded; traffic is opt-in to avoid unbounded logs during a
long session. The chain is exported as JSON and verifiable without the daemon.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Peer exits | Broker closes the channel, revokes tokens, audits |
| Ring full | Policy per kind; counters exposed |
| Corrupt length | Consumer rejects the record, closes the channel |
| Token expired | Operation returns `expired`; broker logs |
| Broker crash | Channels close; agents receive EOF; no orphan mappings |
| Agent tries another channel | Subject/resource check fails; audit records denial |

## What this design does not solve

- **Semantic trust.** A worker can send a malicious patch to a peer with a valid
  token. The OS enforces transport, not content.
- **Covert channels.** Shared rings can carry steganographic data if two agents
  both hold capabilities. Channel creation is audited; content is not inspected.
- **Fairness.** A fast producer can dominate a ring; the policy counters make this
  visible but do not schedule.

These are accepted residual risks, listed in the
[threat model](/ops/security-threat-model).
