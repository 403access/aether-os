import { describe, expect, test } from "bun:test";
import { MESSAGE } from "@aether/protocol";
import { fnv1a64, RingBuffer, WRAP_MARKER } from "../src/index";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function payload(text: string): Uint8Array {
  return encoder.encode(text);
}

describe("RingBuffer basics", () => {
  test("round-trips kind, capability and payload", () => {
    const ring = RingBuffer.create(1024, { shared: false });
    expect(ring.tryPush(MESSAGE.chatToken, 42n, payload("hello"))).toEqual({
      ok: true,
      bytes: 24,
    });
    const message = ring.tryPop();
    expect(message).not.toBeNull();
    expect(message?.kind).toBe(MESSAGE.chatToken);
    expect(message?.capability).toBe(42n);
    expect(decoder.decode(message?.payload)).toBe("hello");
    expect(ring.tryPop()).toBeNull();
  });

  test("preserves FIFO order", () => {
    const ring = RingBuffer.create(4096, { shared: false });
    for (let index = 0; index < 100; index += 1) {
      expect(ring.tryPush(1, BigInt(index), payload(`m${index}`)).ok).toBe(true);
    }
    for (let index = 0; index < 100; index += 1) {
      const message = ring.tryPop();
      expect(message?.capability).toBe(BigInt(index));
      expect(decoder.decode(message?.payload)).toBe(`m${index}`);
    }
  });

  test("tracks used and free bytes", () => {
    const ring = RingBuffer.create(1024, { shared: false });
    expect(ring.freeBytes).toBe(1024);
    ring.tryPush(1, 0n, payload("abcd"));
    // 16 byte header + 4 bytes payload rounded to 8 = 24
    expect(ring.usedBytes).toBe(24);
    expect(ring.freeBytes).toBe(1000);
    ring.tryPop();
    expect(ring.usedBytes).toBe(0);
  });

  test("empty payload is legal", () => {
    const ring = RingBuffer.create(512, { shared: false });
    expect(ring.tryPush(MESSAGE.control, 1n, new Uint8Array(0)).ok).toBe(true);
    expect(ring.tryPop()?.payload.length).toBe(0);
  });
});

describe("backpressure", () => {
  test("rejects when full and counts drops", () => {
    const ring = RingBuffer.create(256, { shared: false });
    let accepted = 0;
    for (let index = 0; index < 100; index += 1) {
      const outcome = ring.tryPush(1, 0n, payload("0123456789abcdef"));
      if (!outcome.ok) {
        expect(outcome.reason).toBe("full");
        break;
      }
      accepted += 1;
    }
    expect(accepted).toBeGreaterThan(0);
    expect(accepted).toBeLessThan(100);
    expect(ring.stats().dropped).toBeGreaterThan(0);
    expect(ring.drain()).toBe(accepted);
    expect(ring.stats().dropped).toBeGreaterThan(0);
  });

  test("rejects records that can never fit", () => {
    const ring = RingBuffer.create(256, { shared: false });
    const huge = new Uint8Array(300);
    expect(ring.tryPush(1, 0n, huge)).toEqual({ ok: false, reason: "too-large" });
    expect(ring.usedBytes).toBe(0);
  });

  test("recovers after draining", () => {
    const ring = RingBuffer.create(256, { shared: false });
    const chunk = payload("x".repeat(64));
    while (ring.tryPush(1, 0n, chunk).ok) {
      // fill
    }
    ring.drain();
    expect(ring.tryPush(1, 0n, chunk).ok).toBe(true);
  });
});

