#!/usr/bin/env node
/**
 * Per-app design parity: capture every Water OS demo surface as an element
 * screenshot, crop the matching Aether surface out of the compositor captures,
 * and write a side-by-side report.
 *
 *   node scripts/screenshots/app-parity.mjs
 *   node scripts/screenshots/app-parity.mjs --source-only   # refresh the demo
 *   node scripts/screenshots/app-parity.mjs --report-only   # rebuild the HTML
 *
 * Inputs
 *   live demo:  https://403access.github.io/data-cloud-compare/design-system/os/
 *   Aether:     artifacts/screenshots/shell/*.png (from bun run screenshots:shell)
 *
 * Outputs (committed)
 *   reports/os-app-parity/index.html
 *   reports/os-app-parity/ROADMAP.md
 *   reports/os-app-parity/source/*.png
 *   reports/os-app-parity/aether/*.png
 *
 * Screenshots are always a single window, panel, menu or popover. Never the
 * whole desktop.
 */
import { createReadStream } from "node:fs";
import { mkdir, readFile, writeFile, access, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { execFileSync } from "node:child_process";
import { dirname, extname, join, normalize, resolve } from "node:path";
import { PNG } from "pngjs";

const LIVE_ORIGIN =
  "https://403access.github.io/data-cloud-compare/design-system/os/";
const FIXED_UNIX = 1_795_020_420; // 2026-11-18 16:47 UTC.

const root = resolve(dirname(new URL(import.meta.url).pathname), "../..");
// The source of truth is the local Water OS concept. Serving it locally also
// picks up commits that have not been deployed yet.
const sourceRepo = process.env.OS_SOURCE_REPO ?? resolve(root, "..", "jottacloud-comparison");
const sourceRoot = join(sourceRepo, "design-system");
const shellDir = join(root, "artifacts", "screenshots", "shell");
const outDir = join(root, "reports", "os-app-parity");
const sourceDir = join(outDir, "source");
const aetherDir = join(outDir, "aether");

/** Demo apps and the command that opens each one. */
const SOURCE_APPS = [
  { id: "compare", cmd: "open-compare" },
  { id: "explorer", cmd: "open-explorer" },
  { id: "settings", cmd: "open-settings" },
  { id: "network", cmd: "open-network" },
  { id: "battery", cmd: "open-battery" },
  { id: "devices", cmd: "open-devices" },
  { id: "calendar", cmd: "open-calendar" },
  { id: "taskmanager", cmd: "open-taskmanager" },
  { id: "terminal", cmd: "open-terminal" },
  { id: "music", cmd: "open-music" },
  { id: "shortcuts", cmd: "open-shortcuts" },
  { id: "about", cmd: "open-about" },
  { id: "builder", cmd: "open-builder" },
];

/** System chrome surfaces in the demo. */
const SOURCE_CHROME = [
  { id: "menubar", selector: ".os-menubar" },
  { id: "dock", selector: ".os-dock-wrap" },
  { id: "command", cmd: "open-command", selector: ".osx-command" },
];

/** Tray popovers, opened from their menu-bar buttons. */
const SOURCE_TRAYS = [
  { id: "tray-network", toggle: "network", selector: '[data-tray="network"]' },
  { id: "tray-devices", toggle: "devices", selector: '[data-tray="devices"]' },
  { id: "tray-battery", toggle: "battery", selector: '[data-tray="battery"]' },
  { id: "tray-sound", toggle: "sound", selector: '[data-tray="sound"]' },
  { id: "tray-calendar", toggle: "calendar", selector: '[data-tray="calendar"]' },
];

/**
 * Aether crops. Rectangles mirror the geometry in aether-shell: 1280 x 800
 * output, 38 px menu bar, dock 8 px above the bottom edge.
 */
const AETHER_CROPS = {
  menubar: { file: "desktop.png", rect: [0, 0, 1280, 38] },
  dock: { file: "desktop.png", rect: [420, 730, 439, 62] },
  menu: { file: "menu.png", rect: [110, 0, 250, 215] },
  command: { file: "palette-default.png", rect: [300, 190, 680, 420] },
  explorer: { file: "explorer.png", rect: [16, 54, 1248, 610] },
  settings: { file: "themes.png", rect: [310, 56, 660, 688] },
  calendar: { file: "calendar.png", rect: [290, 88, 700, 590] },
  taskmanager: { file: "task-manager.png", rect: [0, 38, 1280, 688] },
  terminal: { file: "client-window.png", rect: [16, 54, 619, 730] },
  shortcuts: { file: "shortcuts.png", rect: [260, 120, 760, 560] },
  audio: { file: "audio.png", rect: [310, 120, 660, 560] },
  chat: { file: "chat.png", rect: [864, 54, 400, 660] },
  agents: { file: "sandboxes.png", rect: [0, 38, 1280, 688] },
  remote: { file: "remote.png", rect: [330, 190, 620, 420] },
};

/** The checklist that drives the report. `aether` names a crop key. */
const SURFACES = [
  {
    group: "System chrome",
    title: "Menu bar",
    priority: "P0 · done",
    where: "`app/overlays/menubar/`",
    source: "menubar",
    aether: "menubar",
    status: "implemented",
    notes:
      "Brand, active app, five menus, sync pill, CPU/RAM/STORAGE meters, tray and clock. Geometry within 2-3 px; icon glyphs differ.",
  },
  {
    group: "System chrome",
    title: "Dock",
    priority: "P0 · done",
    where: "`app/overlays/dock.rs`",
    source: "dock",
    aether: "dock",
    status: "implemented",
    notes:
      "Same 46 px items, 38 px tinted tiles, separator and running dot. The demo dock has ten tiles (adding Projects and App Studio); Aether ships eight for its own apps.",
  },
  {
    group: "System chrome",
    title: "App menus",
    priority: "P0 · done",
    where: "`app/overlays/menubar/layout.rs`",
    source: "menu",
    aether: "menu",
    status: "implemented",
    notes:
      "File/View/Go/Window/Help open popovers that run real commands. The Aether crop includes the bar segment above the popover.",
  },
  {
    group: "System chrome",
    title: "Command overlay",
    priority: "P2",
    where: "`crates/aether-shell/src/shell/palette.rs`",
    source: "command",
    aether: "command",
    status: "partial",
    notes:
      "Both are ⌘K surfaces. The demo uses the explorer's command palette (groups, filters, previews); Aether uses its own providers and row language.",
  },
  {
    group: "Apps",
    title: "Data Cloud Compare",
    priority: "—",
    where: "product-specific",
    source: "compare",
    aether: null,
    status: "missing",
    notes:
      "The product's two-pane cloud-vs-index comparator with match badges, legend and command bar. Aether has no cloud archive model.",
  },
  {
    group: "Apps",
    title: "Explorer",
    priority: "P2",
    where: "`render/surfaces/explorer.rs`",
    source: "explorer",
    aether: "explorer",
    status: "partial",
    notes:
      "Window geometry, title bar, app bar, pane card, rows and status strip match. The demo explorer is a single-pane file browser with a breadcrumb and search row; Aether's is a single indexed pane.",
  },
  {
    group: "Apps",
    title: "Settings",
    priority: "P2",
    where: "`render/surfaces/themes.rs`",
    source: "settings",
    aether: "settings",
    status: "partial",
    notes:
      "The demo Settings app is tabbed: General, Appearance, Displays, Desktop & Dock, Search, Widgets, Accessibility, Network, Bluetooth, Sound, Battery, Privacy, Screen Time and Storage. Aether only ships the Water settings panel (Themes).",
  },
  {
    group: "Apps",
    title: "Network",
    priority: "P1",
    where: "—",
    source: "network",
    aether: null,
    status: "missing",
    notes: "Wi-Fi, VPN and known networks. The menu bar Wi-Fi tray is a stub that logs.",
  },
  {
    group: "Apps",
    title: "Battery",
    priority: "P1",
    where: "—",
    source: "battery",
    aether: null,
    status: "missing",
    notes: "Charge, health, energy mode and history. The menu bar battery item is a stub.",
  },
  {
    group: "Apps",
    title: "Devices",
    priority: "P1",
    where: "—",
    source: "devices",
    aether: null,
    status: "missing",
    notes: "Bluetooth accessories, pairing and battery levels. The tray item is a stub.",
  },
  {
    group: "Apps",
    title: "Calendar",
    priority: "P0 · done",
    where: "`app/overlays/calendar.rs`",
    source: "calendar",
    aether: "calendar",
    status: "implemented",
    notes:
      "Six-row month grid, today fill, selected rim, event dots and agenda column. Keyboard navigation implemented.",
  },
  {
    group: "Apps",
    title: "Task Manager",
    priority: "P1",
    where: "`app/overlays/task_manager.rs`",
    source: "taskmanager",
    aether: "taskmanager",
    status: "partial",
    notes:
      "Live /proc sampling, tabs and a sorted table in both. Aether runs it as a full-width panel below the menu bar; the demo uses a floating window with its own title bar.",
  },
  {
    group: "Apps",
    title: "Terminal",
    priority: "P0 · done",
    where: "`render/window_frame.rs`",
    source: "terminal",
    aether: "terminal",
    status: "implemented",
    notes:
      "Aether hosts real Wayland clients (foot) with the compositor's Water title bar; the demo renders a mock terminal inside its own window.",
  },
  {
    group: "Apps",
    title: "Tide Radio",
    priority: "P3",
    where: "—",
    source: "music",
    aether: null,
    status: "missing",
    notes: "Playback, queue and presets. Aether has an audio-route overlay but no player.",
  },
  {
    group: "Apps",
    title: "Keyboard shortcuts",
    priority: "P2",
    where: "`app/overlays/shortcuts.rs`",
    source: "shortcuts",
    aether: "shortcuts",
    status: "partial",
    notes:
      "Both are Super+/ cheatsheets. The demo groups the explorer key map; Aether lists the compositor's own surfaces and window actions.",
  },
  {
    group: "Apps",
    title: "About Aether OS",
    priority: "P2",
    where: "—",
    source: "about",
    aether: null,
    status: "missing",
    notes: "Version, build, hardware and licence. The menu-bar logo currently opens the cheatsheet.",
  },
  {
    group: "Apps",
    title: "App Studio",
    priority: "—",
    where: "product-specific",
    source: "builder",
    aether: null,
    status: "missing",
    notes: "The demo's GUI builder for app surfaces. No Aether equivalent.",
  },
  {
    group: "System trays",
    title: "Wi-Fi tray",
    priority: "P1",
    where: "—",
    source: "tray-network",
    aether: null,
    status: "missing",
    notes: "Connected network, known networks, VPN state, link to Network.",
  },
  {
    group: "System trays",
    title: "Bluetooth tray",
    priority: "P1",
    where: "—",
    source: "tray-devices",
    aether: null,
    status: "missing",
    notes: "Accessory list and toggle, link to Devices.",
  },
  {
    group: "System trays",
    title: "Battery tray",
    priority: "P1",
    where: "—",
    source: "tray-battery",
    aether: null,
    status: "missing",
    notes: "Charge figure, energy mode and time remaining.",
  },
  {
    group: "System trays",
    title: "Sound tray",
    priority: "P3",
    where: "—",
    source: "tray-sound",
    aether: null,
    status: "missing",
    notes: "Output selection and volume. Aether's audio overlay covers the routing UI but not this tray.",
  },
  {
    group: "System trays",
    title: "Calendar tray",
    priority: "P2",
    where: "—",
    source: "tray-calendar",
    aether: null,
    status: "partial",
    notes:
      "The demo opens a month popover from the clock and offers Open Calendar. Aether opens the Calendar window directly.",
  },
  {
    group: "Aether-only",
    title: "Audio routes",
    priority: "P0 · done",
    where: "`render/surfaces/audio.rs`",
    source: null,
    aether: "audio",
    status: "aether-only",
    notes: "660 x 560 overlay for outputs, inputs, presets and the input meter.",
  },
  {
    group: "Aether-only",
    title: "Chat drawer",
    priority: "P0 · done",
    where: "`render/surfaces/chat.rs`",
    source: null,
    aether: "chat",
    status: "aether-only",
    notes: "Streaming markdown with MCP tool traces.",
  },
  {
    group: "Aether-only",
    title: "Agent sandboxes",
    priority: "P0 · done",
    where: "`render/surfaces/sandboxes.rs`",
    source: null,
    aether: "agents",
    status: "aether-only",
    notes: "Isolation tiers and per-agent resource use.",
  },
  {
    group: "Aether-only",
    title: "Remote display",
    priority: "P0 · done",
    where: "`render/surfaces/remote.rs`",
    source: null,
    aether: "remote",
    status: "aether-only",
    notes: "Pairing code, WebRTC transport and the recording indicator.",
  },
];

function sourceRevision() {
  try {
    const head = execFileSync("git", ["-C", sourceRepo, "log", "-1", "--format=%h %s"], {
      encoding: "utf8",
    }).trim();
    const dirty = execFileSync("git", ["-C", sourceRepo, "status", "--porcelain"], {
      encoding: "utf8",
    }).trim().length > 0;
    return dirty ? `${head} (+ local changes)` : head;
  } catch {
    return null;
  }
}

let SOURCE = { origin: LIVE_ORIGIN, revision: "live deployment" };
{
  const revision = sourceRevision();
  if (revision) SOURCE = { origin: "local design-system/ checkout", revision };
}
const AETHER_REVISION = (() => {
  try {
    return execFileSync("git", ["-C", root, "log", "-1", "--format=%h %s"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return "working tree";
  }
})();

const STATUS_LABEL = {
  implemented: "Implemented",
  partial: "Partial",
  missing: "Missing",
  "aether-only": "Aether only",
};

const args = new Set(process.argv.slice(2));

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
};

async function serveSource() {
  if (!(await exists(join(sourceRoot, "os", "index.html")))) return null;
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
  const revision = (() => {
    try {
      return execFileSync("git", ["-C", sourceRepo, "log", "-1", "--format=%h %s"], {
        encoding: "utf8",
      }).trim();
    } catch {
      return "unknown";
    }
  })();
  return {
    origin: `http://127.0.0.1:${port}/os/`,
    revision,
    close: () => server.close(),
  };
}

async function captureSource(origin) {
  const { chromium } = await import("@playwright/test");
  await mkdir(sourceDir, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    timezoneId: "UTC",
    locale: "en-US",
  });
  await page.addInitScript((fixed) => {
    const RealDate = Date;
    const offset = fixed * 1000 - RealDate.now();
    class FixedDate extends RealDate {
      constructor(...values) {
        if (values.length === 0) super(RealDate.now() + offset);
        else super(...values);
      }
      static now() {
        return RealDate.now() + offset;
      }
    }
    window.Date = FixedDate;
  }, FIXED_UNIX);
  await page.goto(origin, { waitUntil: "networkidle" });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(800);

  const closeAll = async () => {
    await page.evaluate(() => {
      document
        .querySelectorAll('.os-window.is-open [data-window-action="close"]')
        .forEach((button) => button.click());
      document.querySelectorAll("[data-tray]").forEach((tray) => (tray.hidden = true));
    });
    await page.waitForTimeout(250);
  };

  // Commands live in menus that are hidden until opened. Clicking the element
  // directly still runs the demo's command handler, which is what a user would
  // reach through Window or the dock.
  const clickVisible = async (selector) => {
    const clicked = await page.evaluate((sel) => {
      const candidates = [...document.querySelectorAll(sel)];
      const isVisible = (element) => {
        if (element.closest("[hidden]")) return false;
        const style = getComputedStyle(element);
        if (style.display === "none" || style.visibility === "hidden") return false;
        return element.getClientRects().length > 0;
      };
      const target = candidates.find(isVisible) ?? candidates[0];
      target?.click();
      return Boolean(target);
    }, selector);
    if (!clicked) throw new Error(`no target for ${selector}`);
  };

  const shot = async (selector, file) => {
    const locator = page.locator(selector).first();
    await locator.waitFor({ state: "visible", timeout: 5000 });
    await locator.scrollIntoViewIfNeeded().catch(() => {});
    await locator.screenshot({ path: join(sourceDir, `${file}.png`) });
    const box = await locator.boundingBox();
    return box;
  };

  for (const app of SOURCE_APPS) {
    await closeAll();
    await clickVisible(`[data-cmd="${app.cmd}"]`);
    await page.waitForTimeout(700);
    await shot(`#win-${app.id}`, app.id);
  }

  await closeAll();
  await shot(".os-menubar", "menubar");
  await shot(".os-dock-wrap", "dock");

  // Command overlay is modal: close it before moving on.
  await clickVisible('[data-cmd="open-command"]');
  await page.waitForTimeout(700);
  await shot(".osx-command", "command");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);

  // File menu popover, rooted at the menu button so the crop shows the bar too.
  await closeAll();
  const file = page.getByRole("button", { name: "File", exact: true }).first();
  const fileBox = await file.boundingBox();
  await file.click();
  await page.waitForTimeout(300);
  const popover = page.locator(".os-menu__pop:not([hidden])").first();
  const menuBox = await popover.boundingBox();
  if (menuBox && fileBox) {
    const x = Math.max(0, Math.min(fileBox.x, menuBox.x) - 10);
    await page.screenshot({
      path: join(sourceDir, "menu.png"),
      clip: {
        x,
        y: 0,
        width: Math.min(1280 - x, menuBox.x + menuBox.width + 10 - x),
        height: menuBox.y + menuBox.height + 8,
      },
    });
  }
  await page.keyboard.press("Escape");

  for (const tray of SOURCE_TRAYS) {
    await closeAll();
    await clickVisible(`[data-tray-toggle="${tray.toggle}"]`);
    await page.waitForTimeout(400);
    await shot(tray.selector, tray.id);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(200);
  }

  await browser.close();
  console.log(`source: ${sourceDir}`);
}

