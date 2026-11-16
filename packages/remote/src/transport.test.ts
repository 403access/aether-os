import { describe, expect, test } from "bun:test";
import {
  HttpSignalTransport,
  normalizeSignalUrl,
  parseSdp,
  waitForIceGathering,
} from "./transport";
import type { SignalMessage } from "./types";

type Handler = (event?: unknown) => void;

class FakeChannel {
  readyState: RTCDataChannelState = "open";
  sent: string[] = [];
  closed = false;

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.closed = true;
    this.readyState = "closed";
  }
}

class FakePeerConnection {
  static last: FakePeerConnection | null = null;

  iceGatheringState: RTCIceGatheringState = "complete";
  connectionState: RTCPeerConnectionState = "new";
  localDescription: RTCSessionDescription | null = null;
  ontrack: ((event: RTCTrackEvent) => void) | null = null;
  ondatachannel: ((event: RTCDataChannelEvent) => void) | null = null;
  onconnectionstatechange: (() => void) | null = null;
  remote: RTCSessionDescriptionInit | null = null;
  closed = false;
  readonly config: RTCConfiguration;
  #listeners = new Map<string, Set<Handler>>();

  constructor(config: RTCConfiguration) {
    this.config = config;
    FakePeerConnection.last = this;
  }

  addEventListener(type: string, handler: Handler): void {
    const set = this.#listeners.get(type) ?? new Set<Handler>();
    set.add(handler);
    this.#listeners.set(type, set);
  }

  removeEventListener(type: string, handler: Handler): void {
    this.#listeners.get(type)?.delete(handler);
  }

  dispatch(type: string): void {
    for (const handler of this.#listeners.get(type) ?? []) handler();
  }

  async setRemoteDescription(description: RTCSessionDescriptionInit): Promise<void> {
    this.remote = description;
  }

  async createAnswer(): Promise<RTCSessionDescriptionInit> {
    return { type: "answer", sdp: "v=0\r\nanswer" };
  }

  async setLocalDescription(description: RTCSessionDescriptionInit): Promise<void> {
    this.localDescription = {
      type: "answer",
      sdp: description.sdp,
    } as RTCSessionDescription;
  }

  close(): void {
    this.closed = true;
    this.connectionState = "closed";
  }

  setConnectionState(state: RTCPeerConnectionState): void {
    this.connectionState = state;
    this.onconnectionstatechange?.();
  }

  openChannel(channel: RTCDataChannel): void {
    this.ondatachannel?.({ channel } as RTCDataChannelEvent);
  }

  emitTrack(stream: MediaStream, track: MediaStreamTrack): void {
    this.ontrack?.({ streams: [stream], track } as unknown as RTCTrackEvent);
  }
}

const factory = (config: RTCConfiguration): RTCPeerConnection =>
  new FakePeerConnection(config) as unknown as RTCPeerConnection;

// The host nests a bare welcome document without the signal tag.
const WELCOME = {
  version: 1,
  session_id: "rs-test",
  token: "token",
  expires_at_ms: 1,
  tier: "webrtc",
  codec: "h264",
  dtls_fingerprint: "sha-256",
  policy: {
    allow_input: true,
    allow_clipboard: false,
    allow_audio: false,
    max_bitrate_kbps: 4_000,
    session_ttl_ms: 3_600_000,
  },
} as const;

interface RecordedCall {
  url: string;
  body: string;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function lastPeer(): FakePeerConnection | null {
  return FakePeerConnection.last;
}

describe("signaling URL and SDP adaptation", () => {
  test("a bare host points at the signal endpoint", () => {
    expect(normalizeSignalUrl("http://127.0.0.1:7443")).toBe(
      "http://127.0.0.1:7443/signal",
    );
    expect(normalizeSignalUrl("http://127.0.0.1:7443/")).toBe(
      "http://127.0.0.1:7443/signal",
    );
    expect(normalizeSignalUrl("http://127.0.0.1:7443/signal")).toBe(
      "http://127.0.0.1:7443/signal",
    );
    expect(() => normalizeSignalUrl("   ")).toThrow("signaling URL");
  });

  test("unwraps the str0m JSON and accepts a bare SDP string", () => {
    expect(
      parseSdp(JSON.stringify({ type: "offer", sdp: "v=0\r\nm=video" })),
    ).toEqual({ type: "offer", sdp: "v=0\r\nm=video" });
    expect(parseSdp("v=0\r\nm=video")).toEqual({
      type: "offer",
      sdp: "v=0\r\nm=video",
    });
    expect(() => parseSdp("{}")).toThrow("malformed SDP");
    expect(() => parseSdp("not json")).toThrow("malformed SDP");
  });
});

describe("HttpSignalTransport", () => {
  test("posts the hello and dispatches the welcome before the offer", async () => {
    FakePeerConnection.last = null;
    const calls: RecordedCall[] = [];
    const fetchImpl = (async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: String(init?.body) });
      return jsonResponse({
        welcome: WELCOME,
        offer: JSON.stringify({ type: "offer", sdp: "v=0\r\noffer" }),
      });
    }) as unknown as typeof fetch;

    const seen: SignalMessage[] = [];
    let answer = "";
    let transport!: HttpSignalTransport;
    transport = new HttpSignalTransport({
      fetch: fetchImpl,
      peerConnection: factory,
      onSignal: async (message) => {
        seen.push(message);
        if (message.type === "offer") answer = await transport.handleOffer(message.sdp);
      },
    });

    await transport.connect("http://127.0.0.1:7443");
    transport.send({
      type: "hello",
      version: 1,
      pairing_code: "K7QM2H4P",
      client_name: "test",
      udp_blocked: false,
      caps: {
        webrtc: true,
        websocket: true,
        webcodecs: true,
        jpeg: true,
        h264: true,
        hevc: false,
        av1: false,
      },
    });
    await flush();

    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe("http://127.0.0.1:7443/signal");
    expect(JSON.parse(calls[0]?.body ?? "{}").type).toBe("hello");
    expect(seen.map((message) => message.type)).toEqual(["welcome", "offer"]);
    expect(seen[0]).toMatchObject({ type: "welcome", session_id: "rs-test" });
    expect(lastPeer()?.remote).toEqual({
      type: "offer",
      sdp: "v=0\r\noffer",
    });
    expect(JSON.parse(answer)).toEqual({
      type: "answer",
      sdp: "v=0\r\nanswer",
    });
  });

