/**
 * Drive the browser client through pairing, negotiation, streaming and a
 * fallback, then write a small evidence document. Run with:
 *
 *   bun run scripts/remote-client-report.ts
 */
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import {
  CountingSink,
  RemoteClient,
  encodeWireFrame,
  type ClientCaps,
  type RemoteTransport,
  type SignalMessage,
} from "@aether/remote";

const caps: ClientCaps = {
  webrtc: true,
  websocket: true,
  webcodecs: true,
  jpeg: true,
  h264: true,
  hevc: false,
  av1: true,
};

class LoopbackSocket implements RemoteTransport {
  sent: SignalMessage[] = [];
  closed = false;
  async connect(): Promise<void> {}
  send(message: SignalMessage): void {
    this.sent.push(message);
  }
  async handleOffer(): Promise<string> {
    return "v=0 loopback-answer";
  }
  close(): void {
    this.closed = true;
  }
}

const transport = new LoopbackSocket();
const sink = new CountingSink();
const client = new RemoteClient({
  transport,
  sink,
  caps,
  targetFps: 60,
  now: () => 0,
});

await client.start("K7QM2H4P", "wss://aether.local:7443/signal");
await client.handle({
  type: "welcome",
  version: 1,
  session_id: "rs-report-kestrel",
  token: "report-token",
  expires_at_ms: 3_600_000,
  tier: "webrtc",
  codec: "h264",
  dtls_fingerprint: "sha-256 report",
  policy: {
    allow_input: true,
    allow_clipboard: false,
    allow_audio: false,
    max_bitrate_kbps: 12_000,
    session_ttl_ms: 3_600_000,
  },
});
await client.handle({ type: "offer", sdp: "v=0 offer" });
client.onConnected();

for (let sequence = 1; sequence <= 60; sequence += 1) {
  client.handleMedia(
    encodeWireFrame({
      tier: "webrtc",
      codec: "h264",
      keyframe: sequence === 1,
      jpeg: false,
      sequence,
      ptsUs: Math.round(sequence * 16_667),
      payload: new Uint8Array(1200),
    }),
  );
  client.markRoundTrip(16 + (sequence % 7));
}
const beforeFallback = client.stats();
client.fallback("ws-au");
const afterFallback = client.stats();
client.onDisconnected("simulated UDP block");
client.stop("report complete");

const report = {
  state_after_stop: client.state,
  session_id: client.sessionId,
  tier: afterFallback.tier,
  degraded: afterFallback.degraded,
  codec: client.codec,
  hello: transport.sent[0],
  answer_sent: transport.sent.some((message) => message.type === "answer"),
  transport_closed: transport.closed,
  sink_closed: sink.closed,
  frames_decoded: sink.frames,
  stats_before_fallback: beforeFallback,
  stats_after_fallback: afterFallback,
  last_error: client.lastError,
  note: "loopback transport + counting sink; no browser, decoder or network involved",
};

const path = resolve(import.meta.dir, "..", "artifacts", "p7", "client.json");
await mkdir(dirname(path), { recursive: true });
await writeFile(path, `${JSON.stringify(report, null, 2)}\n`);
console.log(`wrote ${path}`);
console.log(
  `client: ${report.stats_before_fallback.frames} frames, p99 ${report.stats_before_fallback.latencyP99Ms} ms, state ${report.state_after_stop}`,
);
