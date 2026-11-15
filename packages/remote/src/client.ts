import type { ClientCaps, Codec, SignalMessage, Tier } from "./types";
import { SIGNAL_VERSION } from "./types";
import { selectTier, tierInfo } from "./capabilities";
import type { FrameSink } from "./decoder";
import { ClientStatsTracker, type ClientStats } from "./stats";
import { decodeWireFrame } from "./wire";

export type ClientState =
  | "idle"
  | "pairing"
  | "negotiating"
  | "streaming"
  | "degraded"
  | "closed";

export interface RemoteTransport {
  connect(url: string): Promise<void>;
  send(message: SignalMessage): void;
  /** WebRTC transport turns the host offer into an answer. */
  handleOffer?(sdp: string): Promise<string>;
  close(): void;
}

export interface RemoteClientOptions {
  transport: RemoteTransport;
  sink: FrameSink;
  caps: ClientCaps;
  targetFps?: number;
  udpBlocked?: boolean;
  now?: () => number;
}

export class RemoteClient {
  #transport: RemoteTransport;
  #sink: FrameSink;
  #caps: ClientCaps;
  #stats: ClientStatsTracker;
  #udpBlocked: boolean;
  #state: ClientState = "idle";
  #tier: Tier = "webrtc";
  #codec: Codec = "h264";
  #token = "";
  #sessionId = "";
  #expiresAtMs = 0;
  #allowInput = true;
  #allowClipboard = false;
  #lastError: string | null = null;

  constructor(options: RemoteClientOptions) {
    this.#transport = options.transport;
    this.#sink = options.sink;
    this.#caps = options.caps;
    this.#stats = new ClientStatsTracker(options.targetFps ?? 60, options.now);
    this.#udpBlocked = options.udpBlocked ?? false;
  }

  get state(): ClientState {
    return this.#state;
  }

  get tier(): Tier {
    return this.#tier;
  }

  get codec(): Codec {
    return this.#codec;
  }

  get token(): string {
    return this.#token;
  }

  get sessionId(): string {
    return this.#sessionId;
  }

  get allowInput(): boolean {
    return this.#allowInput;
  }

  get allowClipboard(): boolean {
    return this.#allowClipboard;
  }

  get lastError(): string | null {
    return this.#lastError;
  }

  get statsTracker(): ClientStatsTracker {
    return this.#stats;
  }

  async start(pairingCode: string, url: string): Promise<void> {
    if (this.#state !== "idle" && this.#state !== "closed") {
      throw new Error(`cannot start from state ${this.#state}`);
    }
    const code = pairingCode.replace(/[\s_-]/g, "").toUpperCase();
    if (!/^[2-9A-HJ-NP-Z]{8}$/.test(code)) {
      throw new Error("pairing code must be 8 characters");
    }
    this.#state = "pairing";
    this.#lastError = null;
    await this.#transport.connect(url);
    this.#state = "negotiating";
    this.#transport.send({
      type: "hello",
      version: SIGNAL_VERSION,
      pairing_code: code,
      client_name: "aether-browser",
      udp_blocked: this.#udpBlocked,
      caps: this.#caps,
    });
  }

  async handle(message: SignalMessage): Promise<void> {
    switch (message.type) {
      case "welcome": {
        if (message.version !== SIGNAL_VERSION) {
          throw new Error(`signaling version ${message.version}, expected ${SIGNAL_VERSION}`);
        }
        this.#sessionId = message.session_id;
        this.#token = message.token;
        this.#expiresAtMs = message.expires_at_ms;
        this.#tier = message.tier;
        this.#codec = message.codec;
        this.#allowInput = message.policy.allow_input;
        this.#allowClipboard = message.policy.allow_clipboard;
        this.#sink.configure({
          codec: this.#codec,
          tier: this.#tier,
          width: 1920,
          height: 1080,
        });
        this.#state = "negotiating";
        return;
      }
      case "offer": {
        if (this.#state === "idle" || this.#state === "closed") {
          throw new Error("offer before pairing");
        }
        const answer = await this.#transport.handleOffer?.(message.sdp);
        if (answer !== undefined) {
          this.#transport.send({ type: "answer", sdp: answer });
        }
        this.#state = tierInfo(this.#tier).degraded ? "degraded" : "streaming";
        return;
      }
      case "candidate":
        return;
      case "bye": {
        this.#transport.close();
        this.#sink.close();
        this.#state = "closed";
        return;
      }
      case "error": {
        this.#lastError = `${message.code}: ${message.message}`;
        throw new Error(this.#lastError);
      }
      case "answer":
      case "hello":
        return;
    }
  }

  /** A media packet from WebRTC's encoded stream or the WebSocket fallback. */
  handleMedia(bytes: ArrayBuffer | Uint8Array): void {
    if (this.#state !== "streaming" && this.#state !== "degraded") {
      throw new Error(`media before streaming (${this.#state})`);
    }
    const frame = decodeWireFrame(bytes);
    this.#sink.push(frame);
    this.#stats.onFrame(frame.sequence, frame.ptsMs, frame.payload.length);
  }

  onConnected(): void {
    if (this.#state === "negotiating" || this.#state === "degraded") {
      this.#state = tierInfo(this.#tier).degraded ? "degraded" : "streaming";
    }
  }

  onDisconnected(reason = "transport closed"): void {
    if (this.#state === "closed") return;
    this.#lastError = reason;
    this.#state = "degraded";
  }

  /** Network downgrade without restarting the session. */
  fallback(tier: Tier): void {
    if (this.#state === "closed") return;
    this.#tier = tier;
    this.#sink.configure({
      codec: this.#codec,
      tier,
      width: 1920,
      height: 1080,
    });
    this.#state = tierInfo(tier).degraded ? "degraded" : "streaming";
  }

  markRoundTrip(ms: number): void {
    this.#stats.onRoundTrip(ms);
  }

  stats(): ClientStats {
    return this.#stats.snapshot(this.#tier, tierInfo(this.#tier).degraded);
  }

  stop(reason = "user stopped"): void {
    if (this.#state === "closed") return;
    try {
      this.#transport.send({ type: "bye", reason });
    } catch {
      // A dead transport is already closed.
    }
    this.#transport.close();
    this.#sink.close();
    this.#state = "closed";
  }

  /** Select the best tier from the current capabilities without a new hello. */
  preferredTier(): Tier {
    return selectTier(this.#caps, this.#udpBlocked);
  }

  get expiresAtMs(): number {
    return this.#expiresAtMs;
  }
}