function readPng(path) {
  return readFile(path).then((buffer) => PNG.sync.read(buffer));
}

function crop(png, rect) {
  const [x, y, width, height] = rect;
  const out = new PNG({ width, height });
  PNG.bitblt(png, out, x, y, width, height, 0, 0);
  return PNG.sync.write(out);
}

async function cropAether() {
  await mkdir(aetherDir, { recursive: true });
  const cache = new Map();
  for (const [name, spec] of Object.entries(AETHER_CROPS)) {
    const path = join(shellDir, spec.file);
    if (!(await exists(path))) {
      console.warn(`skip ${name}: missing ${path}`);
      continue;
    }
    let png = cache.get(path);
    if (!png) {
      png = await readPng(path);
      cache.set(path, png);
    }
    const [x, y, width, height] = spec.rect;
    if (x + width > png.width || y + height > png.height) {
      console.warn(`skip ${name}: crop outside ${spec.file}`);
      continue;
    }
    await writeFile(join(aetherDir, `${name}.png`), crop(png, spec.rect));
  }
  console.log(`aether: ${aetherDir}`);
}

function imageSize(buffer) {
  const png = PNG.sync.read(buffer);
  return `${png.width}x${png.height}`;
}

async function buildReport() {
  await mkdir(outDir, { recursive: true });
  const counts = { implemented: 0, partial: 0, missing: 0, "aether-only": 0 };
  for (const surface of SURFACES) counts[surface.status] += 1;

  const cards = [];
  let group = "";
  for (const surface of SURFACES) {
    if (surface.group !== group) {
      group = surface.group;
      cards.push(`<h2 class="group">${group}</h2>`);
    }
    const sourceFile = `source/${surface.source}.png`;
    const aetherFile = surface.aether ? `aether/${surface.aether}.png` : null;
    const hasSource = surface.source && (await exists(join(outDir, sourceFile)));
    const hasAether = aetherFile && (await exists(join(outDir, aetherFile)));
    const left = hasSource
      ? `<figure><img src="${sourceFile}" alt="Demo ${surface.title}"><figcaption>Source · ${await imageSize(await readFile(join(outDir, sourceFile)))}</figcaption></figure>`
      : `<figure class="empty"><div>No source surface</div><figcaption>—</figcaption></figure>`;
    const right = hasAether
      ? `<figure><img src="${aetherFile}" alt="Aether ${surface.title}"><figcaption>Aether · ${await imageSize(await readFile(join(outDir, aetherFile)))}</figcaption></figure>`
      : `<figure class="empty"><div>Not implemented yet</div><figcaption>roadmap</figcaption></figure>`;
    cards.push(`
      <section class="surface">
        <header>
          <h3>${surface.title}</h3>
          <span class="priority">${surface.priority}</span>
          <span class="pill pill--${surface.status}">${STATUS_LABEL[surface.status]}</span>
        </header>
        <div class="pair">${left}${right}</div>
        <p class="notes">${surface.notes}</p>
      </section>`);
  }

  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Water OS app parity · Aether</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  :root { color-scheme: dark; --bg:#031922; --panel:#072733; --line:rgba(127,230,255,.18); --text:#e9fbff; --dim:#a5dbe5; --mute:#6ba9b6; --accent:#38d6ff; --green:#5ef2b0; --amber:#ffd27a; --red:#ff8a92; --violet:#c3a8ff; }
  * { box-sizing: border-box; }
  body { margin:0; padding:32px clamp(16px,4vw,56px) 64px; background:radial-gradient(1200px 700px at 12% -10%, rgba(46,196,224,.22), transparent 60%), linear-gradient(165deg,#083545,#052530 48%,#031922); color:var(--text); font:14px/1.5 -apple-system,BlinkMacSystemFont,"Inter","Segoe UI",sans-serif; }
  h1 { margin:0 0 6px; font-size:30px; letter-spacing:-.02em; }
  .lede { margin:0 0 18px; max-width:900px; color:var(--dim); }
  .meta { display:flex; flex-wrap:wrap; gap:10px 18px; margin:0 0 26px; color:var(--mute); font-size:12px; }
  .meta a { color:var(--accent); }
  .stats { display:flex; flex-wrap:wrap; gap:10px; margin:0 0 30px; }
  .stat { border:1px solid var(--line); border-radius:10px; padding:10px 14px; background:rgba(7,39,51,.7); min-width:120px; }
  .stat b { display:block; font-size:22px; }
  .stat span { color:var(--mute); font-size:11px; text-transform:uppercase; letter-spacing:.08em; }
  h2.group { margin:36px 0 12px; font-size:12px; letter-spacing:.14em; text-transform:uppercase; color:var(--mute); border-bottom:1px solid var(--line); padding-bottom:8px; }
  .surface { border:1px solid var(--line); border-radius:14px; background:rgba(7,39,51,.55); padding:14px 16px 12px; margin:0 0 18px; }
  .surface header { display:flex; align-items:center; gap:10px; margin-bottom:10px; }
  .surface h3 { margin:0; font-size:16px; letter-spacing:-.01em; }
  .priority { margin-left:auto; color:var(--mute); font-size:10px; letter-spacing:.08em; text-transform:uppercase; }
  .pill { font-size:10px; letter-spacing:.1em; text-transform:uppercase; padding:3px 9px; border-radius:999px; border:1px solid; }
  .pill--implemented { color:var(--green); border-color:color-mix(in srgb,var(--green) 40%,transparent); background:color-mix(in srgb,var(--green) 12%,transparent); }
  .pill--partial { color:var(--amber); border-color:color-mix(in srgb,var(--amber) 40%,transparent); background:color-mix(in srgb,var(--amber) 12%,transparent); }
  .pill--missing { color:var(--red); border-color:color-mix(in srgb,var(--red) 40%,transparent); background:color-mix(in srgb,var(--red) 12%,transparent); }
  .pill--aether-only { color:var(--violet); border-color:color-mix(in srgb,var(--violet) 40%,transparent); background:color-mix(in srgb,var(--violet) 12%,transparent); }
  .pair { display:grid; grid-template-columns:repeat(auto-fit,minmax(340px,1fr)); gap:14px; align-items:start; }
  figure { margin:0; border:1px solid var(--line); border-radius:10px; overflow:hidden; background:#041c26; display:flex; flex-direction:column; }
  figure img { flex:1 1 auto; object-fit:contain; }
  figure img { display:block; width:auto; max-width:100%; height:auto; margin:0 auto; }
  figcaption { padding:5px 9px; font-size:10px; color:var(--mute); border-top:1px solid var(--line); font-variant-numeric:tabular-nums; }
  figure.empty div { display:grid; place-items:center; min-height:150px; color:var(--mute); font-size:12px; border-bottom:1px dashed var(--line); }
  .notes { margin:10px 0 0; color:var(--dim); font-size:12.5px; }
  footer { margin-top:40px; color:var(--mute); font-size:11px; }
</style>
</head>
<body>
  <h1>Water OS app parity</h1>
  <p class="lede">Every surface from the Water OS demo next to the current Aether compositor. Captures are element or window crops only — no full-desktop frames.</p>
  <p class="meta">
    <span>Source · ${SOURCE.revision}</span>
    <span>Aether · ${AETHER_REVISION}</span>
    <span>1280×800 · clock pinned to 2026-11-18 16:47 UTC</span>
  </p>
  <div class="stats">
    <div class="stat"><b>${counts.implemented}</b><span>Implemented</span></div>
    <div class="stat"><b>${counts.partial}</b><span>Partial</span></div>
    <div class="stat"><b>${counts.missing}</b><span>Missing</span></div>
    <div class="stat"><b>${counts["aether-only"]}</b><span>Aether only</span></div>
  </div>
  ${cards.join("\n")}
  <footer>Generated by <code>scripts/screenshots/app-parity.mjs</code>. Roadmap: <code>ROADMAP.md</code>.</footer>
</body>
</html>
`;
  await writeFile(join(outDir, "index.html"), html);

  const lines = [
    "# Water OS app parity · roadmap",
    "",
    `Source: \`${SOURCE.revision}\` (${SOURCE.origin})`,
    "",
    `Aether: \`${AETHER_REVISION}\``,
    "",
    `Implemented **${counts.implemented}** · partial **${counts.partial}** · missing **${counts.missing}** · Aether-only **${counts["aether-only"]}**.`,
    "",
    "**Status key** — implemented: matches the source surface · partial: present but a different scope or layout · missing: no Aether surface · Aether only: no source counterpart.",
    "",
    "| Surface | Group | Status | Priority | Where | Notes |",
    "| --- | --- | --- | --- | --- | --- |",
    ...SURFACES.map(
      (surface) =>
        `| ${surface.title} | ${surface.group} | ${STATUS_LABEL[surface.status]} | ${surface.priority} | ${surface.where} | ${surface.notes.replaceAll("|", "\\|")} |`,
    ),
    "",
    "## Next up",
    "",
    "1. **P1 · Network, Battery, Devices** windows and their tray popovers (menu-bar items are stubs today).",
    "2. **P1 · Task Manager as a floating window** instead of a full-width panel.",
    "3. **P2 · Explorer parity**: sidebar/favourites, breadcrumb, in-pane search row and size/modified columns.",
    "4. **P2 · Settings tabs** beyond Water (Appearance, Displays, Desktop & Dock, Search, Accessibility).",
    "5. **P2 · About Aether OS** window behind the menu-bar logo.",
    "6. **P2 · Calendar and Sound trays** (month popover from the clock, output/volume popover).",
    "7. **P3 · Tide Radio** player and the sound tray that opens it.",
    "8. **Not planned**: Data Cloud Compare and App Studio are product-specific to the demo.",
    "",
    "## Regenerate",
    "",
    "```bash",
    "bun run screenshots:shell   # Aether capture set (Docker Xvfb)",
    "bun run parity:apps         # source element shots + Aether crops + this report",
    "```",
    "",
    "`parity:apps` serves `../jottacloud-comparison/design-system` on a loopback port,",
    "so it always compares against the current source checkout rather than the",
    "deployed Pages build. Set `OS_SOURCE_REPO` to point at another checkout.",
    "",
  ];
  await writeFile(join(outDir, "ROADMAP.md"), lines.join("\n"));
  console.log(`report: ${join(outDir, "index.html")}`);
}

if (!args.has("--report-only")) {
  const served = await serveSource();
  if (served) {
    SOURCE = { origin: "local design-system/ checkout", revision: served.revision };
    await captureSource(served.origin);
  } else {
    await captureSource(LIVE_ORIGIN);
  }
}
if (!args.has("--source-only")) {
  await cropAether();
  await buildReport();
}
SOURCE.close?.();