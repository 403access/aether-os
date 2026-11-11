import { describe, expect, test } from "bun:test";
import {
  compareToBaseline,
  percentile,
  runBenchmarks,
  toBaseline,
  type Baseline,
  type BenchResult,
} from "../src/index";

function result(metrics: BenchResult["metrics"]): BenchResult {
  return {
    createdAt: "2026-11-12T00:00:00.000Z",
    environment: { platform: "test", arch: "test", bun: "test", cpu: "test" },
    metrics,
  };
}

const baseline: Baseline = {
  version: 1,
  createdAt: "2026-11-01T00:00:00.000Z",
  tolerance: 0.25,
  metrics: {
    "index.query.p95": { value: 100, unit: "µs", direction: "lower" },
    "ipc.ring.throughput": { value: 1_000_000, unit: "msgs/s", direction: "higher" },
  },
};

describe("percentile", () => {
  test("computes p50 and p95 deterministically", () => {
    const samples = Array.from({ length: 100 }, (_, index) => index + 1);
    expect(percentile(samples, 50)).toBe(50);
    expect(percentile(samples, 95)).toBe(95);
    expect(percentile(samples, 100)).toBe(100);
    expect(percentile([], 50)).toBe(0);
  });
});

describe("compareToBaseline", () => {
  test("passes within tolerance", () => {
    const comparison = compareToBaseline(
      result([
        { name: "index.query.p95", value: 120, unit: "µs", direction: "lower" },
        { name: "ipc.ring.throughput", value: 800_000, unit: "msgs/s", direction: "higher" },
      ]),
      baseline,
    );
    expect(comparison.ok).toBe(true);
    expect(comparison.regressions).toEqual([]);
  });

  test("fails on a latency regression beyond tolerance", () => {
    const comparison = compareToBaseline(
      result([
        { name: "index.query.p95", value: 150, unit: "µs", direction: "lower" },
        { name: "ipc.ring.throughput", value: 1_000_000, unit: "msgs/s", direction: "higher" },
      ]),
      baseline,
    );
    expect(comparison.ok).toBe(false);
    expect(comparison.regressions[0]?.name).toBe("index.query.p95");
    expect(comparison.regressions[0]?.changePercent).toBe(50);
  });

  test("fails on a throughput regression", () => {
    const comparison = compareToBaseline(
      result([
        { name: "index.query.p95", value: 100, unit: "µs", direction: "lower" },
        { name: "ipc.ring.throughput", value: 500_000, unit: "msgs/s", direction: "higher" },
      ]),
      baseline,
    );
    expect(comparison.ok).toBe(false);
    expect(comparison.regressions[0]?.name).toBe("ipc.ring.throughput");
  });

  test("reports new and removed metrics", () => {
    const comparison = compareToBaseline(
      result([
        { name: "index.query.p95", value: 100, unit: "µs", direction: "lower" },
        { name: "brand.new", value: 1, unit: "x", direction: "lower" },
      ]),
      baseline,
    );
    expect(comparison.missing).toEqual(["brand.new"]);
    expect(comparison.extra).toEqual(["ipc.ring.throughput"]);
  });

  test("toBaseline round-trips metrics", () => {
    const measured = result([
      { name: "a", value: 12.5, unit: "ms", direction: "lower" },
    ]);
    const snapshot = toBaseline(measured);
    expect(snapshot.metrics["a"]).toEqual({ value: 12.5, unit: "ms", direction: "lower" });
  });
});

describe("runBenchmarks", () => {
  test("produces the full metric set on a small corpus", async () => {
    const measured = await runBenchmarks({
      documents: 200,
      queryIterations: 10,
      ringIterations: 2_000,
      capabilityIterations: 1_000,
      paletteIterations: 4,
      paletteItems: 50,
      warmup: 2,
    });

    const names = measured.metrics.map((metric) => metric.name);
    expect(names).toEqual([
      "index.build",
      "index.query.p50",
      "index.query.p95",
      "ipc.ring.throughput",
      "capability.verify",
      "palette.query.p50",
    ]);
    for (const metric of measured.metrics) {
      expect(metric.value).toBeGreaterThanOrEqual(0);
      expect(metric.unit.length).toBeGreaterThan(0);
    }
    expect(measured.environment.bun.length).toBeGreaterThan(0);
  }, 20_000);
});
