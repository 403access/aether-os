#!/usr/bin/env node
/**
 * Design-state parity: extract the same design values from both projects and
 * diff them property by property.
 *
 *   node scripts/screenshots/design-state.mjs            # extract + diff
 *   node scripts/screenshots/design-state.mjs --diff-only # reuse the JSON
 *
 * Source of truth
 *   Water OS demo  ../jottacloud-comparison/design-system/os
 *                  read with Chromium's computed styles, so cascade, custom
 *                  properties, `color-mix()` and the 1.14 OS font scale are
 *                  already applied.
 *   Aether         `aether-shell-wayland --bin design-manifest`, which prints
 *                  the values the renderer paints with (tokens, type scales,
 *                  menu-bar/dock geometry, chrome effects).
 *
 * Outputs (committed)
 *   reports/os-app-parity/state/source.json
 *   reports/os-app-parity/state/aether.json
 *   reports/os-app-parity/state/diff.json
 *   reports/os-app-parity/state/diff.md
 *
 * Accepted differences live in state/baseline.json as { key: reason }. The run
 * exits non-zero when a difference is not in the baseline, so drift fails CI.
 */
import { execFileSync } from "node:child_process";
import { createReadStream } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile, access } from "node:fs/promises";
import { dirname, extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const sourceRepo = process.env.OS_SOURCE_REPO ?? resolve(root, "..", "jottacloud-comparison");
const sourceRoot = join(sourceRepo, "design-system");
const outDir = join(root, "reports", "os-app-parity", "state");
const baselinePath = join(outDir, "baseline.json");
const container = process.env.AETHER_CONTAINER ?? "aether-dev2";
const manifestBin =
  process.env.AETHER_MANIFEST_BIN ??
  "/src/os/compositor/aether-shell-wayland/target/debug/design-manifest";

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
};

/** Trays, in the order the demo's menu bar lists them. */
const TRAYS = ["network", "devices", "battery", "sound", "calendar"];

const diffOnly = process.argv.includes("--diff-only");

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

