#!/usr/bin/env node
/**
 * Water OS design parity: screenshot the reference demo and the compositor at
 * 1280 x 800, then report how many pixels differ.
 *
 *   node scripts/screenshots/os-parity.mjs --reference   # capture the live demo
 *   node scripts/screenshots/os-parity.mjs --compare     # diff shell vs reference
 *
 * Reference states come from https://403access.github.io/data-cloud-compare/
 * design-system/os/ (the canonical Water OS concept). Shell states come from
 * smoke-os-parity-xvfb.sh in the capture container.
 *
 * The report always succeeds; pass --gate to fail on the thresholds. App
 * content (the explorer's two panes) is tracked, not yet aligned, so the gate
 * is opt-in until that surface is rebuilt.
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { PNG } from "pngjs";
import pixelmatch from "pixelmatch";

const ORIGIN =
  process.env.OS_REFERENCE_ORIGIN ??
  "https://403access.github.io/data-cloud-compare/design-system/os/";
const FIXED_UNIX = 1_795_020_420; // 2026-11-18 16:47 UTC, the reference capture.

const root = resolve(dirname(new URL(import.meta.url).pathname), "../..");
const outDir = join(root, "artifacts", "os-parity");
const referenceDir = join(outDir, "reference");
const shellDir = join(root, "artifacts", "screenshots", "shell");
const diffDir = join(outDir, "diff");

/** Regions compared separately so chrome and app content can be judged apart. */
const REGIONS = [
  { name: "menubar", x: 0, y: 0, width: 1280, height: 38 },
  { name: "desktop", x: 0, y: 38, width: 1280, height: 800 - 38 - 70 },
  { name: "dock", x: 0, y: 730, width: 1280, height: 70 },
];

const STATES = [
  { name: "desktop", reference: "desktop.png", shell: "desktop.png" },
  { name: "calendar", reference: "calendar.png", shell: "calendar.png" },
];

async function captureReference() {
  const { chromium } = await import("@playwright/test");
  await mkdir(referenceDir, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    timezoneId: "UTC",
    locale: "en-US",
  });
  // Pin the demo clock before any page script runs so the menu bar renders the
  // comparison instant in UTC, matching AETHER_FIXED_UNIX in the compositor.
  await page.addInitScript((fixed) => {
    const RealDate = Date;
    const offset = fixed * 1000 - RealDate.now();
    class FixedDate extends RealDate {
      constructor(...args) {
        if (args.length === 0) super(RealDate.now() + offset);
        else super(...args);
      }
      static now() {
        return RealDate.now() + offset;
      }
    }
    window.Date = FixedDate;
  }, FIXED_UNIX);
  await page.goto(ORIGIN, { waitUntil: "networkidle" });
  await page.waitForTimeout(1200);
  await page.screenshot({ path: join(referenceDir, "desktop.png") });

  await page.evaluate(() => {
    document.querySelectorAll(".os-window").forEach((el) => {
      el.hidden = true;
      el.classList.remove("is-open", "is-focused");
    });
    document.querySelector('[data-cmd="open-calendar"]').click();
  });
  await page.waitForTimeout(900);
  await page.screenshot({ path: join(referenceDir, "calendar.png") });
  await browser.close();
  console.log(`reference: ${referenceDir}`);
}

function crop(png, region) {
  const out = new PNG({ width: region.width, height: region.height });
  PNG.bitblt(png, out, region.x, region.y, region.width, region.height, 0, 0);
  return out;
}

async function compare() {
  await mkdir(diffDir, { recursive: true });
  const report = { origin: ORIGIN, states: [] };
  let failed = false;

  for (const state of STATES) {
    const expected = PNG.sync.read(await readFile(join(referenceDir, state.reference)));
    const actual = PNG.sync.read(await readFile(join(shellDir, state.shell)));
    if (expected.width !== actual.width || expected.height !== actual.height) {
      throw new Error(
        `${state.name}: size mismatch ${expected.width}x${expected.height} vs ${actual.width}x${actual.height}`,
      );
    }
    const diff = new PNG({ width: expected.width, height: expected.height });
    const mismatched = pixelmatch(
      expected.data,
      actual.data,
      diff.data,
      expected.width,
      expected.height,
      { threshold: 0.15, includeAA: false },
    );
    await writeFile(join(diffDir, `${state.name}.png`), PNG.sync.write(diff));

    const total = expected.width * expected.height;
    const regions = [];
    for (const region of REGIONS) {
      const a = crop(expected, region);
      const b = crop(actual, region);
      const scratch = new PNG({ width: region.width, height: region.height });
      const count = pixelmatch(a.data, b.data, scratch.data, region.width, region.height, {
        threshold: 0.15,
        includeAA: false,
      });
      const area = region.width * region.height;
      regions.push({
        name: region.name,
        mismatched: count,
        percent: Number(((count / area) * 100).toFixed(2)),
      });
    }
    const entry = {
      name: state.name,
      mismatched,
      percent: Number(((mismatched / total) * 100).toFixed(2)),
      regions,
    };
    report.states.push(entry);
    console.log(
      `${state.name}: ${entry.percent}% differ ` +
        regions.map((region) => `${region.name} ${region.percent}%`).join(" · "),
    );
  }

  const gate = args.includes("--gate");
  for (const state of report.states) {
    const menubar = state.regions.find((region) => region.name === "menubar");
    const dock = state.regions.find((region) => region.name === "dock");
    if (!gate) continue;
    if (menubar.percent > 12 || dock.percent > 3) failed = true;
  }

  await writeFile(join(outDir, "report.json"), JSON.stringify(report, null, 2));
  const lines = [
    "# Water OS design parity",
    "",
    `Reference: ${ORIGIN}`,
    "",
    "| State | Total | Menu bar | Desktop | Dock |",
    "| --- | --- | --- | --- | --- |",
    ...report.states.map((state) => {
      const by = Object.fromEntries(state.regions.map((region) => [region.name, region.percent]));
      return `| ${state.name} | ${state.percent}% | ${by.menubar}% | ${by.desktop}% | ${by.dock}% |`;
    }),
    "",
    "Diffs: `artifacts/os-parity/diff/`.",
    "",
  ];
  await writeFile(join(outDir, "report.md"), lines.join("\n"));
  console.log(`report: ${join(outDir, "report.md")}`);
  if (failed) {
    console.error("parity gate: menu bar or dock exceeds its mismatch budget");
    process.exitCode = 1;
  }
}

const args = process.argv.slice(2);

if (args.includes("--reference")) await captureReference();
if (args.includes("--compare") || args.length === 0) await compare();