describe("wrap-around", () => {
  test("keeps integrity across 2000 variable-size messages", () => {
    const ring = RingBuffer.create(1024, { shared: false });
    const received: string[] = [];

    for (let index = 0; index < 2000; index += 1) {
      const text = `msg-${index}-${"z".repeat((index * 7) % 120)}`;
      while (!ring.tryPush(MESSAGE.indexUpdate, BigInt(index), payload(text)).ok) {
        const message = ring.tryPop();
        if (message === null) throw new Error("ring reported full but was empty");
        received.push(decoder.decode(message.payload));
      }
      while (received.length < index - 3) {
        const message = ring.tryPop();
        if (message === null) break;
        received.push(decoder.decode(message.payload));
      }
    }

    while (true) {
      const message = ring.tryPop();
      if (message === null) break;
      received.push(decoder.decode(message.payload));
    }

    expect(received.length).toBe(2000);
    expect(received[0]).toBe("msg-0-");
    expect(received[1999]).toBe(`msg-1999-${"z".repeat((1999 * 7) % 120)}`);
    expect(new Set(received).size).toBe(2000);
  });

  test("writes a wrap marker instead of splitting a record", () => {
    const ring = RingBuffer.create(256, { shared: false });

    // Fill 216 of 256 bytes, then drain so the tail is at 216 but the head is
    // also at 216: the next record cannot fit contiguously and must wrap.
    const big = payload("b".repeat(200));
    expect(ring.tryPush(1, 0n, big)).toEqual({ ok: true, bytes: 216 });
    expect(ring.tryPop()?.payload.length).toBe(200);

    const small = payload("s".repeat(32));
    expect(ring.tryPush(2, 5n, small)).toEqual({ ok: true, bytes: 48 });

    const marker = new DataView(ring.buffer).getUint32(64 + 216, true);
    expect(marker).toBe(WRAP_MARKER);

    const message = ring.tryPop();
    expect(message?.kind).toBe(2);
    expect(message?.capability).toBe(5n);
    expect(message?.payload.length).toBe(32);
    expect(ring.tryPop()).toBeNull();
    expect(ring.usedBytes).toBe(0);
  });
});

describe("zero-copy view", () => {
  test("handler receives a view into the ring buffer", () => {
    const ring = RingBuffer.create(1024);
    ring.tryPush(MESSAGE.agentHandoff, 7n, payload("inline"));
    let owned = false;
    let seen = "";
    const handled = ring.tryPopWith((message) => {
      owned = message.payload.buffer === ring.buffer;
      seen = decoder.decode(message.payload);
      expect(message.capability).toBe(7n);
    });
    expect(handled).toBe(true);
    expect(owned).toBe(true);
    expect(seen).toBe("inline");
    expect(ring.tryPopWith(() => {})).toBe(false);
  });
});

describe("shared memory and attach", () => {
  test("uses a SharedArrayBuffer by default", () => {
    const ring = RingBuffer.create(512);
    expect(ring.buffer).toBeInstanceOf(SharedArrayBuffer);
    expect(ring.shared).toBe(true);
  });

  test("attach validates magic and version", () => {
    const ring = RingBuffer.create(512);
    const attached = RingBuffer.attach(ring.buffer);
    expect(attached.capacity).toBe(512);

    const corrupt = new SharedArrayBuffer(1024);
    expect(() => RingBuffer.attach(corrupt)).toThrow(/bad magic/);

    const buffer = new SharedArrayBuffer(1024);
    const view = new DataView(buffer);
    view.setUint32(0, 0x41455448, true);
    view.setUint32(4, 999, true);
    expect(() => RingBuffer.attach(buffer)).toThrow(/unsupported ring version/);
  });

  test("two handles on the same buffer see the same messages", () => {
    const producer = RingBuffer.create(1024);
    const consumer = RingBuffer.attach(producer.buffer);
    producer.tryPush(1, 9n, payload("shared"));
    expect(decoder.decode(consumer.tryPop()?.payload)).toBe("shared");
    expect(producer.stats().usedBytes).toBe(0);
  });
});

describe("tokens", () => {
  test("fnv1a64 is deterministic and 64-bit masked", () => {
    expect(fnv1a64("cap_1")).toBe(fnv1a64("cap_1"));
    expect(fnv1a64("cap_1")).not.toBe(fnv1a64("cap_2"));
    expect(fnv1a64("cap_1")).toBeLessThanOrEqual(0xffff_ffff_ffff_ffffn);
  });

  test("WRAP_MARKER is outside every valid payload length", () => {
    expect(WRAP_MARKER).toBe(0xffff_ffff);
  });
});