function gitRevision(repo) {
  try {
    return execFileSync("git", ["-C", repo, "log", "-1", "--format=%h %s"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return "unknown";
  }
}

async function serveSource() {
  if (!(await exists(join(sourceRoot, "os", "index.html")))) {
    throw new Error(`no Water OS demo at ${join(sourceRoot, "os", "index.html")}`);
  }
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    let path = normalize(join(sourceRoot, decodeURIComponent(url.pathname)));
    if (!path.startsWith(sourceRoot)) {
      response.writeHead(403).end();
      return;
    }
    if (url.pathname.endsWith("/")) path = join(path, "index.html");
    createReadStream(path)
      .on("error", () => response.writeHead(404).end("not found"))
      .on("open", () => {
        response.writeHead(200, {
          "content-type": MIME[extname(path)] ?? "application/octet-stream",
        });
      })
      .pipe(response);
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const { port } = server.address();
  return {
    origin: `http://127.0.0.1:${port}/os/`,
    close: () => server.close(),
  };
}

/* ------------------------------------------------------------ source side */

/**
 * Runs in the page. Everything here reads computed styles, so the result is
 * what Chromium would paint, not what the stylesheet says.
 */
function collectSource(trays) {
  const num = (value) => {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? Math.round(parsed * 100) / 100 : null;
  };

  /** Normalise rgb(), rgba(), color(srgb ...) and hex to #rrggbbaa. */
  const color = (value) => {
    const text = String(value ?? "").trim();
    if (text === "" || text === "none" || text === "transparent") return null;
    const hex = text.match(/^#([0-9a-f]{6})([0-9a-f]{2})?$/i);
    if (hex) return `#${hex[1].toLowerCase()}${(hex[2] ?? "ff").toLowerCase()}`;
    const srgb = text.match(
      /color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)/,
    );
    const rgba = text.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)/);
    const parts = srgb
      ? [srgb[1], srgb[2], srgb[3], srgb[4]].map((part, index) =>
          index < 3 ? Number(part) * 255 : part === undefined ? 1 : Number(part),
        )
      : rgba
        ? [rgba[1], rgba[2], rgba[3], rgba[4] === undefined ? 1 : rgba[4]].map(Number)
        : null;
    if (!parts) return null;
    return `#${parts
      .map((part, index) =>
        Math.round(Math.max(0, Math.min(255, index < 3 ? part : part * 255)))
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")}`;
  };

  const weight = (value) => {
    const numeric = Number(value);
    if (!Number.isFinite(numeric)) return "regular";
    return numeric >= 650 ? "bold" : numeric >= 550 ? "medium" : "regular";
  };

  const element = (selector) => document.querySelector(selector);
  const style = (selector) => {
    const node = element(selector);
    return node ? getComputedStyle(node) : null;
  };
  const box = (selector) => {
    const node = element(selector);
    if (!node) return null;
    const rect = node.getBoundingClientRect();
    return { width: num(rect.width), height: num(rect.height), x: num(rect.x), y: num(rect.y) };
  };
  const padding = (selector) => {
    const cs = style(selector);
    return cs ? num(cs.paddingLeft) : null;
  };

  const values = {};
  const add = (key, value) => {
    if (value !== null && value !== undefined) values[key] = value;
  };

  const rootStyle = getComputedStyle(document.documentElement);
  const bodyStyle = getComputedStyle(document.body);

  // Tokens. Custom properties are raw declarations, so parse the same way.
  const token = (name, key = name) =>
    add(`token.${key}`, color(bodyStyle.getPropertyValue(`--ds-${name}`)));

  token("bg");
  token("text");
  token("text-2");
  token("text-3");
  token("accent");
  token("line");
  token("line-strong");
  token("green");
  token("popover");
  // Window fill is `color-mix(surface-solid 92%, transparent)`.
  add("token.surface", color(style(".os-window")?.backgroundColor));
  add("token.menubar-fill", color(style(".os-menubar")?.backgroundColor));

  // Type ramp. The OS scales the design tokens by --os-font-scale.
  const type = (scale, selector) => {
    const cs = style(selector);
    if (cs) add(`type.${scale}.px`, num(cs.fontSize));
  };
  type(7, ".os-resource__label");
  type(8, ".os-resource__value");
  type(9, ".os-clock");
  type(10, ".os-menu");
  type(11, "body");
  type(13, ".os-settings-nav__label");

  // Menu bar.
  add("menubar.height", box(".os-menubar")?.height);
  add("menubar.pad-x", padding(".os-menubar"));
  add("menubar.logo.width", box(".os-logo")?.width);
  add("menubar.logo.height", box(".os-logo")?.height);
  const app = style(".os-menubar__app");
  if (app) {
    add("menubar.app.font-px", num(app.fontSize));
    add("menubar.app.tracking", num(app.letterSpacing));
    add("menubar.app.weight", weight(app.fontWeight));
    add("menubar.app.max-width", num(app.maxWidth));
  }
  const menu = style(".os-menu");
  if (menu) {
    add("menubar.menu.item-height", box(".os-menu")?.height);
    add("menubar.menu.pad-x", num(menu.paddingLeft));
    add("menubar.menu.gap", num(style(".os-menus")?.gap));
    add("menubar.menu.radius", num(menu.borderRadius));
    add("menubar.menu.font-px", num(menu.fontSize));
    add("menubar.menu.weight", weight(menu.fontWeight));
    add("menubar.menu.color", color(menu.color));
  }
  const sync = style(".os-status--sync");
  if (sync) {
    add("menubar.sync.height", box(".os-status--sync")?.height);
    add("menubar.sync.radius", num(sync.borderRadius));
    add("menubar.sync.font-px", num(sync.fontSize));
    add("menubar.sync.border", color(sync.borderTopColor));
  }
  const resources = style(".os-resources");
  if (resources) {
    add("menubar.resources.height", box(".os-resources")?.height);
    add("menubar.resources.pad-x", num(resources.paddingLeft));
    add("menubar.resources.gap", num(resources.gap));
  }
  const label = style(".os-resource__label");
  if (label) {
    add("menubar.resources.label-px", num(label.fontSize));
    add("menubar.resources.label-tracking", num(label.letterSpacing));
    add("menubar.resources.label-weight", weight(label.fontWeight));
    add("menubar.resources.label-color", color(label.color));
  }
  const value = style(".os-resource__value");
  if (value) {
    add("menubar.resources.value-px", num(value.fontSize));
    add("menubar.resources.value-weight", weight(value.fontWeight));
    add("menubar.resources.value-color", color(value.color));
  }
  add("menubar.resources.meter-w", box(".os-resource__meter")?.width);
  add("menubar.resources.meter-h", box(".os-resource__meter")?.height);
  const clock = style(".os-clock");
  if (clock) {
    add("menubar.clock.height", box(".os-clock")?.height);
    add("menubar.clock.pad-x", num(clock.paddingLeft));
    add("menubar.clock.gap", num(clock.gap));
    add("menubar.clock.font-px", num(clock.fontSize));
    add("menubar.clock.date-color", color(style(".os-clock [data-clock-date]")?.color));
    add("menubar.clock.time-color", color(style(".os-clock b")?.color));
    add("menubar.clock.time-weight", weight(style(".os-clock b")?.fontWeight));
  }
  add("menubar.tray.button-width", box(".os-status--icon:not([data-battery-tray-value])")?.width);
  add("menubar.tray.battery-width", box(".os-status--icon:has([data-battery-tray-value])")?.width);
  add("menubar.tray.icon", box(".os-status__icon, .os-status--icon .icon")?.width);

  // Window chrome.
  const windowStyle = style(".os-window");
  if (windowStyle) {
    add("window.radius", num(windowStyle.borderTopLeftRadius));
  }
  add("window.titlebar-height", box(".os-titlebar")?.height);
  add("window.light-size", box(".os-light")?.width);
  add("window.light-gap", num(style(".os-lights")?.gap));
  const light = style(".os-light--close");
  if (light) add("window.light-close", color(light.backgroundColor));
  const min = style(".os-light--min");
  if (min) add("window.light-min", color(min.backgroundColor));
  const max = style(".os-light--max");
  if (max) add("window.light-max", color(max.backgroundColor));
  const title = style(".os-titlebar__title b");
  if (title) {
    add("window.title-px", num(title.fontSize));
    add("window.title-weight", weight(title.fontWeight));
    add("window.title-tracking", num(title.letterSpacing));
    add("window.title-color", color(title.color));
  }
  const subtitle = style(".os-titlebar__sub");
  if (subtitle) {
    add("window.subtitle-px", num(subtitle.fontSize));
    add("window.subtitle-color", color(subtitle.color));
  }
  add("window.mark", box(".os-app-mark")?.width);
  add("window.mark-icon", box(".os-app-mark .icon")?.width);
  // `inset 0 1px 0 rgba(255, 255, 255, 0.06)` in the window box-shadow.
  const inset = windowStyle?.boxShadow?.match(
    /rgba?\(([^)]+)\)\s+0px\s+1px\s+0px\s+0px\s+inset/,
  );
  if (inset) {
    const parts = inset[1].split(",").map((part) => Number.parseFloat(part));
    add("window.inset-highlight", Math.round((parts[3] ?? 0) * 100) / 100);
  }
  // Tide line variables (`--os-wave-*`).
  add("window.tide.ring", num(rootStyle.getPropertyValue("--os-wave-ring")));
  add("window.tide.cycle-ms", (num(rootStyle.getPropertyValue("--os-wave-cycle")) ?? 0) * 1000);
  add("window.tide.opacity", num(rootStyle.getPropertyValue("--os-wave-peak")));

  // File-menu popover (opened by the caller).
  const pop = style(".os-menu__pop:not([hidden])");
  if (pop && menu) {
    const popBox = box(".os-menu__pop:not([hidden])");
    const menuBox = box(".os-menu[aria-expanded='true']") ?? box(".os-menu");
    if (popBox && menuBox) {
      add("menu.offset", Math.round((popBox.y - (menuBox.y + menuBox.height)) * 100) / 100);
    }
    add("menu.min-width", num(pop.minWidth));
    add("menu.padding", num(pop.paddingLeft));
    add("menu.radius", num(pop.borderRadius));
    add("menu.popover-fill", color(pop.backgroundColor));
    const row = style(".os-menu__pop button");
    if (row) {
      add("menu.row-height", box(".os-menu__pop button")?.height);
      add("menu.row-font-px", num(row.fontSize));
      add("menu.row-weight", weight(row.fontWeight));
      add("menu.icon-gap", num(row.gap));
    }
    add("menu.icon", box(".os-menu__pop .icon")?.width);
  }

  // Dock (the demo hides it below 1360 px; the caller uses 1440).
  const dock = style(".os-dock");
  if (dock && (box(".os-dock")?.width ?? 0) > 0) {
    add("dock.width", box(".os-dock")?.width);
    add("dock.height", box(".os-dock")?.height);
    add("dock.pad", num(dock.paddingTop));
    add("dock.gap", num(dock.gap));
    add("dock.item", box(".os-dock__item")?.width);
    add("dock.tile", box(".os-dock__tile")?.width);
    add("dock.tile-radius", num(style(".os-dock__tile")?.borderTopLeftRadius));
    add("dock.radius", num(dock.borderTopLeftRadius));
  }

  // Tray panel sizes; `trays` are the ids to look for.
  for (const id of trays) {
    const panel = document.querySelector(`[data-tray="${id}"]`);
    if (!panel || panel.hidden) continue;
    const rect = panel.getBoundingClientRect();
    add(`tray.${id}.width`, num(rect.width));
    add(`tray.${id}.height`, num(rect.height));
  }

  // Structure counts.
  add("structure.menus", document.querySelectorAll(".os-menu").length);
  add("structure.dock-items", document.querySelectorAll(".os-dock__item").length);
  add(
    "structure.settings-tabs",
    document.querySelectorAll("#win-settings .os-settings__nav button, #win-settings [data-settings-tab]").length ||
      null,
  );

  return values;
}

