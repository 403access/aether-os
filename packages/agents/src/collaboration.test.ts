import { describe, expect, test } from "bun:test";
import { fnv1a64 } from "@aether/ipc";
import { CollaborationBus } from "../src/collaboration";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function bytes(text: string): Uint8Array {
  return encoder.encode(text);
}

describe("CollaborationBus", () => {
  test("opens a channel and moves payloads in both directions", () => {
    const bus = new CollaborationBus();
    const opened = bus.open("agent-a", "agent-b");
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;

    const { channel, tokens } = opened;
    expect(bus.send(channel.id, "agent-a", tokens.sender.id, 5, bytes("hello"))).toEqual({
      ok: true,
      bytes: 24,
    });

    const received = bus.receive(channel.id, "agent-b", tokens.receiver.id);
    expect(received.ok).toBe(true);
    if (!received.ok) return;
    expect(received.messages.length).toBe(1);
    expect(decoder.decode(received.messages[0]?.payload)).toBe("hello");
    expect(received.messages[0]?.capability).toBe(fnv1a64(tokens.sender.id));

    // Reply over the same channel using the receiver's capability.
    expect(bus.send(channel.id, "agent-b", tokens.receiver.id, 6, bytes("ack")).ok).toBe(true);
    const reply = bus.receive(channel.id, "agent-a", tokens.sender.id);
    expect(reply.ok && decoder.decode(reply.messages[0]?.payload)).toBe("ack");
  });

  test("rejects self-channels", () => {
    const bus = new CollaborationBus();
    expect(bus.open("agent-a", "agent-a")).toEqual({ ok: false, reason: "invalid" });
    expect(bus.list()).toEqual([]);
  });

  test("denies agents that do not hold a capability for the channel", () => {
    const bus = new CollaborationBus();
    const opened = bus.open("agent-a", "agent-b");
    if (!opened.ok) throw new Error("open failed");
    const { channel, tokens } = opened;

    // A third agent cannot use either membership token.
    expect(bus.send(channel.id, "agent-c", tokens.sender.id, 1, bytes("x"))).toEqual({
      ok: false,
      reason: "denied",
    });
    expect(bus.send("chan_nope", "agent-a", tokens.sender.id, 1, bytes("x"))).toEqual({
      ok: false,
      reason: "unknown-channel",
    });
  });

  test("closing revokes both capabilities immediately", () => {
    const bus = new CollaborationBus();
    const opened = bus.open("agent-a", "agent-b");
    if (!opened.ok) throw new Error("open failed");
    const { channel, tokens } = opened;

    expect(bus.close(channel.id, "agent-a")).toEqual({ ok: true, revoked: 2 });
    expect(bus.send(channel.id, "agent-a", tokens.sender.id, 1, bytes("x"))).toEqual({
      ok: false,
      reason: "unknown-channel",
    });
    // Even the capability table now considers the token revoked.
    expect(
      bus.capabilities.verify(tokens.sender.id, {
        subject: "agent-a",
        scope: "agent.collab",
        resource: `channel:${channel.id}`,
      }),
    ).toEqual({ ok: false, reason: "revoked" });
    expect(bus.close(channel.id, "agent-a")).toEqual({
      ok: false,
      reason: "unknown-channel",
    });
  });

  test("a non-member cannot close a channel", () => {
    const bus = new CollaborationBus();
    const opened = bus.open("agent-a", "agent-b");
    if (!opened.ok) throw new Error("open failed");
    expect(bus.close(opened.channel.id, "agent-c")).toEqual({ ok: false, reason: "denied" });
    expect(bus.channel(opened.channel.id)).toBeDefined();
  });

  test("expired capabilities are rejected", () => {
    let now = 1_000;
    const bus = new CollaborationBus({ now: () => now });
    const opened = bus.open("agent-a", "agent-b", { ttlMs: 500 });
    if (!opened.ok) throw new Error("open failed");
    now = 2_000;
    expect(
      bus.send(opened.channel.id, "agent-a", opened.tokens.sender.id, 1, bytes("late")),
    ).toEqual({ ok: false, reason: "expired" });
  });

  test("reports backpressure instead of growing without bound", () => {
    const bus = new CollaborationBus();
    const opened = bus.open("agent-a", "agent-b", { capacity: 512 });
    if (!opened.ok) throw new Error("open failed");

    let accepted = 0;
    let failure: string | null = null;
    for (let index = 0; index < 100; index += 1) {
      const result = bus.send(
        opened.channel.id,
        "agent-a",
        opened.tokens.sender.id,
        1,
        bytes("x".repeat(64)),
      );
      if (result.ok) accepted += 1;
      else {
        failure = result.reason;
        break;
      }
    }
    expect(accepted).toBeGreaterThan(0);
    expect(failure).toBe("full");
    expect(bus.pending(opened.channel.id)).toBeGreaterThan(0);
  });

  test("audit log records lifecycle, denials and optionally traffic", () => {
    const bus = new CollaborationBus({ auditTraffic: true });
    const opened = bus.open("agent-a", "agent-b");
    if (!opened.ok) throw new Error("open failed");
    bus.send(opened.channel.id, "agent-a", opened.tokens.sender.id, 1, bytes("hello"));
    bus.send(opened.channel.id, "agent-c", opened.tokens.sender.id, 1, bytes("nope"));
    bus.receive(opened.channel.id, "agent-b", opened.tokens.receiver.id);
    bus.close(opened.channel.id, "agent-b");

    expect(bus.audit.verify()).toEqual({ ok: true });
    const actions = bus.audit.entries.map((entry) => entry.action);
    expect(actions).toContain("channel.open");
    expect(actions).toContain("channel.send.denied");
    expect(actions).toContain("channel.send");
    expect(actions).toContain("channel.receive");
    expect(actions).toContain("channel.close");
  });
});
