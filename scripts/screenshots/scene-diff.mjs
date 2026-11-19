#!/usr/bin/env node
/**
 * Scene differ: compare the source DOM tree with the Aether render chunk.
 *
 * Pixels only tell you *that* something differs; the state manifest only knows
 * the values someone enumerated. This pass compares what each side actually
 * laid out for a surface — every visible text run and its box — so a missing
 * label, row description or sidebar group is reported as data.
 *
 *   node scripts/screenshots/scene-diff.mjs
 *   node scripts/screenshots/scene-diff.mjs --update   # accept current coverage
 *
 * Inputs
 *   artifacts/screenshots/shell/scenes/source-<id>.json  (app-parity.mjs)
 *   artifacts/screenshots/shell/scenes/aether-<id>.json  (smoke scene dumps)
 *
 * Outputs
 *   reports/os-app-parity/state/scene-diff.json
 *   reports/os-app-parity/state/scene-diff.md
 *   reports/os-app-parity/state/scene-baseline.json     (--update only)
 */
import { mkdir, readFile, writeFile, access } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const shellDir = join(root, "artifacts", "screenshots", "shell");
const sceneDir = join(shellDir, "scenes");
const outDir = join(root, "reports", "os-app-parity", "state");
const baselinePath = join(outDir, "scene-baseline.json");

const WINDOW = [16, 54, 1248, 730];
const TRAY = {
  "tray-network": [620, 42, 300, 328],
  "tray-devices": [656, 42, 300, 253],
  "tray-battery": [736, 42, 286, 270],
  "tray-sound": [766, 42, 292, 256],
  "tray-calendar": [900, 42, 308, 351],
};

/**
 * Surface id (the report's Aether crop key) → source scene id, Aether scene id
 * and the rectangle both sides are clipped to.
 */
const SURFACES = [
  { id: "about", source: "about", aether: "app-about", rect: WINDOW },
  { id: "explorer", source: "explorer", aether: "app-explorer", rect: WINDOW },
  { id: "network", source: "network", aether: "app-network", rect: WINDOW },
  { id: "battery", source: "battery", aether: "app-battery", rect: WINDOW },
  { id: "devices", source: "devices", aether: "app-devices", rect: WINDOW },
  { id: "radio", source: "music", aether: "app-radio", rect: WINDOW },
  { id: "calendar", source: "calendar", aether: "calendar", rect: WINDOW },
  { id: "taskmanager", source: "taskmanager", aether: "app-task-manager", rect: WINDOW },
  ...["general", "appearance", "displays", "desktop", "commands", "accessibility", "network", "bluetooth", "sound", "battery", "notifications", "security", "storage"].map(
    (pane) => ({
      id: `settings-${pane}`,
      source: `settings-${pane}`,
      aether: `app-settings-${pane}`,
      rect: WINDOW,
    }),
  ),
  { id: "menubar", source: "menubar", aether: "desktop", rect: [0, 0, 1280, 38] },
  { id: "menu", source: "menu", aether: "menu", rect: [110, 0, 250, 230] },
  ...Object.entries(TRAY).map(([id, rect]) => ({
    id,
    source: id,
    aether: id,
    rect,
  })),
];

const args = new Set(process.argv.slice(2));
const update = args.has("--update");

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function readJson(path) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch {
    return null;
  }
}

const inside = (x, y, [rx, ry, rw, rh]) =>
  x + 1 >= rx && x <= rx + rw && y + 1 >= ry && y <= ry + rh;

const normalize = (text) =>
  text
    .normalize("NFKC")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();

/**
 * Data readings (CPU %, IP addresses, dates, counts) differ between machines
 * by design and would drown the structural comparison. Text that contains a
 * digit is still captured in the scene, but it is not part of the coverage
 * denominator.
 */
function isDynamic(text) {
  return /\d/.test(text);
}

function sourceTexts(scene, rect) {
  const texts = [];
  let dynamic = 0;
  for (const element of scene.elements ?? []) {
    if (!element.text || element.text.length < 2) continue;
    const cx = element.x + element.w / 2;
    const cy = element.y + element.h / 2;
    if (!inside(cx, cy, rect)) continue;
    if (isDynamic(element.text)) {
      dynamic += 1;
      continue;
    }
    texts.push(element.text);
  }
  return { texts, dynamic };
}

function aetherTexts(scene, rect) {
  const texts = [];
  for (const label of scene.labels ?? []) {
    if (!label.text || label.text.length < 2) continue;
    if (isDynamic(label.text)) continue;
    // Labels are anchored at their top-left; text height is roughly 1.6em.
    const cx = label.x + 4;
    const cy = label.y + Math.max(4, (label.px ?? 10) * 0.8);
    if (!inside(cx, cy, rect)) continue;
    texts.push(label.text);
  }
  return texts;
}

