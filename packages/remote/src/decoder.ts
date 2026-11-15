import type { Codec, Tier } from "./types";
import type { DecodedWireFrame } from "./wire";

export interface SinkConfig {
  codec: Codec;
  tier: Tier;
  width: number;
  height: number;
}

export interface FrameSink {
  configure(config: SinkConfig): void;
  push(frame: DecodedWireFrame): void;
  close(): void;
}

/** Counts frames without decoding; used by the harness and tests. */
export class CountingSink implements FrameSink {
  config: SinkConfig | null = null;
  frames = 0;
  bytes = 0;
  lastSequence = -1;
  closed = false;

  configure(config: SinkConfig): void {
    this.config = config;
  }

  push(frame: DecodedWireFrame): void {
    this.frames += 1;
    this.bytes += frame.payload.length;
    this.lastSequence = frame.sequence;
  }

  close(): void {
    this.closed = true;
  }
}

export interface CanvasLike {
  width: number;
  height: number;
  getContext(context: "2d"): CanvasRenderingContext2D | null;
}

interface DecoderLike {
  configure(config: unknown): void;
  decode(chunk: unknown): void;
  close(): void;
}

interface ChunkConstructor {
  new (init: {
    type: "key" | "delta";
    timestamp: number;
    data: Uint8Array;
  }): unknown;
}

interface DecoderConstructor {
  new (init: { output: (frame: unknown) => void; error: (error: unknown) => void }): DecoderLike;
}

interface CodecSupportLike {
  isConfigSupported(config: { codec: string; codedWidth: number; codedHeight: number }): Promise<{
    supported: boolean;
  }>;
}

const CODEC_STRINGS: Record<Codec, string> = {
  h264: "avc1.42E01F",
  hevc: "hvc1.1.6.L93.B0",
  av1: "av01.0.04M.08",
};

/**
 * WebCodecs sink. `VideoDecoder` output frames are drawn directly to the canvas;
 * there is no YUV conversion and no CPU readback in the browser path.
 */
export class WebCodecsSink implements FrameSink {
  #canvas: CanvasLike;
  #decoder: DecoderLike;
  #chunk: ChunkConstructor;
  #config: SinkConfig | null = null;
  #frames = 0;

  constructor(canvas: CanvasLike, decoder: DecoderLike, chunk: ChunkConstructor) {
    this.#canvas = canvas;
    this.#decoder = decoder;
    this.#chunk = chunk;
  }

  static create(canvas: CanvasLike): WebCodecsSink | null {
    const globals = globalThis as unknown as {
      VideoDecoder?: DecoderConstructor;
      EncodedVideoChunk?: ChunkConstructor;
    };
    if (globals.VideoDecoder === undefined || globals.EncodedVideoChunk === undefined) {
      return null;
    }
    const Decoder = globals.VideoDecoder;
    const Chunk = globals.EncodedVideoChunk;
    const decoder = new Decoder({
      output: (frame) => {
        const context = canvas.getContext("2d");
        if (context !== null) {
          context.drawImage(frame as CanvasImageSource, 0, 0, canvas.width, canvas.height);
        }
        const closeable = frame as { close?: () => void };
        closeable.close?.();
      },
      error: () => undefined,
    });
    return new WebCodecsSink(canvas, decoder, Chunk);
  }

  configure(config: SinkConfig): void {
    this.#config = config;
    this.#decoder.configure({
      codec: CODEC_STRINGS[config.codec],
      codedWidth: config.width,
      codedHeight: config.height,
      optimizeForLatency: true,
    });
  }

  push(frame: DecodedWireFrame): void {
    if (this.#config === null) throw new Error("sink is not configured");
    this.#decoder.decode(
      new this.#chunk({
        type: frame.keyframe ? "key" : "delta",
        timestamp: frame.ptsUs,
        data: frame.payload,
      }),
    );
    this.#frames += 1;
  }

  get frames(): number {
    return this.#frames;
  }

  close(): void {
    this.#decoder.close();
  }
}

/**
 * JPEG-tile fallback sink. Each wire payload is one complete image; the browser
 * decodes it with the platform image decoder and paints it on the canvas.
 */
export class JpegTileSink implements FrameSink {
  #canvas: CanvasLike;
  frames = 0;

  constructor(canvas: CanvasLike) {
    this.#canvas = canvas;
  }

  configure(_config: SinkConfig): void {}

  push(frame: DecodedWireFrame): void {
    const blob = new Blob([frame.payload as BlobPart], { type: "image/jpeg" });
    void createImageBitmap(blob).then((bitmap) => {
      const context = this.#canvas.getContext("2d");
      context?.drawImage(bitmap, 0, 0, this.#canvas.width, this.#canvas.height);
      bitmap.close();
    });
    this.frames += 1;
  }

  close(): void {}
}

/** Feature-detect the best sink for a tier. */
export function createSink(canvas: CanvasLike, tier: Tier): FrameSink | null {
  if (tier === "ws-jpeg") return new JpegTileSink(canvas);
  return WebCodecsSink.create(canvas);
}

export async function probeDecoder(codec: Codec): Promise<boolean> {
  const globals = globalThis as unknown as { VideoDecoder?: CodecSupportLike };
  if (globals.VideoDecoder === undefined) return false;
  try {
    const result = await globals.VideoDecoder.isConfigSupported({
      codec: CODEC_STRINGS[codec],
      codedWidth: 1920,
      codedHeight: 1080,
    });
    return result.supported;
  } catch {
    return false;
  }
}
