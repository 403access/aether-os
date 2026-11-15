import { useEffect, useRef, useState } from "react";
import {
  CountingSink,
  RemoteClient,
  encodeWireFrame,
  type ClientCaps,
  type ClientStats,
  type RemoteTransport,
  type SignalMessage,
} from "@aether/remote";

const CAPS: ClientCaps = {
  webrtc: true,
  websocket: true,
  webcodecs: true,
  jpeg: true,
  h264: true,
  hevc: false,
  av1: true,
};

const PAIRING_CODE = "K7QM-2H4P";

class DemoSocket implements RemoteTransport {
  sent: SignalMessage[] = [];
  closed = false;

  async connect(): Promise<void> {}

  send(message: SignalMessage): void {
    this.sent.push(message);
  }

  async handleOffer(): Promise<string> {
    return "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n";
  }

  close(): void {
    this.closed = true;
  }
}

interface View {
  phase: "idle" | "pairing" | "streaming" | "degraded" | "closed";
  stats: ClientStats | null;
  tier: string;
  codec: string;
  session: string;
  frames: number;
}

const INITIAL: View = {
  phase: "idle",
  stats: null,
  tier: "webrtc",
  codec: "h264",
  session: "—",
  frames: 0,
};

export function Remote() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const clientRef = useRef<RemoteClient | null>(null);
  const timerRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);
  const sequenceRef = useRef(0);
  const [view, setView] = useState<View>(INITIAL);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) window.clearInterval(timerRef.current);
      if (rafRef.current !== null) window.cancelAnimationFrame(rafRef.current);
      clientRef.current?.stop("unmounted");
    };
  }, []);

  const paint = () => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const context = canvas.getContext("2d");
    if (context === null) return;
    const width = canvas.width;
    const height = canvas.height;
    const phase = (sequenceRef.current % 360) / 360;

    const gradient = context.createLinearGradient(0, 0, width, height);
    gradient.addColorStop(0, "#031922");
    gradient.addColorStop(0.55, "#062b38");
    gradient.addColorStop(1, "#0b3d4c");
    context.fillStyle = gradient;
    context.fillRect(0, 0, width, height);

    context.fillStyle = "rgba(56, 214, 255, 0.07)";
    for (let x = -80; x < width + 80; x += 42) {
      const y = Math.sin((x / width + phase) * Math.PI * 2) * 22;
      context.fillRect(x, 90 + y, 18, height - 180);
    }

    const drawWindow = (x: number, y: number, w: number, h: number, label: string) => {
      context.fillStyle = "rgba(7, 39, 51, 0.94)";
      context.fillRect(x, y, w, h);
      context.fillStyle = "rgba(11, 57, 71, 1)";
      context.fillRect(x, y, w, 34);
      context.strokeStyle = "rgba(56, 214, 255, 0.38)";
      context.lineWidth = 1;
      context.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
      context.fillStyle = "#e9fbff";
      context.font = "600 14px ui-monospace, SFMono-Regular, Menlo, monospace";
      context.fillText(label, x + 14, y + 22);
      context.fillStyle = "rgba(165, 219, 229, 0.48)";
      for (let line = 0; line < 7; line += 1) {
        const lineWidth = w * (0.74 - line * 0.06);
        context.fillRect(x + 16, y + 58 + line * 22, lineWidth, 5);
      }
    };

    drawWindow(54, 70, width * 0.5, height - 160, "aether-shell — workspace 1");
    drawWindow(width * 0.58, 96, width * 0.34, height - 230, "aether-index");
    drawWindow(width * 0.56, 268, width * 0.38, height - 330, "Chat · DeepSeek");

    // Command palette.
    const paletteW = 340;
    const paletteX = (width - paletteW) / 2;
    context.fillStyle = "rgba(13, 61, 75, 0.97)";
    context.fillRect(paletteX, 132, paletteW, 214);
    context.strokeStyle = "#38d6ff";
    context.strokeRect(paletteX + 0.5, 132.5, paletteW - 1, 213);
    context.fillStyle = "#e9fbff";
    context.font = "700 18px ui-sans-serif, system-ui, sans-serif";
    context.fillText("Command palette", paletteX + 18, 166);
    context.fillStyle = "rgba(3, 25, 34, 0.9)";
    context.fillRect(paletteX + 16, 180, paletteW - 32, 30);
    context.fillStyle = "#a5dbe5";
    context.font = "600 14px ui-monospace, SFMono-Regular, Menlo, monospace";
    context.fillText("> index", paletteX + 26, 201);
    ["Rebuild search index", "Open remote session", "Switch workspace 2"].forEach(
      (row, index) => {
        if (index === 1) {
          context.fillStyle = "rgba(56, 214, 255, 0.16)";
          context.fillRect(paletteX + 12, 224 + index * 38, paletteW - 24, 32);
        }
        context.fillStyle = index === 1 ? "#e9fbff" : "#a5dbe5";
        context.font = "500 13px ui-sans-serif, system-ui, sans-serif";
        context.fillText(row, paletteX + 22, 245 + index * 38);
      },
    );

    // Recording indicator is always visible while a session is live.
    context.fillStyle = "#ff5a5a";
    context.beginPath();
    context.arc(width - 42, 26, 5, 0, Math.PI * 2);
    context.fill();
    context.fillStyle = "#ffd7d7";
    context.font = "600 12px ui-sans-serif, system-ui, sans-serif";
    context.fillText("REC", width - 78, 31);

    context.strokeStyle = "#e9fbff";
    context.lineWidth = 1.5;
    context.strokeRect(width * 0.62 + Math.sin(phase * Math.PI * 2) * 30, height * 0.63, 12, 18);

    context.fillStyle = "rgba(3, 25, 34, 0.82)";
    context.fillRect(0, height - 26, width, 26);
    context.fillStyle = "#6ba9b6";
    context.font = "500 12px ui-monospace, SFMono-Regular, Menlo, monospace";
    context.fillText(
      "aether-shell · 1440p · 60 fps · damage-tracked · 0 queued frames",
      14,
      height - 9,
    );
  };

  const start = async () => {
    if (timerRef.current !== null) window.clearInterval(timerRef.current);
    sequenceRef.current = 0;
    const transport = new DemoSocket();
    const client = new RemoteClient({
      transport,
      sink: new CountingSink(),
      caps: CAPS,
      targetFps: 60,
      now: () => performance.now(),
    });
    clientRef.current = client;
    setView({ ...INITIAL, phase: "pairing" });
    await client.start(PAIRING_CODE, "wss://aether.local:7443/signal");
    await client.handle({
      type: "welcome",
      version: 1,
      session_id: "rs-demo-kestrel",
      token: "demo-session-token",
      expires_at_ms: Date.now() + 3_600_000,
      tier: "webrtc",
      codec: "h264",
      dtls_fingerprint: "sha-256 6D:2A:…",
      policy: {
        allow_input: true,
        allow_clipboard: false,
        allow_audio: false,
        max_bitrate_kbps: 12_000,
        session_ttl_ms: 3_600_000,
      },
    });
    await client.handle({ type: "offer", sdp: "v=0" });
    setView((current) => ({ ...current, phase: "streaming", session: client.sessionId }));

    timerRef.current = window.setInterval(() => {
      sequenceRef.current += 1;
      const sequence = sequenceRef.current;
      client.handleMedia(
        encodeWireFrame({
          tier: "webrtc",
          codec: "h264",
          keyframe: sequence % 120 === 1,
          jpeg: false,
          sequence,
          ptsUs: Math.round(sequence * 16_667),
          payload: new Uint8Array(1600),
        }),
      );
      if (sequence % 6 === 0) client.markRoundTrip(16 + (sequence % 11));
      if (sequence % 4 === 0) {
        setView({
          phase: "streaming",
          stats: client.stats(),
          tier: client.tier,
          codec: client.codec,
          session: client.sessionId,
          frames: client.stats().frames,
        });
      }
    }, 16);
    paint();
  };

  const stop = () => {
    if (timerRef.current !== null) window.clearInterval(timerRef.current);
    timerRef.current = null;
    clientRef.current?.stop("demo stopped");
    setView((current) => ({ ...current, phase: "closed" }));
  };

  useEffect(() => {
    paint();
    const loop = () => {
      if (view.phase === "streaming" || view.phase === "degraded") paint();
      rafRef.current = window.requestAnimationFrame(loop);
    };
    rafRef.current = window.requestAnimationFrame(loop);
    return () => {
      if (rafRef.current !== null) window.cancelAnimationFrame(rafRef.current);
    };
  }, [view.phase]);

  const live = view.phase === "streaming" || view.phase === "degraded";
  const latency = view.stats?.latencyP99Ms;
  const bitrate = view.stats?.bitrateKbps ?? 0;

  return (
    <section className="section remote" id="remote">
      <div className="section__head">
        <p className="eyebrow">Remote display</p>
        <h2>The desktop, in a tab.</h2>
        <p className="section__lede">
          One fast surface also means one honest remote path: capture the compositor&apos;s
          frames, encode them without a CPU copy, and decode straight into a canvas. This
          viewer is the real client state machine — the frames below come from a synthetic
          loopback source because the marketing site has no compositor to capture.
        </p>
      </div>

      <div className="remote-shell">
        <aside className="remote-pair" aria-label="Session pairing">
          <p className="eyebrow">On the host</p>
          <div className="remote-code" aria-label="Pairing code">
            {PAIRING_CODE}
          </div>
          <p className="remote-pair__hint">
            One-shot code · expires in 120 s · DTLS-SRTP · no public listener
          </p>
          <dl className="remote-pair__facts">
            <div>
              <dt>Transport</dt>
              <dd>WebRTC + WebCodecs</dd>
            </div>
            <div>
              <dt>Encode</dt>
              <dd>H.264 · software fallback</dd>
            </div>
            <div>
              <dt>Input</dt>
              <dd>scancodes only</dd>
            </div>
            <div>
              <dt>Clipboard</dt>
              <dd>off by default</dd>
            </div>
          </dl>
          <div className="remote-pair__actions">
            <button
              type="button"
              className="ds-btn ds-btn--primary"
              onClick={() => void start()}
              disabled={live}
            >
              {live ? "Session live" : "Start session"}
            </button>
            <button
              type="button"
              className="ds-btn ds-btn--ghost"
              onClick={stop}
              disabled={!live && view.phase !== "pairing"}
            >
              Stop
            </button>
          </div>
          <p className="remote-pair__state" role="status" aria-live="polite">
            {view.phase === "idle" && "Waiting for pairing"}
            {view.phase === "pairing" && "Pairing…"}
            {view.phase === "streaming" && `Streaming · session ${view.session}`}
            {view.phase === "degraded" && "Degraded · fallback"}
            {view.phase === "closed" && "Session closed"}
          </p>
        </aside>

        <div className="remote-window">
          <div className="remote-window__bar">
            <span className="traffic" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
            <span className="remote-window__url">https://aether.local:7443/remote</span>
            <span className={`remote-rec${live ? " remote-rec--live" : ""}`}>
              <i aria-hidden="true" />
              {live ? "recording" : "ready"}
            </span>
          </div>
          <div className="remote-window__stage">
            <canvas
              ref={canvasRef}
              width={960}
              height={540}
              className="remote-canvas"
              aria-label="Simulated Aether desktop streamed into the browser client"
            />
            <div className="remote-hud" data-testid="remote-hud">
              <span>{view.tier === "webrtc" ? "WebRTC" : view.tier}</span>
              <span>{view.codec.toUpperCase()}</span>
              <span>{latency === null || latency === undefined ? "—" : `${latency.toFixed(1)} ms p99`}</span>
              <span>{bitrate > 0 ? `${(bitrate / 1000).toFixed(1)} Mbps` : "— Mbps"}</span>
              <span>{view.frames} frames</span>
            </div>
          </div>
          <div className="remote-window__foot">
            <span>Tier 1 · interactive</span>
            <span>{live ? "Glass-to-glass budget 40 ms (target)" : "No session"}</span>
            <span>{view.stats?.intervalBudgetOk === false ? "interval over budget" : "pacing ok"}</span>
          </div>
        </div>
      </div>
      <p className="remote-note">
        The Rust host harness and the browser client exchange the same signaling and wire
        formats. Glass-to-glass and zero-copy numbers remain targets until the reference
        machine run — see <a href="/docs/reference/claims-and-evidence/">claims and evidence</a>.
      </p>
    </section>
  );
}
