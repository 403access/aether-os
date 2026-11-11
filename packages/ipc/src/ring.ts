import {
  AETHER_RING_MAGIC,
  PROTOCOL_VERSION,
  RING_HEADER_BYTES,
  type MessageKind,
} from "@aether/protocol";

/**
 * Single-producer / single-consumer ring buffer over a byte array.
 *
 * This is the userspace reference implementation of `aether-ipc`. The daemon
 * maps the same layout from `memfd_create`; the only difference is that the
 * backing store is a file descriptor instead of a SharedArrayBuffer.
 *
 * Layout (little-endian):
 *
 *   header (64 bytes)
 *     0   u32  magic "AETH"
 *     4   u32  protocol version
 *     8   u32  capacity in payload bytes (multiple of 8)
 *     12  u32  reserved
 *     16  i64  head  (monotonic bytes produced)
 *     24  i64  tail  (monotonic bytes consumed)
 *     32  u32  dropped records (incremented when a push hits a full ring)
 *     36  ...  reserved
 *
 *   record
 *     0   u32  payload length, or 0xFFFFFFFF for a wrap marker
 *     4   u32  message kind
 *     8   u64  capability token (FNV-1a of the capability id, or 0)
 *     16  ...  payload, padded to an 8-byte boundary
 *
 * Guarantees:
 *  - exactly one producer and one consumer; the producer only writes `head`,
 *    the consumer only writes `tail`
 *  - a record is published only after its bytes are written (seq-cst store)
 *  - oversized records are rejected, never truncated
 *  - when a record does not fit contiguously, a wrap marker is written and the
 *    producer continues at offset 0; the consumer transparently skips padding
 */

export const RING_VERSION = PROTOCOL_VERSION;
export const WRAP_MARKER = 0xffff_ffff;

const OFFSET_MAGIC = 0;
const OFFSET_VERSION = 4;
const OFFSET_CAPACITY = 8;
const OFFSET_HEAD = 16;
const OFFSET_TAIL = 24;
const OFFSET_DROPPED = 32;
const RECORD_HEADER_BYTES = 16;
const MIN_CAPACITY = 256;

export type PushOutcome =
  | { ok: true; bytes: number }
  | { ok: false; reason: "full" | "too-large" };

export interface RingMessage {
  kind: MessageKind | number;
  capability: bigint;
  payload: Uint8Array;
}

export interface RingMessageView {
  kind: MessageKind | number;
  capability: bigint;
  /** View into the ring. Valid only for the duration of the callback. */
  payload: Uint8Array;
}

export interface RingStats {
  capacity: number;
  usedBytes: number;
  freeBytes: number;
  dropped: number;
}

export function align8(value: number): number {
  return (value + 7) & ~7;
}

export function fnv1a64(input: string): bigint {
  let hash = 0xcbf2_9ce4_8422_2325n;
  const prime = 0x100_0000_01b3n;
  for (let index = 0; index < input.length; index += 1) {
    hash ^= BigInt(input.charCodeAt(index));
    hash = (hash * prime) & 0xffff_ffff_ffff_ffffn;
  }
  return hash;
}

function isShared(buffer: ArrayBufferLike): buffer is SharedArrayBuffer {
  return typeof SharedArrayBuffer !== "undefined" && buffer instanceof SharedArrayBuffer;
}

export class RingBuffer {
  readonly buffer: ArrayBufferLike;
  readonly capacity: number;
  readonly #view: DataView;
  readonly #counters: BigInt64Array;
  readonly #shared: boolean;

  private constructor(buffer: ArrayBufferLike, capacity: number) {
    this.buffer = buffer;
    this.capacity = capacity;
    this.#view = new DataView(buffer);
    this.#counters = new BigInt64Array(buffer, OFFSET_HEAD, 2);
    this.#shared = isShared(buffer);
  }

