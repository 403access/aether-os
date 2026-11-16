import type { ClientInputEvent } from "./input";
import type { RemoteTransport } from "./client";
import type { SignalMessage } from "./types";

export type PeerConnectionFactory = (config: RTCConfiguration) => RTCPeerConnection;

export interface HttpSignalTransportOptions {
  /** Injected for tests; defaults to the global `fetch`. */
  fetch?: typeof fetch;
  /** Injected for tests; defaults to `new RTCPeerConnection(config)`. */
  peerConnection?: PeerConnectionFactory;
  /**
   * Delivered in order: the welcome first, then the offer. The caller feeds both
   * back into `RemoteClient.handle`.
   */
  onSignal: (message: SignalMessage) => void | Promise<void>;
  onTrack?: (stream: MediaStream, track: MediaStreamTrack) => void;
  onDataChannel?: (channel: RTCDataChannel) => void;
  onConnectionState?: (state: RTCPeerConnectionState) => void;
  onError?: (error: Error) => void;
  /** Bound on ICE gathering before the answer is posted. Default 3 s. */
  iceGatheringTimeoutMs?: number;
}

interface HelloResponse {
  welcome?: unknown;
  offer?: unknown;
  error?: unknown;
}

/**
 * The real signaling transport for `aether-remote serve`.
 *
 * The host exposes one plain HTTP endpoint (`POST /signal`): a hello answers
 * with a welcome and a str0m offer, an answer acknowledges with `{ok:true}` and
 * a bye closes the session. Media is WebRTC over UDP, so the browser uses its
 * own H.264 decoder and the remote track is attached to a `<video>` element.
 *
 * str0m serializes SDP as `{ "type": "offer" | "answer", "sdp": "v=0…" }`, which
 * is already the shape `RTCPeerConnection` accepts. The host nests that JSON in
 * the signal message, so `handleOffer` unwraps it first.
 */
export class HttpSignalTransport implements RemoteTransport {
  #fetch: typeof fetch;
  #factory: PeerConnectionFactory;
  #onSignal: HttpSignalTransportOptions["onSignal"];
  #onTrack: HttpSignalTransportOptions["onTrack"];
  #onDataChannel: HttpSignalTransportOptions["onDataChannel"];
  #onConnectionState: HttpSignalTransportOptions["onConnectionState"];
  #onError: HttpSignalTransportOptions["onError"];
  #iceGatheringTimeoutMs: number;

  #url = "";
  #pc: RTCPeerConnection | null = null;
  #channel: RTCDataChannel | null = null;
  #closed = true;

  constructor(options: HttpSignalTransportOptions) {
    this.#fetch = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.#factory = options.peerConnection ?? ((config) => new RTCPeerConnection(config));
    this.#onSignal = options.onSignal;
    this.#onTrack = options.onTrack;
    this.#onDataChannel = options.onDataChannel;
    this.#onConnectionState = options.onConnectionState;
    this.#onError = options.onError;
    this.#iceGatheringTimeoutMs = options.iceGatheringTimeoutMs ?? 3_000;
  }

  get peerConnection(): RTCPeerConnection | null {
    return this.#pc;
  }

  get channel(): RTCDataChannel | null {
    return this.#channel;
  }

  get connectionState(): RTCPeerConnectionState {
    return this.#pc?.connectionState ?? "closed";
  }

  async connect(url: string): Promise<void> {
    this.#url = normalizeSignalUrl(url);
    this.#closed = false;
  }

