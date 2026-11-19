#!/usr/bin/env node
/**
 * Per-surface pixel regression gate.
 *
 * Text and geometry say what was laid out; this says what was actually painted.
 * Compares each source/Aether crop with pixelmatch, writes a diff heatmap, and
 * baselines the ratio so the report cannot drift visually without failing CI.
 *
 *   node scripts/screenshots/pixel-diff.mjs
 *   node scripts/screenshots/pixel-diff.mjs --update   # accept current ratios
 *
 * Outputs
 *   reports/os-app-parity/state/pixel-diff.json
 *   reports/os-app-parity/state/pixel-diff.md
 *   reports/os-app-parity/state/pixel-baseline.json   (--update only)
 *   artifacts/screenshots/pixels/<surface>.png        (heatmaps, ignored)
 */
import { mkdir, readFile, writeFile, access } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PNG } from "pngjs";
import pixelmatch from "pixelmatch";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const outDir = join(root, "reports", "os-app-parity");
const sourceDir = join(outDir, "source");
const aetherDir = join(outDir, "aether");
const stateDir = join(outDir, "state");
const heatDir = join(root, "artifacts", "screenshots", "pixels");
const baselinePath = join(stateDir, "pixel-baseline.json");

/** Aether crop id → source crop id. */
const PAIRS = [
  ["menubar", "menubar"],
  ["dock", "dock"],
  ["menu", "menu"],
  ["command", "command"],
  ["explorer", "explorer"],
  ["settings", "settings"],
  ...[
    "about",
    "network",
    "battery",
    "devices",
    "calendar",
    "taskmanager",
    "shortcuts",
    "audio",
    "chat",
    "agents",
    "remote",
    "terminal",
  ].map((id) => [id, id]),
  ["radio", "music"],
  ["settings-general", "settings-general"],
  ["settings-appearance", "settings-appearance"],
  ["settings-displays", "settings-displays"],
  ["settings-desktop", "settings-desktop"],
  ["settings-commands", "settings-commands"],
  ["settings-accessibility", "settings-accessibility"],
  ["settings-network", "settings-network"],
  ["settings-bluetooth", "settings-bluetooth"],
  ["settings-sound", "settings-sound"],
  ["settings-battery", "settings-battery"],
  ["settings-notifications", "settings-notifications"],
  ["settings-security", "settings-security"],
  ["settings-storage", "settings-storage"],
  ["tray-network", "tray-network"],
  ["tray-devices", "tray-devices"],
  ["tray-battery", "tray-battery"],
  ["tray-sound", "tray-sound"],
  ["tray-calendar", "tray-calendar"],
];

const update = process.argv.includes("--update");

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function readPng(path) {
  return PNG.sync.read(await readFile(path));
}

async function loadBaseline() {
  if (!(await exists(baselinePath))) return {};
  return JSON.parse(await readFile(baselinePath, "utf8"));
}

async function main() {
  await mkdir(stateDir, { recursive: true });
  await mkdir(heatDir, { recursive: true });
  const baseline = await loadBaseline();
  const nextBaseline = {};
  const surfaces = [];
  const regressions = [];

  for (const [aether, source] of PAIRS) {
    const sourcePath = join(sourceDir, `${source}.png`);
    const aetherPath = join(aetherDir, `${aether}.png`);
    if (!(await exists(sourcePath)) || !(await exists(aetherPath))) {
      surfaces.push({ id: aether, status: "missing-crop", ratio: null });
      continue;
    }
    const left = await readPng(sourcePath);
    const right = await readPng(aetherPath);
    if (left.width !== right.width || left.height !== right.height) {
      surfaces.push({
        id: aether,
        status: "size-mismatch",
        source: `${left.width}x${left.height}`,
        aether: `${right.width}x${right.height}`,
        ratio: null,
      });
      continue;
    }
    const heat = new PNG({ width: left.width, height: left.height });
    const different = pixelmatch(left.data, right.data, heat.data, left.width, left.height, {
      threshold: 0.1,
      includeAA: false,
    });
    const ratio = different / (left.width * left.height);
    await writeFile(join(heatDir, `${aether}.png`), PNG.sync.write(heat));

    const previous = baseline[aether] ?? ratio;
    nextBaseline[aether] = Math.round(ratio * 100000) / 100000;
    if (!update && ratio > previous + 0.005) regressions.push(aether);
    surfaces.push({
      id: aether,
      status: ratio === 0 ? "identical" : "different",
      source,
      width: left.width,
      height: left.height,
      pixels: different,
      total: left.width * left.height,
      ratio: Math.round(ratio * 100000) / 100000,
      baseline: Math.round(previous * 100000) / 100000,
    });
  }

  const measured = surfaces.filter((surface) => surface.ratio !== null);
  const totalPixels = measured.reduce((sum, surface) => sum + surface.total, 0);
  const diffPixels = measured.reduce((sum, surface) => sum + surface.pixels, 0);
  const document = {
    summary: {
      surfaces: measured.length,
      identical: measured.filter((surface) => surface.ratio === 0).length,
      skipped: surfaces.length - measured.length,
      diffPixels,
      totalPixels,
      ratio: totalPixels === 0 ? 0 : Math.round((diffPixels / totalPixels) * 100000) / 100000,
      regressions: regressions.length,
    },
    surfaces,
  };
  await writeFile(join(stateDir, "pixel-diff.json"), `${JSON.stringify(document, null, 2)}\n`);

  const lines = [
    "# Pixel diff · source vs Aether crops",
    "",
    `Overall: **${(document.summary.ratio * 100).toFixed(2)} %** of pixels differ across ` +
      `${document.summary.surfaces} same-size surfaces.`,
    "",
    "| Surface | Size | Diff pixels | Ratio | Baseline | Status |",
    "| --- | --- | ---: | ---: | ---: | --- |",
    ...surfaces.map((surface) =>
      surface.ratio === null
        ? `| \`${surface.id}\` | ${surface.source ?? "—"} | — | — | — | ${surface.status} |`
        : `| \`${surface.id}\` | ${surface.width}×${surface.height} | ${surface.pixels} | ${(surface.ratio * 100).toFixed(2)} % | ${(surface.baseline * 100).toFixed(2)} % | ${surface.status} |`,
    ),
    "",
    "Heatmaps are written to `artifacts/screenshots/pixels/`.",
    "",
  ];
  await writeFile(join(stateDir, "pixel-diff.md"), lines.join("\n"));

  if (update) {
    await writeFile(baselinePath, `${JSON.stringify(nextBaseline, null, 2)}\n`);
    console.log(`baseline: ${Object.keys(nextBaseline).length} surfaces`);
  }
  console.log(
    `pixels: ${(document.summary.ratio * 100).toFixed(2)} % differ · ` +
      `${document.summary.skipped} skipped · ${document.summary.regressions} regressions`,
  );
  if (regressions.length > 0) {
    console.error(`pixel regression: ${regressions.join(", ")}`);
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(`error: ${error.stack ?? error.message}`);
  process.exit(1);
});