function compareTexts(source, aether) {
  const pool = aether.map((text) => normalize(text));
  const used = new Set();
  const missing = [];
  const matched = [];

  for (const text of source) {
    const needle = normalize(text);
    let index = pool.findIndex((value, at) => !used.has(at) && value === needle);
    if (index === -1) {
      // Runs split across elements: accept a word-boundary prefix/suffix match
      // when the two strings are of comparable length.
      index = pool.findIndex((value, at) => {
        if (used.has(at)) return false;
        const shorter = Math.min(value.length, needle.length);
        const longer = Math.max(value.length, needle.length);
        if (longer === 0 || shorter / longer < 0.6) return false;
        const pattern = new RegExp(`(^|\\s)${needle.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}(\\s|$)`);
        return pattern.test(value);
      });
    }
    if (index === -1) missing.push(text);
    else {
      used.add(index);
      matched.push(text);
    }
  }

  const extra = aether.filter((_, index) => !used.has(index));
  return { matched, missing, extra };
}

async function loadBaseline() {
  if (!(await exists(baselinePath))) return {};
  return (await readJson(baselinePath)) ?? {};
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const baseline = await loadBaseline();
  const nextBaseline = {};
  const surfaces = [];
  const regressions = [];

  for (const surface of SURFACES) {
    const source = await readJson(join(sceneDir, `source-${surface.source}.json`));
    const aether = await readJson(join(sceneDir, `aether-${surface.aether}.json`));
    if (!source || !aether) {
      surfaces.push({
        id: surface.id,
        status: !source ? "no-source-scene" : "no-aether-scene",
        source: 0,
        aether: 0,
        coverage: null,
      });
      continue;
    }

    const { texts: sourceList, dynamic } = sourceTexts(source, surface.rect);
    const aetherList = aetherTexts(aether, surface.rect);
    const { matched, missing, extra } = compareTexts(sourceList, aetherList);
    const coverage = sourceList.length === 0 ? 1 : matched.length / sourceList.length;
    const accepted = baseline[surface.id] ?? coverage;
    nextBaseline[surface.id] = Math.round(coverage * 1000) / 1000;
    if (!update && coverage + 0.02 < accepted) regressions.push(surface.id);

    surfaces.push({
      id: surface.id,
      status: coverage === 1 ? "complete" : "missing-text",
      source: sourceList.length,
      dynamic,
      aether: aetherList.length,
      matched: matched.length,
      coverage: Math.round(coverage * 1000) / 1000,
      baseline: Math.round(accepted * 1000) / 1000,
      missing: missing.slice(0, 60),
      extra: extra.slice(0, 60),
    });
  }

  const withScenes = surfaces.filter((surface) => surface.coverage !== null);
  const totalSource = withScenes.reduce((sum, surface) => sum + surface.source, 0);
  const totalMatched = withScenes.reduce((sum, surface) => sum + (surface.matched ?? 0), 0);
  const document = {
    summary: {
      surfaces: withScenes.length,
      complete: withScenes.filter((surface) => surface.coverage === 1).length,
      textRuns: totalSource,
      matched: totalMatched,
      coverage: totalSource === 0 ? 1 : Math.round((totalMatched / totalSource) * 1000) / 1000,
      missingScenes: surfaces.filter((surface) => surface.coverage === null).length,
      regressions: regressions.length,
    },
    surfaces,
  };

  await writeFile(join(outDir, "scene-diff.json"), `${JSON.stringify(document, null, 2)}\n`);

  const lines = [
    "# Scene diff · source DOM vs Aether render chunk",
    "",
    `Text coverage: **${document.summary.matched}/${document.summary.textRuns}** runs ` +
      `across ${document.summary.surfaces} surfaces ` +
      `(${(document.summary.coverage * 100).toFixed(1)} %).`,
    "",
    "| Surface | Source runs | Aether runs | Matched | Coverage | Missing (first) |",
    "| --- | ---: | ---: | ---: | ---: | --- |",
    ...surfaces.map((surface) =>
      surface.coverage === null
        ? `| \`${surface.id}\` | — | — | — | — | ${surface.status} |`
        : `| \`${surface.id}\` | ${surface.source} | ${surface.aether} | ${surface.matched} | ${(surface.coverage * 100).toFixed(0)} % | ${surface.missing
            .slice(0, 5)
            .map((text) => `\`${text.replace(/\|/g, "\\|")}\``)
            .join(", ")} |`,
    ),
    "",
    "Missing text is the actionable half: every entry is a source label, row",
    "description or group heading that the Aether frame does not paint.",
    "",
  ];
  await writeFile(join(outDir, "scene-diff.md"), lines.join("\n"));

  if (update) {
    await writeFile(baselinePath, `${JSON.stringify(nextBaseline, null, 2)}\n`);
    console.log(`baseline: ${Object.keys(nextBaseline).length} surfaces`);
  }

  console.log(
    `scene: ${document.summary.matched}/${document.summary.textRuns} text runs ` +
      `(${(document.summary.coverage * 100).toFixed(1)} %) · ${document.summary.complete}/` +
      `${document.summary.surfaces} complete · ${document.summary.regressions} regressions`,
  );
  if (regressions.length > 0) {
    console.error(`scene regression: ${regressions.join(", ")}`);
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(`error: ${error.stack ?? error.message}`);
  process.exit(1);
});