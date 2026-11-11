import { cpus, platform, arch } from "node:os";
import { buildIndex } from "@aether/indexer";
import { CapabilityTable, RingBuffer } from "@aether/ipc";
import { PaletteEngine, createStaticProvider, type PaletteItem } from "@aether/palette";
import type { IndexDocument } from "@aether/protocol";

/**
 * Benchmarks with a stable, small, deterministic shape so they can run in CI.
 *
 * Each metric declares a direction: "lower" is better for latency, "higher" is
 * better for throughput. `compareToBaseline` applies a tolerance so a noisy
 * runner does not fail on a 5% wobble.
 */

export type Direction = "lower" | "higher";

export interface Metric {
  name: string;
  value: number;
  unit: string;
  direction: Direction;
}

export interface BenchResult {
  createdAt: string;
  environment: {
    platform: string;
    arch: string;
    bun: string;
    cpu: string;
  };
  metrics: Metric[];
}

export interface Baseline {
  version: 1;
  createdAt: string;
  tolerance?: number;
  metrics: Record<string, { value: number; unit: string; direction: Direction }>;
}

export interface Regression {
  name: string;
  baseline: number;
  current: number;
  changePercent: number;
}

export interface Comparison {
  ok: boolean;
  regressions: Regression[];
  missing: string[];
  extra: string[];
}

export interface BenchOptions {
  documents?: number;
  queryIterations?: number;
  ringIterations?: number;
  capabilityIterations?: number;
  paletteIterations?: number;
  paletteItems?: number;
  warmup?: number;
}

const WORDS = [
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
  "surface",
  "frame",
  "capability",
  "window",
];

const KINDS = ["file", "app", "command", "content", "setting"] as const;

function corpus(count: number): IndexDocument[] {
  return Array.from({ length: count }, (_, index) => {
    const kind = KINDS[index % KINDS.length] ?? "file";
    const first = WORDS[index % WORDS.length] ?? "alpha";
    const second = WORDS[(index * 5) % WORDS.length] ?? "beta";
    return {
      id: `${kind}:${index}`,
      kind,
      title: `${first} ${second} ${index}`,
      subtitle: `/work/${first}/${second}`,
      body: `alpha beta gamma delta settings indexer compositor audio browser agent stream kernel ${first}${second} `.repeat(
        3,
      ),
      modifiedAt: Date.UTC(2026, 10, 1) - (index % 90) * 86_400_000,
      uses: index % 20,
    };
  });
}

export function percentile(samples: readonly number[], p: number): number {
  if (samples.length === 0) return 0;
  const sorted = [...samples].sort((a, b) => a - b);
  const clamped = Math.max(0, Math.min(100, p));
  const index = Math.floor((clamped / 100) * (sorted.length - 1));
  return sorted[index] ?? 0;
}