async function collectSourceState() {
  const { chromium } = await import("@playwright/test");
  const server = await serveSource();
  const browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    timezoneId: "UTC",
    locale: "en-US",
  });
  page.setDefaultTimeout(10_000);
  await page.goto(server.origin, { waitUntil: "networkidle" });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(600);

  const clickVisible = async (selector) => {
    const clicked = await page.evaluate((sel) => {
      const candidates = [...document.querySelectorAll(sel)];
      const visible = (node) => {
        if (node.closest("[hidden]")) return false;
        const cs = getComputedStyle(node);
        return cs.display !== "none" && cs.visibility !== "hidden" && node.getClientRects().length > 0;
      };
      const target = candidates.find(visible) ?? candidates[0];
      target?.click();
      return Boolean(target);
    }, selector);
    if (!clicked) throw new Error(`no target for ${selector}`);
  };

  const closeAll = async () => {
    await page.evaluate(() => {
      document
        .querySelectorAll('.os-window.is-open [data-window-action="close"]')
        .forEach((button) => button.click());
      document.querySelectorAll("[data-tray]").forEach((tray) => (tray.hidden = true));
    });
    await page.waitForTimeout(200);
  };

  // A window has to be open for its chrome to have a layout box.
  await clickVisible('[data-cmd="open-explorer"]');
  await page.waitForTimeout(500);
  const snapshot = () => page.evaluate(collectSource, TRAYS);
  const only = (all, prefix) =>
    Object.fromEntries(Object.entries(all).filter(([key]) => key.startsWith(prefix)));

  const values = await snapshot();

  // Popover values need the File menu open.
  await page.getByRole("button", { name: "File", exact: true }).first().click();
  await page.waitForTimeout(300);
  Object.assign(values, only(await snapshot(), "menu."));
  await page.keyboard.press("Escape");
  await page.waitForTimeout(200);

  // Tray panels, one at a time.
  for (const id of TRAYS) {
    await closeAll();
    await clickVisible(`[data-tray-toggle="${id}"]`);
    await page.waitForTimeout(350);
    Object.assign(values, only(await snapshot(), "tray."));
    await page.keyboard.press("Escape");
    await page.waitForTimeout(150);
  }

  // The demo hides its dock below 1360 px; measure it at 1440.
  await closeAll();
  await page.evaluate(() => {
    document.querySelectorAll(".os-window").forEach((node) => {
      node.hidden = true;
      node.classList.remove("is-open", "is-focused");
    });
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.evaluate(() => {
    document.documentElement.dataset.dock = "on";
    const dock = document.querySelector(".os-dock-wrap");
    if (dock) dock.hidden = false;
  });
  await page.waitForTimeout(500);
  Object.assign(values, only(await snapshot(), "dock."));

  const revision = gitRevision(sourceRepo);
  await browser.close();
  server.close();

  return {
    side: "source",
    project: "Water OS demo",
    revision,
    viewport: { width: 1280, height: 800 },
    values,
  };
}

/* ----------------------------------------------------------- aether side */

function collectAether() {
  const output = execFileSync("docker", ["exec", container, manifestBin], {
    encoding: "utf8",
    maxBuffer: 4 * 1024 * 1024,
  });
  const manifest = JSON.parse(output);
  return {
    ...manifest,
    project: "Aether compositor",
    revision: gitRevision(root),
  };
}

/* ------------------------------------------------------------------ diff */

const COLOR = /^#[0-9a-f]{6}([0-9a-f]{2})?$/;
const TOLERANCE_PX = 0.75;
const TOLERANCE_COLOR = 2;

function channel(hex, index) {
  return Number.parseInt(hex.slice(1 + index * 2, 3 + index * 2), 16);
}

function colorClose(left, right) {
  const a = left.length === 7 ? `${left}ff` : left;
  const b = right.length === 7 ? `${right}ff` : right;
  for (let index = 0; index < 4; index += 1) {
    if (Math.abs(channel(a, index) - channel(b, index)) > TOLERANCE_COLOR) return false;
  }
  return true;
}

function same(left, right) {
  if (typeof left === "number" && typeof right === "number") {
    return Math.abs(left - right) <= TOLERANCE_PX;
  }
  if (typeof left === "string" && typeof right === "string") {
    if (COLOR.test(left) && COLOR.test(right)) return colorClose(left, right);
    return left === right;
  }
  return left === right;
}

async function loadBaseline() {
  if (!(await exists(baselinePath))) return {};
  try {
    return JSON.parse(await readFile(baselinePath, "utf8"));
  } catch (error) {
    throw new Error(`baseline.json is not valid JSON: ${error.message}`);
  }
}

function compare(source, aether, baseline) {
  const keys = new Set([...Object.keys(source.values), ...Object.keys(aether.values)]);
  const differences = [];
  const accepted = [];
  let compared = 0;
  let matched = 0;

  for (const key of [...keys].sort()) {
    const left = source.values[key];
    const right = aether.values[key];
    const inSource = left !== undefined;
    const inAether = right !== undefined;
    if (!inSource || !inAether) {
      differences.push({
        key,
        source: inSource ? left : null,
        aether: inAether ? right : null,
        kind: inSource ? "source-only" : "aether-only",
      });
      continue;
    }
    compared += 1;
    if (same(left, right)) {
      matched += 1;
      continue;
    }
    const entry = {
      key,
      source: left,
      aether: right,
      kind: "value",
    };
    if (typeof left === "number" && typeof right === "number") {
      entry.delta = Math.round((right - left) * 100) / 100;
    }
    if (baseline[key]) {
      entry.reason = baseline[key];
      accepted.push(entry);
    } else {
      differences.push(entry);
    }
  }

  return {
    summary: {
      compared,
      matched,
      mismatched: compared - matched,
      accepted: accepted.length,
      aetherOnly: differences.filter((entry) => entry.kind === "aether-only").length,
      sourceOnly: differences.filter((entry) => entry.kind === "source-only").length,
      failures: differences.filter((entry) => entry.kind === "value").length,
    },
    differences,
    accepted,
  };
}

/* --------------------------------------------------------------- reports */

function markdown(state, diff) {
  const lines = [
    "# Design state · Water OS vs Aether",
    "",
    `Source: \`${state.source.revision}\`  `,
    `Aether: \`${state.aether.revision}\`  `,
    `Generated by \`node scripts/screenshots/design-state.mjs\`.`,
    "",
    "## Summary",
    "",
    `- compared: **${diff.summary.compared}**`,
    `- matching: **${diff.summary.matched}**`,
    `- mismatched: **${diff.summary.mismatched}** (${diff.summary.accepted} accepted, ${diff.summary.failures} to fix)`,
    `- Aether-only keys: ${diff.summary.aetherOnly}`,
    `- Source-only keys: ${diff.summary.sourceOnly}`,
    "",
    "## Differences",
    "",
    "| key | source | aether | Δ | note |",
    "| --- | --- | --- | --- | --- |",
    ...diff.differences
      .filter((entry) => entry.kind === "value")
      .map(
        (entry) =>
          `| \`${entry.key}\` | \`${entry.source}\` | \`${entry.aether}\` | ${
            entry.delta === undefined ? "—" : entry.delta > 0 ? `+${entry.delta}` : entry.delta
          } | |`,
      ),
    "",
    "## Accepted differences",
    "",
    "| key | source | aether | reason |",
    "| --- | --- | --- | --- |",
    ...diff.accepted.map(
      (entry) => `| \`${entry.key}\` | \`${entry.source}\` | \`${entry.aether}\` | ${entry.reason} |`,
    ),
    "",
  ];
  if (diff.differences.some((entry) => entry.kind !== "value")) {
    lines.push("## Coverage", "");
    for (const entry of diff.differences.filter((entry) => entry.kind !== "value")) {
      lines.push(`- \`${entry.key}\` — ${entry.kind}`);
    }
    lines.push("");
  }
  return lines.join("\n");
}

/* ------------------------------------------------------------------ main */

async function main() {
  await mkdir(outDir, { recursive: true });
  let source;
  let aether;

  if (diffOnly) {
    source = JSON.parse(await readFile(join(outDir, "source.json"), "utf8"));
    aether = JSON.parse(await readFile(join(outDir, "aether.json"), "utf8"));
    console.log("reusing state/source.json and state/aether.json");
  } else {
    console.log("source: reading computed styles…");
    source = await collectSourceState();
    console.log("aether: reading design-manifest…");
    aether = collectAether();
    await writeFile(join(outDir, "source.json"), `${JSON.stringify(source, null, 2)}\n`);
    await writeFile(join(outDir, "aether.json"), `${JSON.stringify(aether, null, 2)}\n`);
  }

  const baseline = await loadBaseline();
  const diff = compare(source, aether, baseline);
  const document = {
    source: { project: source.project, revision: source.revision },
    aether: { project: aether.project, revision: aether.revision },
    ...diff,
  };
  await writeFile(join(outDir, "diff.json"), `${JSON.stringify(document, null, 2)}\n`);
  await writeFile(join(outDir, "diff.md"), `${markdown({ source, aether }, diff)}\n`);

  console.log(
    `state: ${diff.summary.compared} compared · ${diff.summary.matched} match · ` +
      `${diff.summary.failures} mismatch · ${diff.summary.accepted} accepted`,
  );
  for (const entry of diff.differences.filter((item) => item.kind === "value")) {
    console.warn(`mismatch ${entry.key}: source ${entry.source} vs aether ${entry.aether}`);
  }
  console.log(`state: ${outDir}`);
  if (diff.summary.failures > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(`error: ${error.stack ?? error.message}`);
  process.exit(1);
});