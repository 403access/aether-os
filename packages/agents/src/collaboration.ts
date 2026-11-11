import { fnv1a64, CapabilityTable, RingBuffer, type RingMessage } from "@aether/ipc";
import type { Capability } from "@aether/protocol";
import { AuditLog } from "./audit";

/**
 * Capability-gated collaboration between agents.
 *
 * Two agents that must not share state get two independent capabilities over
 * one channel. A message is accepted only if the sender presents a valid,
 * unrevoked capability for that channel; the same applies to the receiver.
 * Closing the channel revokes both capabilities, so any retained handle fails
 * immediately.
 *
 * The transport is the same SPSC ring used by `aether-ipc`; in the shipping
 * system the channel is a memfd mapped by both agents, and the broker never
 * touches the payload.
 */

export interface Channel {
  id: string;
  from: string;
  to: string;
  capacity: number;
  createdAt: number;
}

export interface ChannelTokens {
  sender: Capability;
  receiver: Capability;
}

export type CollaborationFailure =
  | "unknown-channel"
  | "denied"
  | "revoked"
  | "expired"
  | "full"
  | "too-large"
  | "invalid";

export type OpenResult =
  | { ok: true; channel: Channel; tokens: ChannelTokens }
  | { ok: false; reason: CollaborationFailure };

export interface OpenOptions {
  capacity?: number;
  ttlMs?: number;
}

interface InternalChannel {
  channel: Channel;
  ring: RingBuffer;
  senderCapabilityId: string;
  receiverCapabilityId: string;
}

export interface CollaborationBusOptions {
  capabilities?: CapabilityTable;
  audit?: AuditLog;
  now?: () => number;
  /** Log every message, not just channel lifecycle events. Default false. */
  auditTraffic?: boolean;
}

export class CollaborationBus {
  readonly capabilities: CapabilityTable;
  readonly audit: AuditLog;
  readonly #channels = new Map<string, InternalChannel>();
  readonly #now: () => number;
  readonly #auditTraffic: boolean;
  #sequence = 0;

  constructor(options: CollaborationBusOptions = {}) {
    this.#now = options.now ?? (() => Date.now());
    this.capabilities = options.capabilities ?? new CapabilityTable({ now: this.#now });
    this.audit = options.audit ?? new AuditLog({ now: this.#now });
    this.#auditTraffic = options.auditTraffic ?? false;
  }

  open(from: string, to: string, options: OpenOptions = {}): OpenResult {
    if (from === to) {
      this.audit.append({
        actor: from,
        action: "channel.denied",
        subject: from,
        detail: "cannot open a channel to itself",
      });
      return { ok: false, reason: "invalid" };
    }

    this.#sequence += 1;
    const id = `chan_${this.#sequence.toString(36)}`;
    const createdAt = this.#now();
    const capacity = options.capacity ?? 16 * 1024;
    const resource = `channel:${id}`;

    const sender = this.capabilities.issue({
      subject: from,
      scope: "agent.collab",
      resource,
      ...(options.ttlMs === undefined ? {} : { ttlMs: options.ttlMs }),
      delegable: false,
    });
    const receiver = this.capabilities.issue({
      subject: to,
      scope: "agent.collab",
      resource: resource,
      ...(options.ttlMs === undefined ? {} : { ttlMs: options.ttlMs }),
      delegable: false,
    });

    const channel: Channel = { id, from, to, capacity, createdAt };
    this.#channels.set(id, {
      channel,
      ring: RingBuffer.create(capacity),
      senderCapabilityId: sender.id,
      receiverCapabilityId: receiver.id,
    });

    this.audit.append({
      actor: from,
      action: "channel.open",
      subject: `${from}->${to}`,
      detail: id,
    });

    return { ok: true, channel, tokens: { sender, receiver } };
  }

  send(
    channelId: string,
    from: string,
    capabilityId: string,
    kind: number,
    payload: Uint8Array,
  ): { ok: true; bytes: number } | { ok: false; reason: CollaborationFailure } {
    const internal = this.#channels.get(channelId);
    if (internal === undefined) return { ok: false, reason: "unknown-channel" };

    const verification = this.capabilities.verify(capabilityId, {
      subject: from,
      scope: "agent.collab",
      resource: `channel:${channelId}`,
    });
    if (!verification.ok) {
      const reason = mapVerification(verification.reason);
      this.audit.append({
        actor: from,
        action: "channel.send.denied",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: `${channelId}: ${reason}`,
      });
      return { ok: false, reason };
    }

    const outcome = internal.ring.tryPush(kind, fnv1a64(capabilityId), payload);
    if (!outcome.ok) {
      this.audit.append({
        actor: from,
        action: "channel.send.rejected",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: `${channelId}: ${outcome.reason}`,
      });
      return { ok: false, reason: outcome.reason };
    }

    if (this.#auditTraffic) {
      this.audit.append({
        actor: from,
        action: "channel.send",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: `${channelId}: ${payload.byteLength} bytes`,
      });
    }
    return { ok: true, bytes: outcome.bytes };
  }

  receive(
    channelId: string,
    to: string,
    capabilityId: string,
    max = 64,
  ): { ok: true; messages: RingMessage[] } | { ok: false; reason: CollaborationFailure } {
    const internal = this.#channels.get(channelId);
    if (internal === undefined) return { ok: false, reason: "unknown-channel" };

    const verification = this.capabilities.verify(capabilityId, {
      subject: to,
      scope: "agent.collab",
      resource: `channel:${channelId}`,
    });
    if (!verification.ok) {
      const reason = mapVerification(verification.reason);
      this.audit.append({
        actor: to,
        action: "channel.receive.denied",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: `${channelId}: ${reason}`,
      });
      return { ok: false, reason };
    }

    const messages: RingMessage[] = [];
    while (messages.length < max) {
      const message = internal.ring.tryPop();
      if (message === null) break;
      messages.push(message);
    }

    if (this.#auditTraffic && messages.length > 0) {
      this.audit.append({
        actor: to,
        action: "channel.receive",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: `${channelId}: ${messages.length} messages`,
      });
    }
    return { ok: true, messages };
  }

  close(channelId: string, actor: string): { ok: true; revoked: number } | { ok: false; reason: CollaborationFailure } {
    const internal = this.#channels.get(channelId);
    if (internal === undefined) return { ok: false, reason: "unknown-channel" };
    if (actor !== internal.channel.from && actor !== internal.channel.to) {
      this.audit.append({
        actor,
        action: "channel.close.denied",
        subject: `${internal.channel.from}->${internal.channel.to}`,
        detail: channelId,
      });
      return { ok: false, reason: "denied" };
    }

    const revoked =
      this.capabilities.revoke(internal.senderCapabilityId) +
      this.capabilities.revoke(internal.receiverCapabilityId);
    this.#channels.delete(channelId);

    this.audit.append({
      actor,
      action: "channel.close",
      subject: `${internal.channel.from}->${internal.channel.to}`,
      detail: `${channelId}: ${revoked} capabilities revoked`,
    });
    return { ok: true, revoked };
  }

  channel(channelId: string): Channel | undefined {
    return this.#channels.get(channelId)?.channel;
  }

  list(): Channel[] {
    return [...this.#channels.values()].map((internal) => internal.channel);
  }

  pending(channelId: string): number {
    return this.#channels.get(channelId)?.ring.usedBytes ?? 0;
  }
}

function mapVerification(reason: string | undefined): CollaborationFailure {
  if (reason === "revoked" || reason === "generation") return "revoked";
  if (reason === "expired") return "expired";
  return "denied";
}