  /**
   * Create a ring. Shared memory is the default because the intended use is
   * cross-process; pass `shared: false` for tests and single-threaded tools.
   */
  static create(capacityBytes = 64 * 1024, options: { shared?: boolean } = {}): RingBuffer {
    const capacity = align8(capacityBytes);
    if (capacity < MIN_CAPACITY) {
      throw new RangeError(`ring capacity must be at least ${MIN_CAPACITY} bytes`);
    }
    const shared = options.shared ?? true;
    const total = RING_HEADER_BYTES + capacity;
    const buffer = shared ? new SharedArrayBuffer(total) : new ArrayBuffer(total);
    const ring = new RingBuffer(buffer, capacity);

    const view = new DataView(buffer);
    view.setUint32(OFFSET_MAGIC, AETHER_RING_MAGIC, true);
    view.setUint32(OFFSET_VERSION, RING_VERSION, true);
    view.setUint32(OFFSET_CAPACITY, capacity, true);
    view.setUint32(OFFSET_DROPPED, 0, true);
    ring.#counters[0] = 0n;
    ring.#counters[1] = 0n;
    return ring;
  }

  /** Attach to an existing shared buffer (another process/worker). */
  static attach(buffer: ArrayBufferLike): RingBuffer {
    if (buffer.byteLength < RING_HEADER_BYTES + MIN_CAPACITY) {
      throw new RangeError("buffer is too small to contain an Aether ring");
    }
    const view = new DataView(buffer);
    const magic = view.getUint32(OFFSET_MAGIC, true);
    if (magic !== AETHER_RING_MAGIC) {
      throw new Error("not an Aether ring buffer (bad magic)");
    }
    const version = view.getUint32(OFFSET_VERSION, true);
    if (version !== RING_VERSION) {
      throw new Error(`unsupported ring version ${version} (expected ${RING_VERSION})`);
    }
    const capacity = view.getUint32(OFFSET_CAPACITY, true);
    if (capacity < MIN_CAPACITY || capacity + RING_HEADER_BYTES > buffer.byteLength) {
      throw new RangeError("ring capacity does not match the backing buffer");
    }
    return new RingBuffer(buffer, capacity);
  }

  get shared(): boolean {
    return this.#shared;
  }

