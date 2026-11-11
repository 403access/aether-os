/**
 * Cross-thread producer used by the integration test. It is deliberately not a
 * test file: `bun test` skips it, the test spawns it via `Worker`.
 */
import { RingBuffer } from "../ring";

interface StartMessage {
  buffer: SharedArrayBuffer;
  count: number;
  kind: number;
}

declare const self: {
  onmessage: ((event: MessageEvent<StartMessage>) => void) | null;
  postMessage(message: unknown): void;
};

const encoder = new TextEncoder();

self.onmessage = (event: MessageEvent<StartMessage>) => {
  const { buffer, count, kind } = event.data;
  const ring = RingBuffer.attach(buffer);

  for (let index = 0; index < count; index += 1) {
    const padding = "x".repeat(index % 97);
    const payload = encoder.encode(`m:${index}:${padding}`);
    let published = false;

    while (!published) {
      const outcome = ring.tryPush(kind, BigInt(index), payload);
      if (outcome.ok) {
        published = true;
        continue;
      }
      if (outcome.reason === "too-large") {
        self.postMessage({ type: "error", message: "payload too large" });
        return;
      }
      // Ring full: yield so the consumer thread can make progress.
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1);
    }
  }

  self.postMessage({ type: "done", count });
};
