import { describe, expect, test } from "bun:test";
import { InputBatcher, RemoteClient, CountingSink } from "./index";
import type { RemoteTransport } from "./client";
import { detectCapabilities, probeCapabilities, selectCodec, selectTier, tierInfo } from "./capabilities";
import type { ClientCaps, SignalMessage } from "./types";
import { decodeWireFrame, encodeWireFrame } from "./wire";
import { LatencyLedger } from "./stats";
import { scancodeForCode } from "./input";

const MODERN: ClientCaps = {
  webrtc: true,
  websocket: true,
  webcodecs: true,
  jpeg: true,
  h264: true,
  hevc: false,
  av1: true,
};

class FakeTransport implements RemoteTransport {
  readonly sent: SignalMessage[] = [];
  url = "";
  closed = false;
  answer = "v=0 answer";

  async connect(url: string): Promise<void> {
    this.url = url;
  }

  send(message: SignalMessage): void {
    this.sent.push(message);
  }

  async handleOffer(_sdp: string): Promise<string> {
    return this.answer;
  }

  close(): void {
    this.closed = true;
  }
}

function welcome(overrides: { version?: number } = {}): SignalMessage {
  return {
    type: "welcome",
    version: 1,
    session_id: "rs-test",
    token: "token-123",
    expires_at_ms: 10_000,
    tier: "webrtc",
    codec: "h264",
    dtls_fingerprint: "sha-256 AA",
    policy: {
      allow_input: true,
      allow_clipboard: false,
      allow_audio: false,
      max_bitrate_kbps: 12_000,
      session_ttl_ms: 3_600_000,
    },
    ...overrides,
  } as SignalMessage;
}

describe("capability probe", () => {
  test("detects the modern profile and falls back through tiers", () => {
    const scope = {
      RTCPeerConnection: function Rtc() {},
      WebSocket: function Socket() {},
      VideoDecoder: function Decoder() {},
      createImageBitmap: function Bitmap() {},
    };
    const caps = detectCapabilities(scope);
    expect(caps).toEqual(MODERN);
    expect(selectTier(caps, false)).toBe("webrtc");
    expect(selectTier(caps, true)).toBe("ws-au");
    expect(selectTier({ ...caps, webcodecs: false, h264: false, hevc: false, av1: false }, false)).toBe(
      "ws-jpeg",
    );
    expect(
      selectTier(
        { ...caps, websocket: false, webcodecs: false, h264: false, hevc: false, av1: false },
        false,
      ),
    ).toBe("vnc");
    expect(tierInfo("webrtc").budgetMs).toBeLessThan(tierInfo("vnc").budgetMs);
    expect(tierInfo("ws-jpeg").degraded).toBe(true);
  });

  test("probes decoder configurations", async () => {
    const scope = {
      RTCPeerConnection: function Rtc() {},
      WebSocket: function Socket() {},
      VideoDecoder: {
        isConfigSupported: async (config: { codec: string }) => ({
          supported: config.codec.startsWith("avc1"),
        }),
      },
    };
    const caps = await probeCapabilities(scope);
    expect(caps.webcodecs).toBe(true);
    expect(caps.h264).toBe(true);
    expect(caps.av1).toBe(false);
    expect(selectCodec(["av1", "h264"], caps)).toBe("h264");
    expect(selectCodec(["av1", "hevc"], caps)).toBeNull();
  });
});

describe("media wire format", () => {
  test("round-trips an access unit", () => {
    const payload = new Uint8Array([0, 0, 0, 1, 9]);
    const bytes = encodeWireFrame({
      tier: "ws-au",
      codec: "h264",
      keyframe: true,
      jpeg: false,
      sequence: 7,
      ptsUs: 123_456,
      payload,
    });
    expect(String.fromCharCode(...bytes.slice(0, 4))).toBe("AEM1");
    const decoded = decodeWireFrame(bytes);
    expect(decoded.sequence).toBe(7);
    expect(decoded.ptsMs).toBeCloseTo(123.456, 3);
    expect(decoded.keyframe).toBe(true);
    expect([...decoded.payload]).toEqual([...payload]);
  });

  test("rejects malformed and truncated frames", () => {
    expect(() => decodeWireFrame(new Uint8Array(8))).toThrow("shorter");
    const valid = encodeWireFrame({
      tier: "webrtc",
      codec: "h264",
      keyframe: false,
      jpeg: false,
      sequence: 1,
      ptsUs: 0,
      payload: new Uint8Array([1, 2, 3]),
    });
    const broken = new Uint8Array(valid);
    broken[0] = 0x58;
    expect(() => decodeWireFrame(broken)).toThrow("magic");
    const short = valid.slice(0, valid.length - 1);
    expect(() => decodeWireFrame(short)).toThrow("truncated");
  });
});

