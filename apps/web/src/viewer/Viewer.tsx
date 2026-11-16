import { useCallback, useEffect, useRef, useState } from "react";
import {
  CountingSink,
  HttpSignalTransport,
  InputBatcher,
  RemoteClient,
  type ClientCaps,
} from "@aether/remote";

type Phase = "idle" | "connecting" | "live" | "closed" | "error";

interface LiveStats {
  resolution: string;
  fps: number;
  bitrateKbps: number;
  frames: number;
  rttMs: number | null;
}

const EMPTY_STATS: LiveStats = {
  resolution: "—",
  fps: 0,
  bitrateKbps: 0,
  frames: 0,
  rttMs: null,
};

function defaultSignalUrl(): string {
  if (typeof window === "undefined") return "http://127.0.0.1:7443/signal";
  const { protocol, hostname } = window.location;
  const scheme = protocol === "https:" ? "https" : "http";
  return `${scheme}://${hostname || "127.0.0.1"}:7443/signal`;
}

function initialCode(): string {
  if (typeof window === "undefined") return "";
  const params = new URLSearchParams(window.location.search);
  const value = params.get("code") ?? window.location.hash.replace(/^#code=/, "");
  return value.toUpperCase();
}

function initialSignalUrl(): string {
  if (typeof window === "undefined") return defaultSignalUrl();
  const params = new URLSearchParams(window.location.search);
  return params.get("signal") ?? defaultSignalUrl();
}

/**
 * The live page negotiates H.264 into a `<video>` element, so "can I decode"
 * is the media element's answer, not `VideoDecoder`'s. The host's tier ladder
 * uses the `webcodecs` flag as its "client can render video" bit.
 */
function viewerCaps(): ClientCaps {
  const probe = document.createElement("video");
  const canPlay = (type: string) => probe.canPlayType(type) !== "";
  const h264 = canPlay('video/mp4; codecs="avc1.42E01F"');
  const hevc = canPlay('video/mp4; codecs="hvc1.1.6.L93.B0"');
  const av1 = canPlay('video/mp4; codecs="av01.0.04M.08"');
  return {
    webrtc: typeof RTCPeerConnection === "function",
    websocket: typeof WebSocket === "function",
    webcodecs: h264 || hevc || av1,
    jpeg: typeof createImageBitmap === "function",
    h264,
    hevc,
    av1,
  };
}

function modifiers(event: KeyboardEvent | MouseEvent | WheelEvent): number {
  return (
    (event.shiftKey ? 1 : 0) |
    (event.ctrlKey ? 2 : 0) |
    (event.altKey ? 4 : 0) |
    (event.metaKey ? 8 : 0)
  );
}

function readStats(report: RTCStatsReport): {
  stats: LiveStats;
  bytes: number;
  rttMs: number | null;
} {
  const found: {
    inbound?: RTCInboundRtpStreamStats;
    pair?: RTCIceCandidatePairStats;
  } = {};
  report.forEach((entry) => {
    if (entry.type === "inbound-rtp") {
      const stat = entry as RTCInboundRtpStreamStats;
      if (stat.kind === "video") found.inbound = stat;
    } else if (entry.type === "candidate-pair") {
      const stat = entry as RTCIceCandidatePairStats;
      if (stat.nominated === true) found.pair = stat;
    }
  });
  const inbound = found.inbound;
  const pair = found.pair;
  const width = inbound?.frameWidth ?? 0;
  const height = inbound?.frameHeight ?? 0;
  const rtt = pair?.currentRoundTripTime;
  return {
    stats: {
      resolution: width > 0 && height > 0 ? `${width}×${height}` : "—",
      fps: Math.round(inbound?.framesPerSecond ?? 0),
      bitrateKbps: 0,
      frames: inbound?.framesDecoded ?? 0,
      rttMs: typeof rtt === "number" ? Math.round(rtt * 1_000) : null,
    },
    bytes: inbound?.bytesReceived ?? 0,
    rttMs: typeof rtt === "number" ? rtt * 1_000 : null,
  };
}

export function Viewer() {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const clientRef = useRef<RemoteClient | null>(null);
  const transportRef = useRef<HttpSignalTransport | null>(null);
  const channelRef = useRef<RTCDataChannel | null>(null);
  const batcherRef = useRef(new InputBatcher());
  const statsTimerRef = useRef<number | null>(null);
  const inputTimerRef = useRef<number | null>(null);
  const lastSampleRef = useRef<{ at: number; bytes: number } | null>(null);
  const focusedRef = useRef(false);

  const [url, setUrl] = useState(initialSignalUrl);
  const [code, setCode] = useState(initialCode);
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState("—");
  const [tier, setTier] = useState("—");
  const [codec, setCodec] = useState("—");
  const [channelState, setChannelState] = useState("closed");
  const [stats, setStats] = useState<LiveStats>(EMPTY_STATS);
  const [inputEnabled, setInputEnabled] = useState(false);
  const [pointerLocked, setPointerLocked] = useState(false);

  const live = phase === "live";

  const clearTimers = useCallback(() => {
    if (statsTimerRef.current !== null) window.clearInterval(statsTimerRef.current);
    if (inputTimerRef.current !== null) window.clearInterval(inputTimerRef.current);
    statsTimerRef.current = null;
    inputTimerRef.current = null;
  }, []);

  const reset = useCallback(() => {
    clearTimers();
    channelRef.current = null;
    clientRef.current = null;
    transportRef.current = null;
    lastSampleRef.current = null;
    if (videoRef.current !== null) videoRef.current.srcObject = null;
  }, [clearTimers]);

  useEffect(() => reset, [reset]);

  const stop = useCallback(
    (reason = "viewer stopped") => {
      clientRef.current?.stop(reason);
      reset();
      setChannelState("closed");
      setInputEnabled(false);
      setPointerLocked(false);
      focusedRef.current = false;
      batcherRef.current.setFocused(false, performance.now());
      setPhase("closed");
    },
    [reset],
  );

  const startStats = useCallback(() => {
    statsTimerRef.current = window.setInterval(() => {
      const pc = transportRef.current?.peerConnection;
      if (pc === null || pc === undefined) return;
      void pc.getStats().then((report) => {
        const sample = readStats(report);
        const now = performance.now();
        const previous = lastSampleRef.current;
        const bitrateKbps =
          previous !== null && now > previous.at
            ? Math.round(((sample.bytes - previous.bytes) * 8) / (now - previous.at))
            : 0;
        lastSampleRef.current = { at: now, bytes: sample.bytes };
        if (sample.rttMs !== null) clientRef.current?.markRoundTrip(sample.rttMs);
        setStats({ ...sample.stats, bitrateKbps });
      }).catch(() => undefined);
    }, 500);
  }, []);

  const startInput = useCallback(() => {
    inputTimerRef.current = window.setInterval(() => {
      const drained = batcherRef.current.drain();
      const transport = transportRef.current;
      if (transport === null) return;
      for (const event of drained.events) transport.sendInput(event);
    }, 16);
  }, []);

  const connect = useCallback(async () => {
    if (phase === "connecting") return;
    reset();
    setError(null);
    setStats(EMPTY_STATS);
    setSession("—");
    setTier("—");
    setCodec("—");
    setChannelState("closed");
    setPhase("connecting");

    let transport!: HttpSignalTransport;
    const client = new RemoteClient({
      transport: {
        connect: (target) => transport.connect(target),
        send: (message) => transport.send(message),
        handleOffer: (sdp) => transport.handleOffer(sdp),
        close: () => transport.close(),
      },
      sink: new CountingSink(),
      caps: viewerCaps(),
      targetFps: 60,
    });

    transport = new HttpSignalTransport({
      onSignal: async (message) => {
        await client.handle(message);
        setTier(client.tier);
        setCodec(client.codec);
        setSession(client.sessionId || "—");
      },
      onTrack: (stream) => {
        const video = videoRef.current;
        if (video !== null) {
          video.srcObject = stream;
          void video.play().catch(() => undefined);
        }
        setPhase("live");
      },
      onDataChannel: (channel) => {
        channelRef.current = channel;
        setChannelState(channel.readyState);
        channel.onopen = () => setChannelState("open");
        channel.onclose = () => setChannelState("closed");
      },
      onConnectionState: (state) => {
        if (state === "connected") setPhase("live");
        if (state === "failed") {
          setError("ICE failed: the host candidate is unreachable");
          setPhase("error");
        }
      },
      onError: (failure) => {
        setError(failure.message);
        setPhase("error");
      },
    });

    clientRef.current = client;
    transportRef.current = transport;
    batcherRef.current = new InputBatcher();
    batcherRef.current.setFocused(false, performance.now());

    try {
      await client.start(code, url);
      startStats();
      startInput();
    } catch (failure) {
      const message = failure instanceof Error ? failure.message : String(failure);
      setError(message);
      setPhase("error");
      transport.close();
    }
  }, [code, phase, reset, startInput, startStats, url]);

  useEffect(() => {
    const video = videoRef.current;
    if (video === null) return;

    const onPointerLockChange = () => {
      const locked = document.pointerLockElement === video;
      setPointerLocked(locked);
      if (inputEnabled) {
        focusedRef.current = locked;
        batcherRef.current.setFocused(locked, performance.now());
      }
    };
    const onPointerMove = (event: MouseEvent) => {
      if (!focusedRef.current) return;
      batcherRef.current.pushMotion(event.movementX, event.movementY, performance.now());
    };
    const onPointerDown = (event: MouseEvent) => {
      if (!focusedRef.current) return;
      batcherRef.current.pushButton(event.button, true, performance.now());
    };
    const onPointerUp = (event: MouseEvent) => {
      if (!focusedRef.current) return;
      batcherRef.current.pushButton(event.button, false, performance.now());
    };
    const onWheel = (event: WheelEvent) => {
      if (!focusedRef.current) return;
      event.preventDefault();
      batcherRef.current.pushWheel(event.deltaX, event.deltaY, performance.now());
    };
    const onKey = (event: KeyboardEvent, pressed: boolean) => {
      const target = event.target as HTMLElement | null;
      if (target !== null && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (!focusedRef.current) return;
      batcherRef.current.pushKey(event.code, pressed, modifiers(event), performance.now());
      event.preventDefault();
    };
    const onKeyDown = (event: KeyboardEvent) => onKey(event, true);
    const onKeyUp = (event: KeyboardEvent) => onKey(event, false);

    document.addEventListener("pointerlockchange", onPointerLockChange);
    document.addEventListener("mousemove", onPointerMove);
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("mouseup", onPointerUp);
    video.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      document.removeEventListener("pointerlockchange", onPointerLockChange);
      document.removeEventListener("mousemove", onPointerMove);
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("mouseup", onPointerUp);
      video.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [inputEnabled]);

  const requestLock = () => {
    const video = videoRef.current;
    if (video === null) return;
    const result = video.requestPointerLock?.() as unknown;
    if (result instanceof Promise) void result.catch(() => undefined);
  };

  const toggleInput = (enabled: boolean) => {
    setInputEnabled(enabled);
    focusedRef.current = enabled;
    batcherRef.current.setFocused(enabled, performance.now());
    if (enabled) requestLock();
    if (!enabled && document.pointerLockElement !== null) document.exitPointerLock();
  };

  const status =
    phase === "idle"
      ? "Waiting for pairing"
      : phase === "connecting"
        ? "Pairing…"
        : phase === "live"
          ? `Streaming · session ${session}`
          : phase === "closed"
            ? "Session closed"
            : "Session error";

  return (
    <main className="viewer">
      <header className="viewer__head">
        <a className="viewer__brand" href="./">
          Aether OS
        </a>
        <p className="viewer__eyebrow">Remote display · live viewer</p>
        <h1>The desktop, in a tab.</h1>
        <p className="viewer__lede">
          Pair with a local <code>aether-remote serve</code>. The page posts the hello,
          answers the str0m offer, renders the H.264 track and sends scancodes over the
          input channel. Nothing leaves the loopback interface.
        </p>
      </header>

      <section className="viewer__pair" aria-label="Pairing">
        <label className="viewer__field">
          <span>Signaling URL</span>
          <input
            type="url"
            value={url}
            spellCheck={false}
            onChange={(event) => setUrl(event.target.value)}
            disabled={phase === "connecting" || live}
          />
        </label>
        <label className="viewer__field">
          <span>Pairing code</span>
          <input
            type="text"
            value={code}
            placeholder="K7QM-2H4P"
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => setCode(event.target.value.toUpperCase())}
            disabled={phase === "connecting" || live}
          />
        </label>
        <div className="viewer__actions">
          <button
            type="button"
            className="viewer__button viewer__button--primary"
            onClick={() => void connect()}
            disabled={phase === "connecting" || live}
          >
            {phase === "connecting" ? "Pairing…" : "Connect"}
          </button>
          <button
            type="button"
            className="viewer__button"
            onClick={() => stop()}
            disabled={phase === "idle" || phase === "closed" || phase === "error"}
          >
            Stop
          </button>
          <label className="viewer__toggle">
            <input
              type="checkbox"
              checked={inputEnabled}
              onChange={(event) => toggleInput(event.target.checked)}
              disabled={!live}
            />
            Send input
          </label>
        </div>
        <p className="viewer__status" role="status" aria-live="polite">
          {status}
        </p>
        {error !== null && (
          <p className="viewer__error" role="alert">
            {error}
          </p>
        )}
      </section>

      <section className="viewer__stage" aria-label="Remote screen">
        <div className="viewer__chrome">
          <span className={`viewer__rec ${live ? "viewer__rec--live" : ""}`}>
            {live ? "recording" : "ready"}
          </span>
          <span>{pointerLocked ? "pointer locked" : inputEnabled ? "input armed" : "input off"}</span>
        </div>
        <video
          ref={videoRef}
          className="viewer__video"
          aria-label="Live Aether desktop streamed into the browser"
          playsInline
          muted
          onClick={() => {
            if (live && inputEnabled) requestLock();
          }}
        />
        {!live && (
          <p className="viewer__placeholder">
            {phase === "connecting"
              ? "Negotiating WebRTC…"
              : "No session. Start the host, enter its code and connect."}
          </p>
        )}
      </section>

      <dl className="viewer__hud" data-testid="viewer-hud">
        <div>
          <dt>Transport</dt>
          <dd>{tier}</dd>
        </div>
        <div>
          <dt>Codec</dt>
          <dd>{codec}</dd>
        </div>
        <div>
          <dt>Resolution</dt>
          <dd>{stats.resolution}</dd>
        </div>
        <div>
          <dt>FPS</dt>
          <dd>{stats.fps}</dd>
        </div>
        <div>
          <dt>Bitrate</dt>
          <dd>{stats.bitrateKbps} kbps</dd>
        </div>
        <div>
          <dt>RTT</dt>
          <dd>{stats.rttMs === null ? "—" : `${stats.rttMs} ms`}</dd>
        </div>
        <div>
          <dt>Frames</dt>
          <dd>{stats.frames}</dd>
        </div>
        <div>
          <dt>Input channel</dt>
          <dd>{channelState}</dd>
        </div>
      </dl>

      <footer className="viewer__foot">
        <p>
          Clipboard stays off. Input is scancodes and relative motion, rate-limited by
          the client; the host writes no command channel.
        </p>
        <a href="./docs/architecture/remote-display/">Remote display architecture</a>
      </footer>
    </main>
  );
}

