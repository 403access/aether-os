#!/usr/bin/env bun
/**
 * `bun run bench`          measure and print, compare if a baseline exists
 * `bun run bench --check`  fail (exit 1) on a regression beyond tolerance
 * `bun run bench --update-baseline`  rewrite bench/baseline.json
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import {
  compareToBaseline,
  formatComparison,
  runBenchmarks,
  toBaseline,
  type Baseline,
} from "./index";

const BASELINE_PATH = join(import.meta.dir, "..", "..", "bench", "baseline.json");
const RESULTS_DIR = join(import.meta.dir, "..", "..", "bench-results");

const args = new Set(process.argv.slice(2));
const check = args.has("--check");
const updateBaseline = args.has("--update-baseline");
const toleranceArg = [...args].find((arg) => arg.startsWith("--tolerance="));
const tolerance =
  toleranceArg === undefined ? undefined : Number(toleranceArg.slice("--tolerance=".length));
if (tolerance !== undefined && (!Number.isFinite(tolerance) || tolerance <= 0)) {
  console.error("--tolerance must be a positive number (for example --tolerance=3)");
  process.exit(2);
}

const result = await runBenchmarks();

await mkdir(RESULTS_DIR, { recursive: true });
const stamp = result.createdAt.replace(/[:.]/g, "-");
const resultPath = join(RESULTS_DIR, `${stamp}.json`);
await writeFile(resultPath, `${JSON.stringify(result, null, 2)}\n`);

let baseline: Baseline | null = null;
try {
  baseline = JSON.parse(await readFile(BASELINE_PATH, "utf8")) as Baseline;
} catch {
  baseline = null;
}

const comparison =
  baseline === null ? null : compareToBaseline(result, baseline, tolerance ?? baseline.tolerance ?? 0.25);

console.log(`\nAether benchmarks  ·  ${result.environment.cpu}  ·  bun ${result.environment.bun}`);
console.log("─".repeat(72));
for (const metric of result.metrics) {
  const reference = baseline?.metrics[metric.name];
  let delta = "";
  if (reference !== undefined && reference.value !== 0) {
    const change = ((metric.value - reference.value) / reference.value) * 100;
    const sign = change > 0 ? "+" : "";
    delta = `  (${sign}${change.toFixed(1)}%)`;
  }
  console.log(
    `${metric.name.padEnd(26)} ${String(metric.value).padStart(12)} ${metric.unit.padEnd(8)}${delta}`,
  );
}
console.log("─".repeat(72));
console.log(`result: ${resultPath}`);

if (updateBaseline) {
  await mkdir(dirname(BASELINE_PATH), { recursive: true });
  await writeFile(BASELINE_PATH, `${JSON.stringify(toBaseline(result), null, 2)}\n`);
  console.log(`baseline updated: ${BASELINE_PATH}`);
}

if (comparison !== null) {
  console.log(formatComparison(comparison));
  if (check && !comparison.ok) process.exit(1);
} else if (check) {
  console.error(
    "no baseline found; run `bun run bench --update-baseline` and commit bench/baseline.json",
  );
  process.exit(1);
}