describe("input batching", () => {
  test("maps browser key codes to evdev scancodes", () => {
    expect(scancodeForCode("Escape")).toBe(1);
    expect(scancodeForCode("KeyA")).toBe(30);
    expect(scancodeForCode("ArrowLeft")).toBe(105);
    expect(scancodeForCode("F12")).toBe(88);
    expect(scancodeForCode("Unknown")).toBeUndefined();
  });

  test("requires focus, coalesces motion and enforces the rate", () => {
    const batcher = new InputBatcher(2);
    expect(batcher.pushKey("KeyA", true, 0, 0)).toBe(false);
    batcher.setFocused(true, 0);
    expect(batcher.pushMotion(3, 4, 1)).toBe(true);
    expect(batcher.pushMotion(-1, 2, 2)).toBe(true);
    expect(batcher.pushKey("KeyB", true, 0, 3)).toBe(true);
    expect(batcher.pushKey("KeyC", true, 0, 4)).toBe(true);
    expect(batcher.pushKey("KeyD", true, 0, 5)).toBe(false);
    const batch = batcher.drain();
    expect(batch.coalescedMotion).toBe(2);
    expect(batch.rejected).toBe(1);
    expect(batch.dropped).toBe(1);
    expect(batch.events).toContainEqual({ kind: "pointer-motion", dx: 2, dy: 6 });
  });
});

describe("remote client state machine", () => {
  function setup() {
    const transport = new FakeTransport();
    const sink = new CountingSink();
    const client = new RemoteClient({ transport, sink, caps: MODERN, now: () => 1_000 });
    return { transport, sink, client };
  }

  test("pairs, negotiates, streams and falls back without a restart", async () => {
    const { transport, sink, client } = setup();
    await client.start("abcd-efgh", "wss://host/signal");
    expect(client.state).toBe("negotiating");
    expect(transport.url).toBe("wss://host/signal");
    const hello = transport.sent[0];
    expect(hello?.type).toBe("hello");
    if (hello?.type === "hello") {
      expect(hello.pairing_code).toBe("ABCDEFGH");
      expect(hello.caps.webrtc).toBe(true);
    }

    await client.handle(welcome());
    expect(client.token).toBe("token-123");
    expect(client.sessionId).toBe("rs-test");
    expect(sink.config?.codec).toBe("h264");
    expect(client.state).toBe("negotiating");

    await client.handle({ type: "offer", sdp: "v=0 offer" });
    expect(transport.sent.at(-1)).toEqual({ type: "answer", sdp: "v=0 answer" });
    expect(client.state).toBe("streaming");

    const frame = encodeWireFrame({
      tier: "webrtc",
      codec: "h264",
      keyframe: true,
      jpeg: false,
      sequence: 1,
      ptsUs: 16_667,
      payload: new Uint8Array([9, 9]),
    });
    client.handleMedia(frame);
    const stats = client.stats();
    expect(stats.frames).toBe(1);
    expect(stats.intervalBudgetOk).toBe(false); // one frame has no interval sample yet

    client.fallback("ws-jpeg");
    expect(client.state).toBe("degraded");
    expect(sink.config?.tier).toBe("ws-jpeg");
    client.markRoundTrip(21);
    client.onDisconnected("udp blocked");
    expect(client.lastError).toBe("udp blocked");
    expect(client.state).toBe("degraded");

    client.stop();
    expect(client.state).toBe("closed");
    expect(transport.closed).toBe(true);
    expect(sink.closed).toBe(true);
  });

  test("rejects invalid pairing codes and media before streaming", async () => {
    const { client } = setup();
    await expect(client.start("nope", "ws://x")).rejects.toThrow("8 characters");
    expect(() => client.handleMedia(new Uint8Array(32))).toThrow("before streaming");
    await expect(client.handle(welcome({ version: 99 }))).rejects.toThrow("version");
    await expect(client.handle({ type: "error", code: "no-codec", message: "none" })).rejects.toThrow(
      "no-codec",
    );
  });

  test("reports a preferred tier without a session", () => {
    const { client } = setup();
    expect(client.preferredTier()).toBe("webrtc");
  });
});

describe("stats", () => {
  test("nearest-rank percentiles and interval budget", () => {
    const ledger = new LatencyLedger();
    for (let value = 1; value <= 100; value += 1) ledger.push(value);
    expect(ledger.p50()).toBe(50);
    expect(ledger.p99()).toBe(99);
    expect(ledger.max()).toBe(100);
    expect(new LatencyLedger().p99()).toBeNull();
  });
});

