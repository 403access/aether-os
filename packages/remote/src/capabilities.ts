import type { ClientCaps, Codec, Tier } from "./types";

export interface CapabilityScope {
  RTCPeerConnection?: unknown;
  WebSocket?: unknown;
  VideoDecoder?: unknown;
  createImageBitmap?: unknown;
  codecSupport?: Partial<Record<Codec, boolean>>;
}

interface DecoderSupport {
  isConfigSupported(config: { codec: string }): Promise<{ supported: boolean }>;
}

const CODEC_STRINGS: Record<Codec, string> = {
  h264: "avc1.42E01F",
  hevc: "hvc1.1.6.L93.B0",
  av1: "av01.0.04M.08",
};

export const HOST_CODEC_ORDER: Codec[] = ["av1", "hevc", "h264"];

export function detectCapabilities(scope: CapabilityScope = globalThis): ClientCaps {
  const webcodecs = typeof scope.VideoDecoder !== "undefined";
  const support = scope.codecSupport ?? {};
  return {
    webrtc: typeof scope.RTCPeerConnection === "function",
    websocket: typeof scope.WebSocket === "function",
    webcodecs,
    jpeg: typeof scope.createImageBitmap === "function",
    h264: support.h264 ?? webcodecs,
    hevc: support.hevc ?? false,
    av1: support.av1 ?? webcodecs,
  };
}

/**
 * Probe actual decoder configurations. Falls back to the synchronous detector
 * when the platform does not implement `isConfigSupported`.
 */
export async function probeCapabilities(
  scope: CapabilityScope = globalThis,
): Promise<ClientCaps> {
  const base = detectCapabilities(scope);
  const decoder = scope.VideoDecoder as DecoderSupport | undefined;
  if (decoder === undefined || typeof decoder.isConfigSupported !== "function") {
    return base;
  }
  const probe = async (codec: Codec) => {
    try {
      const result = await decoder.isConfigSupported({ codec: CODEC_STRINGS[codec] });
      return result.supported;
    } catch {
      return false;
    }
  };
  const [h264, hevc, av1] = await Promise.all([probe("h264"), probe("hevc"), probe("av1")]);
  return {
    ...base,
    webcodecs: h264 || hevc || av1,
    h264,
    hevc,
    av1,
  };
}

export function selectTier(caps: ClientCaps, udpBlocked: boolean): Tier {
  if (caps.webrtc && !udpBlocked && caps.webcodecs && hasVideo(caps)) return "webrtc";
  if (caps.websocket && caps.webcodecs && hasVideo(caps)) return "ws-au";
  if (caps.websocket && caps.jpeg) return "ws-jpeg";
  return "vnc";
}

export function selectCodec(hostOrder: Codec[], caps: ClientCaps): Codec | null {
  return hostOrder.find((codec) => caps[codec]) ?? null;
}

function hasVideo(caps: ClientCaps): boolean {
  return caps.h264 || caps.hevc || caps.av1;
}

export interface TierInfo {
  tier: Tier;
  label: string;
  budgetMs: number;
  degraded: boolean;
}

export function tierInfo(tier: Tier): TierInfo {
  switch (tier) {
    case "webrtc":
      return { tier, label: "WebRTC + WebCodecs", budgetMs: 40, degraded: false };
    case "ws-au":
      return { tier, label: "WebSocket + access units", budgetMs: 120, degraded: true };
    case "ws-jpeg":
      return { tier, label: "WebSocket + JPEG tiles", budgetMs: 250, degraded: true };
    case "vnc":
      return { tier, label: "VNC (degraded)", budgetMs: 400, degraded: true };
  }
}
