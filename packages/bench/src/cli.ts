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

/**
 * A gate run measures twice and keeps the better sample per metric. This
 * removes transient scheduler noise (a background build, a browser waking up)
 * without hiding a real regression, which affects both runs.
 */
async function gateResult(): Promise<typeof result> {
  if (!check) return result;
  const second = await runBenchmarks();
  const metrics = result.metrics.map((metric) => {
    const other = second.metrics.find((candidate) => candidate.name === metric.name);
    if (other === undefined) return metric;
    const value =
      metric.direction === "lower"
        ? Math.min(metric.value, other.value)
        : Math.max(metric.value, other.value);
    return { ...metric, value };
  });
  return { ...result, metrics };
}

let baseline: Baseline | null = null;
try {
  baseline = JSON.parse(await readFile(BASELINE_PATH, "utf8")) as Baseline;
} catch {
  baseline = null;
}

// A gate run keeps the better of two samples; without a baseline, one run is
// enough and every metric is reported as new.
const measured = baseline === null ? result : await gateResult();

await mkdir(RESULTS_DIR, { recursive: true });
const stamp = measured.createdAt.replace(/[:.]/g, "-");
const resultPath = join(RESULTS_DIR, `${stamp}.json`);
await writeFile(resultPath, `${JSON.stringify(measured, null, 2)}\n`);

const comparison =
  baseline === null
    ? null
    : compareToBaseline(measured, baseline, tolerance ?? baseline.tolerance ?? 0.25);

console.log(`\nAether benchmarks  ·  ${measured.environment.cpu}  ·  bun ${measured.environment.bun}`);
console.log("─".repeat(72));
for (const metric of measured.metrics) {
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
  await writeFile(BASELINE_PATH, `${JSON.stringify(toBaseline(measured), null, 2)}\n`);
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
