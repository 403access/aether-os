import type { Codec, Tier } from "./types";

export const MEDIA_MAGIC = "AEM1";
export const MEDIA_VERSION = 1;
export const HEADER_LEN = 24;

export interface WireFrame {
  tier: Tier;
  codec: Codec;
  keyframe: boolean;
  jpeg: boolean;
  sequence: number;
  ptsUs: number;
  payload: Uint8Array;
}

const TIER_BYTES: Record<Tier, number> = {
  webrtc: 1,
  "ws-au": 2,
  "ws-jpeg": 3,
  vnc: 4,
};
const CODEC_BYTES: Record<Codec, number> = { h264: 1, hevc: 2, av1: 3 };

export function encodeWireFrame(frame: WireFrame): Uint8Array {
  const bytes = new Uint8Array(HEADER_LEN + frame.payload.length);
  bytes.set([0x41, 0x45, 0x4d, 0x31], 0);
  const view = new DataView(bytes.buffer);
  bytes[4] = MEDIA_VERSION;
  bytes[5] = TIER_BYTES[frame.tier];
  bytes[6] = CODEC_BYTES[frame.codec];
  bytes[7] = (frame.keyframe ? 1 : 0) | (frame.jpeg ? 2 : 0);
  view.setUint32(8, frame.sequence, true);
  view.setBigUint64(12, BigInt(frame.ptsUs), true);
  view.setUint32(20, frame.payload.length, true);
  bytes.set(frame.payload, HEADER_LEN);
  return bytes;
}

export interface DecodedWireFrame extends WireFrame {
  /** Presentation timestamp in milliseconds, ready for `VideoFrame`. */
  ptsMs: number;
}

export function decodeWireFrame(input: ArrayBuffer | Uint8Array): DecodedWireFrame {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes.length < HEADER_LEN) throw new Error("media frame is shorter than the header");
  if (MEDIA_MAGIC !== String.fromCharCode(...bytes.slice(0, 4))) {
    throw new Error("media frame magic is not AEM1");
  }
  if (bytes[4] !== MEDIA_VERSION) throw new Error(`unsupported media frame version ${bytes[4]}`);
  const tier = decodeTier(bytes[5] ?? 0);
  const codec = decodeCodec(bytes[6] ?? 0);
  const flags = bytes[7] ?? 0;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const sequence = view.getUint32(8, true);
  const ptsUs = Number(view.getBigUint64(12, true));
  const length = view.getUint32(20, true);
  if (bytes.length < HEADER_LEN + length) {
    throw new Error(`media frame truncated: expected ${HEADER_LEN + length}, got ${bytes.length}`);
  }
  return {
    tier,
    codec,
    keyframe: (flags & 1) !== 0,
    jpeg: (flags & 2) !== 0,
    sequence,
    ptsUs,
    ptsMs: ptsUs / 1000,
    payload: bytes.slice(HEADER_LEN, HEADER_LEN + length),
  };
}

function decodeTier(value: number): Tier {
  switch (value) {
    case 1:
      return "webrtc";
    case 2:
      return "ws-au";
    case 3:
      return "ws-jpeg";
    case 4:
      return "vnc";
    default:
      throw new Error(`unknown transport tier ${value}`);
  }
}

function decodeCodec(value: number): Codec {
  switch (value) {
    case 1:
      return "h264";
    case 2:
      return "hevc";
    case 3:
      return "av1";
    default:
      throw new Error(`unknown codec ${value}`);
  }
}
