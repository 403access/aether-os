import { describe, expect, test } from "bun:test";
import type { IndexDocument } from "@aether/protocol";
import { buildIndex } from "../src/index";

/**
 * Guard rails, not a benchmark. The bench package owns precise numbers; these
 * tests exist so a pathological regression (accidental O(n²), unbounded fuzzy
 * scan) fails CI on a slow runner too.
 */

const NOW = Date.UTC(2026, 10, 12);

function corpus(count: number): IndexDocument[] {
  const kinds = ["file", "app", "command", "content", "setting"] as const;
  const words = [
    "alpha",
    "beta",
    "gamma",
    "delta",
    "settings",
    "indexer",
    "compositor",
    "audio",
    "browser",
    "agent",
    "stream",
    "kernel",
  ];
  return Array.from({ length: count }, (_, i) => {
    const kind = kinds[i % kinds.length] ?? "file";
    const a = words[i % words.length] ?? "alpha";
    const b = words[(i * 7) % words.length] ?? "beta";
    return {
      id: `${kind}:${i}`,
      kind,
      title: `${a} ${b} ${i}`,
      subtitle: `/work/${a}/${b}`,
      body: `alpha beta gamma delta settings indexer compositor audio browser agent stream kernel ${a}${b} `.repeat(
        4,
      ),
      modifiedAt: NOW - (i % 90) * 86_400_000,
      uses: i % 25,
    };
  });
}

describe("index performance guard rails", () => {
  test("indexes 5k documents and answers 100 queries within budget", () => {
    const documents = corpus(5_000);

    const buildStart = performance.now();
    const index = buildIndex(documents);
    const buildMs = performance.now() - buildStart;

    expect(index.size).toBe(5_000);
    expect(buildMs).toBeLessThan(4_000);

    const queries = ["settings", "compositor audio", "brows", "agnt", "stream kernel"];
    const latencies: number[] = [];
    for (let round = 0; round < 20; round += 1) {
      for (const query of queries) {
        const start = performance.now();
        const hits = index.search(query, { now: NOW, limit: 20 });
        latencies.push(performance.now() - start);
        expect(hits.length).toBeGreaterThan(0);
      }
    }

    latencies.sort((a, b) => a - b);
    const p95 = latencies[Math.floor(latencies.length * 0.95)] ?? Number.POSITIVE_INFINITY;
    expect(p95).toBeLessThan(100);
    expect(index.stats().documents).toBe(5_000);
  });
});
