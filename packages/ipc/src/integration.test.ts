import { afterEach, describe, expect, test } from "bun:test";
import { MESSAGE } from "@aether/protocol";
import { RingBuffer } from "../src/index";

const decoder = new TextDecoder();
const workers: Worker[] = [];

afterEach(() => {
  for (const worker of workers.splice(0)) worker.terminate();
});

/**
 * True cross-thread test: the producer runs in a Worker over a
 * SharedArrayBuffer while the consumer drains on the main thread, using the
 * same Atomics-based protocol the daemon uses over memfd.
 */
describe("SPSC ring across threads", () => {
  test(
    "delivers 5000 ordered messages without loss",
    () => {
      const count = 5_000;
      const producerRing = RingBuffer.create(8 * 1024);
      const consumerRing = RingBuffer.attach(producerRing.buffer);

      const worker = new Worker(new URL("./fixtures/producer.worker.ts", import.meta.url).href);
      workers.push(worker);
      worker.postMessage({
        buffer: producerRing.buffer,
        count,
        kind: MESSAGE.chatToken,
      });

      let received = 0;
      let lastCapability = -1n;
      const deadline = Date.now() + 20_000;

      while (received < count) {
        if (Date.now() > deadline) {
          throw new Error(`timed out after receiving ${received}/${count} messages`);
        }
        // Block on the shared counter until the producer publishes.
        consumerRing.waitForMessage(250);
        let message = consumerRing.tryPop();
        while (message !== null) {
          const text = decoder.decode(message.payload);
          expect(text.startsWith(`m:${received}:`)).toBe(true);
          expect(message.kind).toBe(MESSAGE.chatToken);
          expect(message.capability).toBe(BigInt(received));
          expect(message.capability > lastCapability).toBe(true);
          lastCapability = message.capability;
          received += 1;
          message = consumerRing.tryPop();
        }
      }

      expect(received).toBe(count);
      expect(consumerRing.stats().usedBytes).toBe(0);
      // `dropped` counts backpressure rejections (the producer retries), not
      // lost messages: every message produced above was delivered in order.
      expect(producerRing.stats().dropped).toBe(consumerRing.stats().dropped);
    },
    30_000,
  );
});