  get usedBytes(): number {
    return Number(this.#loadHead() - this.#loadTail());
  }

  get freeBytes(): number {
    return this.capacity - this.usedBytes;
  }

  stats(): RingStats {
    return {
      capacity: this.capacity,
      usedBytes: this.usedBytes,
      freeBytes: this.freeBytes,
      dropped: this.#view.getUint32(OFFSET_DROPPED, true),
    };
  }

  /**
   * Publish one message. Never blocks and never partially writes a record:
   * either the whole record lands in the ring or the push is rejected.
   */
  tryPush(kind: number, capability: bigint, payload: Uint8Array): PushOutcome {
    if (!Number.isInteger(kind) || kind < 0) {
      throw new RangeError("kind must be a non-negative integer");
    }
    const recordSize = RECORD_HEADER_BYTES + payload.byteLength;
    if (recordSize > 0xffff_fffe) {
      return { ok: false, reason: "too-large" };
    }
    const aligned = align8(recordSize);
    if (aligned > this.capacity - 8) {
      return { ok: false, reason: "too-large" };
    }

    const head = this.#loadHead();
    const tail = this.#loadTail();
    const free = this.capacity - Number(head - tail);

    let position = Number(head % BigInt(this.capacity));
    let padding = 0;

    if (position + aligned > this.capacity) {
      // Not enough contiguous space at the end: write a wrap marker and
      // restart at zero. Both the padding and the record must fit.
      padding = this.capacity - position;
      if (free < padding + aligned) {
        this.#drop();
        return { ok: false, reason: "full" };
      }
      this.#view.setUint32(RING_HEADER_BYTES + position, WRAP_MARKER, true);
      position = 0;
    } else if (free < aligned) {
      this.#drop();
      return { ok: false, reason: "full" };
    }

    const base = RING_HEADER_BYTES + position;
    this.#view.setUint32(base, payload.byteLength, true);
    this.#view.setUint32(base + 4, kind, true);
    this.#view.setBigUint64(base + 8, BigInt.asUintN(64, capability), true);
    new Uint8Array(this.buffer, base + RECORD_HEADER_BYTES, payload.byteLength).set(payload);

    this.#storeHead(head + BigInt(padding + aligned));
    this.#notify();
    return { ok: true, bytes: aligned };
  }

  /** Pop one message, copying the payload out of the ring. */
  tryPop(): RingMessage | null {
    return this.#consume((view) => ({
      kind: view.kind,
      capability: view.capability,
      payload: view.payload.slice(),
    }));
  }

  /**
   * Zero-copy pop: the handler receives a view into the ring and must not
   * retain it. The slot is only released after the handler returns, so the
   * producer cannot overwrite the payload while it is being read.
   */
  tryPopWith(handler: (message: RingMessageView) => void): boolean {
    const consumed = this.#consume((view) => {
      handler(view);
      return true;
    });
    return consumed === true;
  }

  /** Drain up to `max` messages. Returns the number consumed. */
  drain(max = Number.POSITIVE_INFINITY): number {
    let count = 0;
    while (count < max && this.#consume(() => true) === true) count += 1;
    return count;
  }

  /**
   * Block until at least one message is available, or the timeout elapses.
   * Only meaningful for shared buffers; returns immediately otherwise.
   */
  waitForMessage(timeoutMs = 1000): boolean {
    if (this.#loadHead() !== this.#loadTail()) return true;
    if (!this.#shared) return false;
    try {
      Atomics.wait(this.#counters, 0, this.#loadHead(), timeoutMs);
    } catch {
      // Some runtimes disallow blocking the main thread; callers simply poll.
      return this.#loadHead() !== this.#loadTail();
    }
    return this.#loadHead() !== this.#loadTail();
  }

  /**
   * Consume one record. The handler runs before the tail is published, which
   * is what makes both the copy and zero-copy paths race-free.
   */
  #consume<T>(handler: (message: RingMessageView) => T): T | null {
    let tail = this.#loadTail();

    for (let attempt = 0; attempt < 2; attempt += 1) {
      const head = this.#loadHead();
      if (tail === head) return null;

      const position = Number(tail % BigInt(this.capacity));
      const base = RING_HEADER_BYTES + position;
      const length = this.#view.getUint32(base, true);

      if (length === WRAP_MARKER) {
        tail += BigInt(this.capacity - position);
        this.#storeTail(tail);
        continue;
      }
      if (length > this.capacity) {
        throw new Error("corrupt ring: record length exceeds capacity");
      }

      const kind = this.#view.getUint32(base + 4, true);
      const capability = this.#view.getBigUint64(base + 8, true);
      const payload = new Uint8Array(this.buffer, base + RECORD_HEADER_BYTES, length);
      const value = handler({ kind, capability, payload });
      this.#storeTail(tail + BigInt(align8(RECORD_HEADER_BYTES + length)));
      return value;
    }

    return null;
  }

  #loadHead(): bigint {
    return this.#shared ? Atomics.load(this.#counters, 0) : (this.#counters[0] ?? 0n);
  }

  #loadTail(): bigint {
    return this.#shared ? Atomics.load(this.#counters, 1) : (this.#counters[1] ?? 0n);
  }

  #storeHead(value: bigint): void {
    if (this.#shared) Atomics.store(this.#counters, 0, value);
    else this.#counters[0] = value;
  }

  #storeTail(value: bigint): void {
    if (this.#shared) Atomics.store(this.#counters, 1, value);
    else this.#counters[1] = value;
  }

  #notify(): void {
    if (this.#shared) Atomics.notify(this.#counters, 0, 1);
  }

  #drop(): void {
    if (this.#shared) Atomics.add(new Uint32Array(this.buffer, OFFSET_DROPPED, 1), 0, 1);
    else this.#view.setUint32(OFFSET_DROPPED, this.#view.getUint32(OFFSET_DROPPED, true) + 1, true);
  }
}