  test("posts the answer with the nested str0m shape", async () => {
    const calls: RecordedCall[] = [];
    const fetchImpl = (async (_url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(_url), body: String(init?.body) });
      return jsonResponse({ ok: true });
    }) as unknown as typeof fetch;

    const transport = new HttpSignalTransport({
      fetch: fetchImpl,
      peerConnection: factory,
      onSignal: () => undefined,
    });
    await transport.connect("http://127.0.0.1:7443/signal");
    transport.send({
      type: "answer",
      sdp: JSON.stringify({ type: "answer", sdp: "v=0\r\nanswer" }),
    });
    await flush();

    const body = JSON.parse(calls[0]?.body ?? "{}") as {
      type: string;
      sdp: string;
    };
    expect(body.type).toBe("answer");
    expect(JSON.parse(body.sdp)).toEqual({
      type: "answer",
      sdp: "v=0\r\nanswer",
    });
  });

  test("surfaces the host's pairing error", async () => {
    const fetchImpl = (async () =>
      jsonResponse({ error: "invalid pairing code" }, 403)) as unknown as typeof fetch;
    const errors: string[] = [];
    const transport = new HttpSignalTransport({
      fetch: fetchImpl,
      peerConnection: factory,
      onSignal: () => undefined,
      onError: (error) => errors.push(error.message),
    });
    await transport.connect("http://127.0.0.1:7443");
    transport.send({
      type: "hello",
      version: 1,
      pairing_code: "AAAAAAAA",
      client_name: "test",
      udp_blocked: false,
      caps: {
        webrtc: true,
        websocket: true,
        webcodecs: true,
        jpeg: true,
        h264: true,
        hevc: false,
        av1: false,
      },
    });
    await flush();
    expect(errors).toEqual(["invalid pairing code"]);
  });

  test("waits for ICE gathering before returning the answer", async () => {
    FakePeerConnection.last = null;
    const transport = new HttpSignalTransport({
      fetch: (async () => jsonResponse({ ok: true })) as unknown as typeof fetch,
      peerConnection: factory,
      onSignal: () => undefined,
    });
    await transport.connect("http://127.0.0.1:7443");

    const pending = transport.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));
    await flush();
    const pc = lastPeer();
    expect(pc?.iceGatheringState).toBe("complete");

    // Re-run with a gathering peer to prove the listener gates the answer.
    pc!.iceGatheringState = "gathering";
    pc!.localDescription = null;
    const gathering = transport.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));
    await flush();
    expect(pc!.localDescription).toBeNull();
    pc!.iceGatheringState = "complete";
    pc!.dispatch("icegatheringstatechange");
    const answer = await gathering;
    expect(JSON.parse(answer).type).toBe("answer");

    await pending;
  });

  test("times out a stalled ICE gathering", async () => {
    FakePeerConnection.last = null;
    const transport = new HttpSignalTransport({
      fetch: (async () => jsonResponse({ ok: true })) as unknown as typeof fetch,
      peerConnection: factory,
      onSignal: () => undefined,
      iceGatheringTimeoutMs: 5,
    });
    await transport.connect("http://127.0.0.1:7443");
    const answer = await transport.handleOffer(
      JSON.stringify({ type: "offer", sdp: "v=0" }),
    );
    expect(JSON.parse(answer).type).toBe("answer");
  });

  test("exposes the peer connection and its data channel", async () => {
    const transport = new HttpSignalTransport({
      fetch: (async () => jsonResponse({ ok: true })) as unknown as typeof fetch,
      peerConnection: factory,
      onSignal: () => undefined,
    });
    await transport.connect("http://127.0.0.1:7443");
    await transport.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));

    const fakeChannel = new FakeChannel();
    const channel = fakeChannel as unknown as RTCDataChannel;
    let opened: RTCDataChannel | null = null;
    const withChannel = new HttpSignalTransport({
      fetch: (async () => jsonResponse({ ok: true })) as unknown as typeof fetch,
      peerConnection: factory,
      onSignal: () => undefined,
      onDataChannel: (received) => {
        opened = received;
      },
    });
    await withChannel.connect("http://127.0.0.1:7443");
    await withChannel.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));
    lastPeer()?.openChannel(channel);
    expect(opened as RTCDataChannel | null).toBe(channel);
    expect(withChannel.channel).toBe(channel);

    fakeChannel.readyState = "connecting";
    expect(withChannel.sendInput({ kind: "focus", focused: true })).toBe(false);
    fakeChannel.readyState = "open";
    expect(
      withChannel.sendInput({ kind: "pointer-motion", dx: 4, dy: -2 }),
    ).toBe(true);
    expect(fakeChannel.sent).toEqual([
      '{"kind":"pointer-motion","dx":4,"dy":-2}',
    ]);
  });

  test("bye posts and close tears the peer connection down", async () => {
    const bodies: string[] = [];
    const fetchImpl = (async (_url: string | URL | Request, init?: RequestInit) => {
      bodies.push(String(init?.body));
      return jsonResponse({ ok: true });
    }) as unknown as typeof fetch;
    const transport = new HttpSignalTransport({
      fetch: fetchImpl,
      peerConnection: factory,
      onSignal: () => undefined,
    });
    await transport.connect("http://127.0.0.1:7443");
    await transport.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));
    const pc = lastPeer();

    transport.send({ type: "bye", reason: "test" });
    transport.close();
    await flush();

    expect(JSON.parse(bodies[0] ?? "{}")).toEqual({
      type: "bye",
      reason: "test",
    });
    expect(pc?.closed).toBe(true);
    expect(transport.peerConnection).toBeNull();
  });

  test("reports connection state changes", async () => {
    const states: RTCPeerConnectionState[] = [];
    const transport = new HttpSignalTransport({
      fetch: (async () => jsonResponse({ ok: true })) as unknown as typeof fetch,
      peerConnection: factory,
      onSignal: () => undefined,
      onConnectionState: (state) => states.push(state),
    });
    await transport.connect("http://127.0.0.1:7443");
    await transport.handleOffer(JSON.stringify({ type: "offer", sdp: "v=0" }));
    lastPeer()?.setConnectionState("connected");
    expect(states).toEqual(["connected"]);
  });
});

describe("waitForIceGathering", () => {
  test("resolves immediately when gathering is already complete", async () => {
    const peer = new FakePeerConnection({});
    await waitForIceGathering(peer as unknown as RTCPeerConnection, 50);
  });

  test("resolves on the gathering state event", async () => {
    const peer = new FakePeerConnection({});
    peer.iceGatheringState = "gathering";
    const pending = waitForIceGathering(peer as unknown as RTCPeerConnection, 50);
    peer.iceGatheringState = "complete";
    peer.dispatch("icegatheringstatechange");
    await pending;
  });
});