  send(message: SignalMessage): void {
    if (this.#closed && message.type !== "bye") return;
    switch (message.type) {
      case "hello":
        void this.#hello(message);
        return;
      case "answer":
      case "bye":
        void this.#post(message).catch((error: unknown) => this.#fail(error));
        return;
      case "candidate":
        // The host accepts one answer SDP and has no trickle endpoint.
        return;
    }
  }

  /**
   * Turn the host offer into a browser answer. ICE is not trickled, so wait for
   * gathering (bounded) and post an answer that already carries candidates.
   */
  async handleOffer(sdp: string): Promise<string> {
    const offer = parseSdp(sdp);
    const pc = this.#factory({ iceServers: [] });
    this.#pc = pc;

    pc.ontrack = (event: RTCTrackEvent) => {
      const stream = event.streams[0];
      if (stream !== undefined) this.#onTrack?.(stream, event.track);
    };
    pc.ondatachannel = (event: RTCDataChannelEvent) => {
      this.#channel = event.channel;
      this.#onDataChannel?.(event.channel);
    };
    pc.onconnectionstatechange = () => {
      this.#onConnectionState?.(pc.connectionState);
    };

    await pc.setRemoteDescription({ type: "offer", sdp: offer.sdp });
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);
    await waitForIceGathering(pc, this.#iceGatheringTimeoutMs);

    const local = pc.localDescription;
    if (local === null) throw new Error("peer connection produced no local answer");
    return JSON.stringify({ type: "answer", sdp: local.sdp });
  }

  /** Send one validated input event over the host's `input` data channel. */
  sendInput(event: ClientInputEvent | string): boolean {
    const channel = this.#channel;
    if (channel === null || channel.readyState !== "open") return false;
    channel.send(typeof event === "string" ? event : JSON.stringify(event));
    return true;
  }

  close(): void {
    this.#closed = true;
    this.#channel?.close();
    this.#channel = null;
    this.#pc?.close();
    this.#pc = null;
  }

  async #hello(message: SignalMessage): Promise<void> {
    try {
      const data = (await this.#post(message)) as HelloResponse;
      if (data.welcome === undefined) throw new Error("host did not return a welcome");
      if (typeof data.offer !== "string") throw new Error("host did not return an SDP offer");
      await this.#onSignal(data.welcome as SignalMessage);
      await this.#onSignal({ type: "offer", sdp: data.offer });
    } catch (error) {
      this.#fail(error);
    }
  }

  async #post(message: SignalMessage): Promise<unknown> {
    const response = await this.#fetch(this.#url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(message),
      // The bye must survive the page closing the session immediately.
      keepalive: message.type === "bye",
    });
    const text = await response.text();
    let data: unknown = {};
    if (text.length > 0) {
      try {
        data = JSON.parse(text);
      } catch {
        data = { error: text };
      }
    }
    if (!response.ok) {
      const detail =
        typeof data === "object" && data !== null && typeof (data as { error?: unknown }).error === "string"
          ? (data as { error: string }).error
          : `signaling failed (${response.status})`;
      throw new Error(detail);
    }
    return data;
  }

  #fail(error: unknown): void {
    const normalized = error instanceof Error ? error : new Error(String(error));
    this.#onError?.(normalized);
  }
}

/** A bare host becomes `…/signal`; an explicit path is preserved. */
export function normalizeSignalUrl(url: string): string {
  const trimmed = url.trim();
  if (trimmed.length === 0) throw new Error("a signaling URL is required");
  return trimmed.endsWith("/signal") || trimmed.includes("/signal?")
    ? trimmed
    : new URL("signal", trimmed.endsWith("/") ? trimmed : `${trimmed}/`).toString();
}

/**
 * str0m sends `{"type":"offer","sdp":"v=0…"}` nested inside the signal message,
 * but accept a bare SDP string too so the transport is not coupled to one host.
 */
export function parseSdp(value: string): RTCSessionDescriptionInit {
  const text = value.trim();
  if (text.startsWith("v=")) return { type: "offer", sdp: text };
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("malformed SDP offer");
  }
  if (
    typeof parsed === "object" &&
    parsed !== null &&
    typeof (parsed as { sdp?: unknown }).sdp === "string"
  ) {
    return { type: "offer", sdp: (parsed as { sdp: string }).sdp };
  }
  throw new Error("malformed SDP offer");
}

type GatheringPeer = Pick<
  RTCPeerConnection,
  "iceGatheringState" | "addEventListener" | "removeEventListener"
>;

/**
 * Resolve when ICE gathering finishes, or after `timeoutMs`. A browser that
 * gathers slowly must not hold the answer forever; the host accepts the answer
 * without trickle, so an early resolution still works on a loopback candidate.
 */
export function waitForIceGathering(pc: GatheringPeer, timeoutMs = 3_000): Promise<void> {
  if (pc.iceGatheringState === "complete") return Promise.resolve();
  return new Promise((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      pc.removeEventListener("icegatheringstatechange", check);
      clearTimeout(timer);
      resolve();
    };
    const check = () => {
      if (pc.iceGatheringState === "complete") finish();
    };
    const timer = setTimeout(finish, Math.max(0, timeoutMs));
    pc.addEventListener("icegatheringstatechange", check);
  });
}