export async function runBenchmarks(options: BenchOptions = {}): Promise<BenchResult> {
  const documents = options.documents ?? 5_000;
  const queryIterations = options.queryIterations ?? 200;
  const ringIterations = options.ringIterations ?? 200_000;
  const capabilityIterations = options.capabilityIterations ?? 100_000;
  const paletteIterations = options.paletteIterations ?? 60;
  const paletteItems = options.paletteItems ?? 1_000;
  const warmup = options.warmup ?? 20;

  const metrics: Metric[] = [];

  // --- index build ---------------------------------------------------------
  const corpusDocuments = corpus(documents);
  const buildStart = performance.now();
  const index = buildIndex(corpusDocuments);
  const buildMs = performance.now() - buildStart;
  metrics.push({ name: "index.build", value: round(buildMs), unit: "ms", direction: "lower" });

  // --- index query ---------------------------------------------------------
  const queries = ["settings", "compositor audio", "brows", "agnt", "stream kernel"];
  for (let index_ = 0; index_ < warmup; index_ += 1) {
    index.search(queries[index_ % queries.length] ?? "settings", { now: Date.UTC(2026, 10, 1) });
  }
  const latencies: number[] = [];
  for (let index_ = 0; index_ < queryIterations; index_ += 1) {
    const query = queries[index_ % queries.length] ?? "settings";
    const start = performance.now();
    index.search(query, { now: Date.UTC(2026, 10, 1) });
    latencies.push((performance.now() - start) * 1_000);
  }
  metrics.push({
    name: "index.query.p50",
    value: round(percentile(latencies, 50), 2),
    unit: "µs",
    direction: "lower",
  });
  metrics.push({
    name: "index.query.p95",
    value: round(percentile(latencies, 95), 2),
    unit: "µs",
    direction: "lower",
  });

  // --- ring throughput -----------------------------------------------------
  const ring = RingBuffer.create(64 * 1024, { shared: false });
  const payload = new Uint8Array(64);
  const ringStart = performance.now();
  let processed = 0;
  for (let index_ = 0; index_ < ringIterations; index_ += 1) {
    if (!ring.tryPush(1, 0n, payload).ok) {
      while (ring.tryPop() !== null) processed += 1;
      ring.tryPush(1, 0n, payload);
    }
    if (ring.tryPop() !== null) processed += 1;
  }
  while (ring.tryPop() !== null) processed += 1;
  const ringSeconds = (performance.now() - ringStart) / 1_000;
  metrics.push({
    name: "ipc.ring.throughput",
    value: round(processed / ringSeconds, 0),
    unit: "msgs/s",
    direction: "higher",
  });

  // --- capability verification --------------------------------------------
  const table = new CapabilityTable();
  const capabilities = Array.from({ length: 128 }, (_, index_) =>
    table.issue({
      subject: `agent-${index_}`,
      scope: "fs.read",
      resource: `fs:/work/${index_}`,
    }),
  );
  const capabilityStart = performance.now();
  let verified = 0;
  for (let index_ = 0; index_ < capabilityIterations; index_ += 1) {
    const capability = capabilities[index_ % capabilities.length];
    if (capability === undefined) continue;
    const result = table.verify(capability.id, {
      subject: `agent-${index_ % capabilities.length}`,
      scope: "fs.read",
      resource: `fs:/work/${index_ % capabilities.length}/src`,
    });
    if (result.ok) verified += 1;
  }
  const capabilitySeconds = (performance.now() - capabilityStart) / 1_000;
  metrics.push({
    name: "capability.verify",
    value: round(verified / capabilitySeconds, 0),
    unit: "ops/s",
    direction: "higher",
  });

  // --- palette query -------------------------------------------------------
  const paletteDocs: PaletteItem[] = Array.from({ length: paletteItems }, (_, index_) => ({
    id: `item-${index_}`,
    title: `${WORDS[index_ % WORDS.length] ?? "item"} ${index_}`,
    category: "All",
    keywords: WORDS[(index_ * 3) % WORDS.length] ?? "",
  }));
  const engine = new PaletteEngine({
    providers: [createStaticProvider("all", paletteDocs)],
    limit: 12,
  });
  for (let index_ = 0; index_ < 5; index_ += 1) await engine.query("set");
  const paletteLatencies: number[] = [];
  for (let index_ = 0; index_ < paletteIterations; index_ += 1) {
    const start = performance.now();
    await engine.query(index_ % 2 === 0 ? "set" : "comp");
    paletteLatencies.push((performance.now() - start) * 1_000);
  }
  metrics.push({
    name: "palette.query.p50",
    value: round(percentile(paletteLatencies, 50), 2),
    unit: "µs",
    direction: "lower",
  });

  return {
    createdAt: new Date().toISOString(),
    environment: {
      platform: platform(),
      arch: arch(),
      bun: Bun.version,
      cpu: cpus()[0]?.model ?? "unknown",
    },
    metrics,
  };
}

export function toBaseline(result: BenchResult, tolerance = 0.25): Baseline {
  const metrics: Baseline["metrics"] = {};
  for (const metric of result.metrics) {
    metrics[metric.name] = {
      value: metric.value,
      unit: metric.unit,
      direction: metric.direction,
    };
  }
  return { version: 1, createdAt: result.createdAt, tolerance, metrics };
}

export function compareToBaseline(
  result: BenchResult,
  baseline: Baseline,
  tolerance = baseline.tolerance ?? 0.25,
): Comparison {
  const regressions: Regression[] = [];
  const missing: string[] = [];
  const names = new Set<string>();

  for (const metric of result.metrics) {
    names.add(metric.name);
    const reference = baseline.metrics[metric.name];
    if (reference === undefined) {
      missing.push(metric.name);
      continue;
    }

    const direction = reference.direction;
    const changePercent =
      reference.value === 0 ? 0 : ((metric.value - reference.value) / reference.value) * 100;
    const regressed =
      direction === "lower"
        ? changePercent > tolerance * 100
        : changePercent < -tolerance * 100;

    if (regressed) {
      regressions.push({
        name: metric.name,
        baseline: reference.value,
        current: metric.value,
        changePercent: round(changePercent, 2),
      });
    }
  }

  const extra = Object.keys(baseline.metrics).filter((name) => !names.has(name));
  return { ok: regressions.length === 0, regressions, missing, extra };
}

export function formatComparison(comparison: Comparison): string {
  const lines: string[] = [];
  if (comparison.ok) {
    lines.push("performance gate: PASS");
  } else {
    lines.push("performance gate: FAIL");
    for (const regression of comparison.regressions) {
      const sign = regression.changePercent > 0 ? "+" : "";
      lines.push(
        `  ${regression.name}: ${regression.baseline} -> ${regression.current} (${sign}${regression.changePercent}%)`,
      );
    }
  }
  if (comparison.missing.length > 0) {
    lines.push(`  new metrics without baseline: ${comparison.missing.join(", ")}`);
  }
  if (comparison.extra.length > 0) {
    lines.push(`  baseline metrics no longer produced: ${comparison.extra.join(", ")}`);
  }
  return lines.join("\n");
}

function round(value: number, decimals = 1): number {
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
}
