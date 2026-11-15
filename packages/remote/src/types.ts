export type Tier = "webrtc" | "ws-au" | "ws-jpeg" | "vnc";
export type Codec = "h264" | "hevc" | "av1";

export interface ClientCaps {
  webrtc: boolean;
  websocket: boolean;
  webcodecs: boolean;
  jpeg: boolean;
  h264: boolean;
  hevc: boolean;
  av1: boolean;
}

export interface SignalHello {
  type: "hello";
  version: number;
  pairing_code: string;
  client_name: string;
  udp_blocked: boolean;
  caps: ClientCaps;
}

export interface SignalWelcome {
  type: "welcome";
  version: number;
  session_id: string;
  token: string;
  expires_at_ms: number;
  tier: Tier;
  codec: Codec;
  dtls_fingerprint: string;
  policy: {
    allow_input: boolean;
    allow_clipboard: boolean;
    allow_audio: boolean;
    max_bitrate_kbps: number;
    session_ttl_ms: number;
  };
}

export type SignalMessage =
  | SignalHello
  | SignalWelcome
  | { type: "offer"; sdp: string }
  | { type: "answer"; sdp: string }
  | { type: "candidate"; candidate: string; mid: string }
  | { type: "bye"; reason: string }
  | { type: "error"; code: string; message: string };

export const SIGNAL_VERSION = 1;

export function isSignalMessage(value: unknown): value is SignalMessage {
  if (typeof value !== "object" || value === null) return false;
  const type = (value as { type?: unknown }).type;
  return (
    type === "hello" ||
    type === "welcome" ||
    type === "offer" ||
    type === "answer" ||
    type === "candidate" ||
    type === "bye" ||
    type === "error"
  );
}

export function parseSignalMessage(text: string): SignalMessage {
  const value: unknown = JSON.parse(text);
  if (!isSignalMessage(value)) throw new Error("unknown signaling message");
  return value;
}
