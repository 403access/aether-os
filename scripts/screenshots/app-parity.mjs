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
import { mkdir, readFile, writeFile, access, stat, unlink } from "node:fs/promises";
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
const sceneDir = join(shellDir, "scenes");

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
  menubar: { file: "menubar.png", rect: [0, 0, 1280, 38] },
  dock: { file: "desktop.png", rect: [420, 730, 439, 62] },
  menu: { file: "menu.png", rect: [110, 0, 250, 230] },
  command: { file: "palette-default.png", rect: [300, 190, 680, 420] },
  explorer: { file: "app-explorer.png", rect: [16, 54, 1248, 730] },
  about: { file: "app-about.png", rect: [16, 54, 1248, 730] },
  network: { file: "app-network.png", rect: [16, 54, 1248, 730] },
  battery: { file: "app-battery.png", rect: [16, 54, 1248, 730] },
  devices: { file: "app-devices.png", rect: [16, 54, 1248, 730] },
  radio: { file: "app-radio.png", rect: [16, 54, 1248, 730] },
  settings: { file: "app-settings.png", rect: [16, 54, 1248, 730] },
  "settings-general": { file: "app-settings-general.png", rect: [16, 54, 1248, 730] },
  "settings-appearance": { file: "app-settings-appearance.png", rect: [16, 54, 1248, 730] },
  "settings-displays": { file: "app-settings-displays.png", rect: [16, 54, 1248, 730] },
  "settings-desktop": { file: "app-settings-desktop.png", rect: [16, 54, 1248, 730] },
  "settings-commands": { file: "app-settings-commands.png", rect: [16, 54, 1248, 730] },
  "settings-accessibility": { file: "app-settings-accessibility.png", rect: [16, 54, 1248, 730] },
  "settings-network": { file: "app-settings-network.png", rect: [16, 54, 1248, 730] },
  "settings-bluetooth": { file: "app-settings-bluetooth.png", rect: [16, 54, 1248, 730] },
  "settings-sound": { file: "app-settings-sound.png", rect: [16, 54, 1248, 730] },
  "settings-battery": { file: "app-settings-battery.png", rect: [16, 54, 1248, 730] },
  "settings-notifications": { file: "app-settings-notifications.png", rect: [16, 54, 1248, 730] },
  "settings-security": { file: "app-settings-security.png", rect: [16, 54, 1248, 730] },
  "settings-storage": { file: "app-settings-storage.png", rect: [16, 54, 1248, 730] },
  calendar: { file: "calendar.png", rect: [16, 54, 1248, 730] },
  taskmanager: { file: "app-task-manager.png", rect: [16, 54, 1248, 730] },
  terminal: { file: "client-window.png", rect: [16, 54, 619, 730] },
  shortcuts: { file: "shortcuts.png", rect: [260, 120, 760, 560] },
  audio: { file: "audio.png", rect: [310, 120, 660, 560] },
  chat: { file: "chat.png", rect: [864, 54, 400, 660] },
  agents: { file: "sandboxes.png", rect: [0, 38, 1280, 688] },
  remote: { file: "remote.png", rect: [330, 190, 620, 420] },
  "tray-network": { file: "tray-network.png", rect: [620, 42, 300, 328] },
  "tray-devices": { file: "tray-devices.png", rect: [656, 42, 300, 253] },
  "tray-battery": { file: "tray-battery.png", rect: [736, 42, 286, 270] },
  "tray-sound": { file: "tray-sound.png", rect: [766, 42, 292, 256] },
  "tray-calendar": { file: "tray-calendar.png", rect: [900, 42, 308, 351] },
};

/**
 * Settings panes in the demo's order. A single window screenshot hides twelve
 * of them, so each pane is captured and reviewed on its own.
 */
const SETTINGS_PANES = [
  { id: "general", title: "General", notes: "Host, software update and language/region sections." },
  { id: "appearance", title: "Appearance", notes: "Theme and Water effect switches drive the shell live." },
  { id: "displays", title: "Displays", notes: "Output name, resolution, refresh rate and scale." },
  { id: "desktop", title: "Desktop & Dock", notes: "Working Show dock switch (off by default, like the demo), running indicators, clock and meters." },
  { id: "commands", title: "Search & Commands", notes: "Palette shortcut, prefixes and the search provider." },
  { id: "accessibility", title: "Accessibility", notes: "Reduce motion, text size, focus ring and contrast." },
  { id: "network", title: "Network", notes: "Interface, MAC, IP, router and DNS from the host." },
  { id: "bluetooth", title: "Bluetooth", notes: "Adapter, discoverable switch and paired devices." },
  { id: "sound", title: "Sound", notes: "Output, presets and microphone input." },
  { id: "battery", title: "Battery", notes: "Charge, condition and energy mode." },
  { id: "notifications", title: "Notifications", notes: "Allow/preview/Do Not Disturb plus per-app rows. Aether now ships the demo's 13 panes in the demo order." },
  { id: "security", title: "Privacy & Security", notes: "Agent isolation, telemetry and the audit log." },
  { id: "storage", title: "Storage", notes: "Root filesystem, index and free space." },
];

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
      "Same 46 px items, 38 px tinted tiles, separator and running dot. The demo hides its dock below 1360 px, so the source capture is at 1440; Aether keeps it at 1280. The demo has ten tiles (adding Projects and App Studio); Aether ships eight.",
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
    status: "out-of-scope",
    notes:
      "Ignored by decision: the Data Cloud Compare product surface (cloud-vs-index comparator) is not an Aether app.",
  },
  {
    group: "Apps",
    title: "Explorer",
    priority: "P1",
    where: "`render/surfaces/explorer.rs`",
    source: "explorer",
    aether: "explorer",
    status: "implemented",
    notes:
      "Sidebar (favourites, locations, read-only notice), toolbar with breadcrumb and hidden switch, in-pane search row, Name/Type/Size/Modified columns, footer and status strip. Hidden/Everywhere/Filter are still visual-only.",
  },
  ...SETTINGS_PANES.map((pane) => ({
    group: "Apps",
    title: `Settings · ${pane.title}`,
    priority: "P1",
    where: "`app/system.rs` + `render/surfaces/system_app.rs`",
    source: `settings-${pane.id}`,
    aether: `settings-${pane.id}`,
    status: "partial",
    notes: `${pane.notes} Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn.`,
  })),
  {
    group: "Apps",
    title: "Network",
    priority: "P0 · done",
    where: "`app/system.rs` + `app/system_info.rs`",
    source: "network",
    aether: "network",
    status: "implemented",
    notes:
      "Interfaces from /sys/class/net, IP from fib_trie, router from /proc/net/route and DNS from resolv.conf, in the demo's header/switch/list/details card layout.",
  },
  {
    group: "Apps",
    title: "Battery",
    priority: "P0 · done",
    where: "`app/system.rs` + `app/system_info.rs`",
    source: "battery",
    aether: "battery",
    status: "implemented",
    notes:
      "Charge and estimate, energy mode, low-power and optimised-charging switches, health table and largest CPU consumers. Shows an AC-power state when no battery exists.",
  },
  {
    group: "Apps",
    title: "Devices",
    priority: "P0 · done",
    where: "`app/system.rs` + `app/system_info.rs`",
    source: "devices",
    aether: "devices",
    status: "implemented",
    notes:
      "Bluetooth summary, accessory switches and paired-device list over /sys/class/bluetooth, with an explicit no-adapter state.",
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
    status: "implemented",
    notes:
      "Live /proc sampling, tabs, a sorted table, task-count badge and status strip in the demo's floating window box.",
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
    priority: "P0 · done",
    where: "`app/system.rs`",
    source: "music",
    aether: "radio",
    status: "implemented",
    notes:
      "Player window with artwork, track metadata, progress, transport controls and a five-track queue; Space toggles playback.",
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
    priority: "P0 · done",
    where: "`app/system.rs` + `render/surfaces/system_app.rs`",
    source: "about",
    aether: "about",
    status: "implemented",
    notes: "Version, build, hardware and licence. The menu-bar logo currently opens the cheatsheet.",
  },
  {
    group: "Apps",
    title: "App Studio",
    priority: "—",
    where: "product-specific",
    source: "builder",
    aether: null,
    status: "out-of-scope",
    notes:
      "Ignored by decision: the demo's GUI builder is a product surface, not part of the Aether desktop.",
  },
  {
    group: "System trays",
    title: "Wi-Fi tray",
    priority: "P0 · done",
    where: "`render/surfaces/tray.rs`",
    source: "tray-network",
    aether: "tray-network",
    status: "implemented",
    notes:
      "Wi-Fi switch, connected interface, network list and a Network Settings link. Panel size matches the demo (300 x 328).",
  },
  {
    group: "System trays",
    title: "Bluetooth tray",
    priority: "P0 · done",
    where: "`render/surfaces/tray.rs`",
    source: "tray-devices",
    aether: "tray-devices",
    status: "implemented",
    notes:
      "Bluetooth switch, paired-device area and a Bluetooth Settings link (300 x 253).",
  },
  {
    group: "System trays",
    title: "Battery tray",
    priority: "P0 · done",
    where: "`render/surfaces/tray.rs`",
    source: "tray-battery",
    aether: "tray-battery",
    status: "implemented",
    notes:
      "Charge, estimate, level bar, three-way energy mode and a Battery Settings link (286 x 270).",
  },
  {
    group: "System trays",
    title: "Sound tray",
    priority: "P0 · done",
    where: "`render/surfaces/tray.rs`",
    source: "tray-sound",
    aether: "tray-sound",
    status: "implemented",
    notes:
      "Output name, volume bar and a Sound Settings link (292 x 256). Preset switching stays in the audio overlay.",
  },
  {
    group: "System trays",
    title: "Calendar tray",
    priority: "P0 · done",
    where: "`render/surfaces/tray.rs`",
    source: "tray-calendar",
    aether: "tray-calendar",
    status: "implemented",
    notes:
      "Month popover from the clock with today filled and an Open Calendar link (308 x 351).",
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
let closeServer = null;
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
  "out-of-scope": "Out of scope",
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

/**
 * Runs in the page: serialise every visible element's box, text and computed
 * style. The Aether side emits the same information from its render chunk, so
 * missing labels/boxes show up as data instead of being invisible.
 */
function collectDomScene() {
  const round = (value) => Math.round(value * 100) / 100;
  const skipClass = /(^|\s)(ds-sea|ds-wave|os-water|os-vignette|sprite|skip-link)(\s|$)/;
  const skipTag = new Set(["SCRIPT", "STYLE", "HEAD", "META", "LINK", "CANVAS", "SVG"]);
  const elements = [];

  const walk = (element, depth, path) => {
    if (elements.length >= 4000) return;
    if (skipTag.has(element.tagName)) return;
    if (skipClass.test(typeof element.className === "string" ? element.className : "")) return;
    if (element.closest("[hidden]")) return;
    const style = getComputedStyle(element);
    if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity) === 0) {
      return;
    }
    const rect = element.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return;
    if (
      rect.bottom < 0 ||
      rect.right < 0 ||
      rect.left > window.innerWidth ||
      rect.top > window.innerHeight
    ) {
      return;
    }
    const textNodes = [...element.childNodes].filter(
      (node) => node.nodeType === Node.TEXT_NODE && node.textContent.trim() !== "",
    );
    const ownText = textNodes
      .map((node) => node.textContent)
      .join(" ")
      .replace(/\s+/g, " ")
      .trim();
    // The element box can be a full-width row with centred text; measure the
    // text node's own box so geometry compares text to text.
    let textRect = rect;
    if (textNodes.length > 0) {
      const range = document.createRange();
      range.setStartBefore(textNodes[0]);
      range.setEndAfter(textNodes[textNodes.length - 1]);
      const measured = range.getBoundingClientRect();
      if (measured.width > 0 && measured.height > 0) textRect = measured;
    }
    const fontSize = Number.parseFloat(style.fontSize);
    elements.push({
      path,
      tag: element.tagName.toLowerCase(),
      id: element.id || "",
      role: element.getAttribute("role") || "",
      classes:
        typeof element.className === "string"
          ? element.className.split(/\s+/).filter(Boolean)
          : [],
      text: ownText,
      x: round(textRect.x),
      y: round(textRect.y),
      w: round(textRect.width),
      h: round(textRect.height),
      font: Number.isFinite(fontSize)
        ? {
            px: round(fontSize),
            weight: Number(style.fontWeight) || null,
            tracking: style.letterSpacing,
            lineHeight: style.lineHeight,
          }
        : null,
      background: style.backgroundColor,
      color: style.color,
      radius: style.borderTopLeftRadius,
      border: `${style.borderTopWidth} ${style.borderTopColor}`,
      shadow: style.boxShadow === "none" ? "" : style.boxShadow,
      opacity: Number(style.opacity),
      depth,
    });
    [...element.children].forEach((child, index) =>
      walk(child, depth + 1, `${path}/${child.tagName.toLowerCase()}[${index}]`),
    );
  };

  walk(document.body, 0, "body");
  return { viewport: { width: window.innerWidth, height: window.innerHeight }, elements };
}

async function captureSource(origin) {
  const { chromium } = await import("@playwright/test");
  await mkdir(sourceDir, { recursive: true });
  await mkdir(sceneDir, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    timezoneId: "UTC",
    locale: "en-US",
  });
  // Every element must appear promptly; fail loudly instead of hanging.
  page.setDefaultTimeout(10_000);
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

  const scene = async (file) => {
    const snapshot = await page.evaluate(collectDomScene);
    await writeFile(join(sceneDir, `source-${file}.json`), JSON.stringify(snapshot));
  };

  const shot = async (selector, file) => {
    const locator = page.locator(selector).first();
    await locator.waitFor({ state: "visible", timeout: 5000 });
    await locator.scrollIntoViewIfNeeded().catch(() => {});
    await locator.screenshot({
      path: join(sourceDir, `${file}.png`),
      animations: "disabled",
    });
    await scene(file);
    const box = await locator.boundingBox();
    return box;
  };

  for (const app of SOURCE_APPS) {
    try {
      await closeAll();
      await clickVisible(`[data-cmd="${app.cmd}"]`);
      await page.waitForTimeout(700);
      await shot(`#win-${app.id}`, app.id);
      console.log(`source: app ${app.id}`);
    } catch (error) {
      console.warn(`skip ${app.id}: ${error.message.split("\n")[0]}`);
    }
  }

  // Settings is one window with thirteen panes; capture each pane.
  await closeAll();
  await clickVisible('[data-cmd="open-settings"]');
  await page.waitForTimeout(500);
  for (const pane of SETTINGS_PANES) {
    await page.evaluate((id) => {
      document.querySelector(`[data-settings-tab="${id}"]`)?.click();
    }, pane.id);
    await page.waitForTimeout(220);
    await shot("#win-settings", `settings-${pane.id}`);
    console.log(`source: settings ${pane.id}`);
  }
  await closeAll();

  // Chrome elements are captured with every window hidden so the dock is
  // never occluded by a maximised window.
  await closeAll();
  await page.evaluate(() => {
    document.querySelectorAll(".os-window").forEach((windowElement) => {
      windowElement.hidden = true;
      windowElement.classList.remove("is-open", "is-focused");
    });
  });
  await page.waitForTimeout(300);
  await shot(".os-menubar", "menubar");
  // The demo hides the dock below 1360 px. Capture it at 1440 so the
  // comparison still shows the design, and note the breakpoint.
  try {
    await page.setViewportSize({ width: 1440, height: 900 });
    // The demo ships the dock disabled; enable it for the capture.
    await page.evaluate(() => {
      document.documentElement.dataset.dock = "on";
      const dock = document.querySelector(".os-dock-wrap");
      if (dock) dock.hidden = false;
    });
    await page.waitForTimeout(500);
    await shot(".os-dock-wrap", "dock");
  } catch (error) {
    console.warn(`skip dock: ${error.message.split("\n")[0]}`);
  } finally {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.waitForTimeout(300);
  }
  console.log("source: chrome");

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
  await scene("menu");
  await page.keyboard.press("Escape");

  for (const tray of SOURCE_TRAYS) {
    await closeAll();
    await clickVisible(`[data-tray-toggle="${tray.toggle}"]`);
    await page.waitForTimeout(400);
    await shot(tray.selector, tray.id);
    console.log(`source: ${tray.id}`);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(200);
  }

  await browser.close();
  console.log(`source: ${sourceDir}`);
}

function readPng(path) {
  return readFile(path).then((buffer) => PNG.sync.read(buffer));
}

/**
 * A crop that contains only the sea gradient means the app never opened (or a
 * stale frame was captured). Real chrome has text/edges: >= 0.4 % pixels with a
 * local gradient and a stddev >= 9. The empty sea measures ~0.19 % / 6.6.
 */
function blankStats(png) {
  const { width, height, data } = png;
  let sum = 0;
  let sumSq = 0;
  let bright = 0;
  let lights = 0;
  let pixels = 0;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = (y * width + x) * 4;
      const r = data[index];
      const g = data[index + 1];
      const b = data[index + 2];
      const value = (r + g + b) / 3;
      sum += value;
      sumSq += value * value;
      if (value > 110) bright += 1;
      // Traffic lights: the only strongly red/amber pixels in the chrome.
      if (r > 150 && g < 170 && b < 170) lights += 1;
      pixels += 1;
    }
  }
  const mean = sum / pixels;
  const stddev = Math.sqrt(Math.max(0, sumSq / pixels - mean * mean));
  return {
    stddev,
    brightRatio: bright / pixels,
    lights,
    // The empty sea is a smooth teal gradient: no bright text and no lights.
    blank: bright / pixels < 0.002 && lights === 0,
  };
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
  const blanks = [];
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
    const cropped = crop(png, spec.rect);
    const stats = blankStats(PNG.sync.read(cropped));
    const target = join(aetherDir, `${name}.png`);
    if (stats.blank) {
      blanks.push(`${name} (${spec.file})`);
      await unlink(target).catch(() => {});
      console.warn(
        `blank capture: ${name} from ${spec.file} ` +
          `(stddev ${stats.stddev.toFixed(1)}, bright ${(stats.brightRatio * 100).toFixed(2)}%)`,
      );
      continue;
    }
    await writeFile(target, cropped);
  }
  console.log(`aether: ${aetherDir}`);
  if (blanks.length > 0) {
    console.error(
      `error: ${blanks.length} capture(s) contained no UI: ${blanks.join(", ")}`,
    );
    process.exitCode = 1;
  }
}

function imageSize(buffer) {
  const png = PNG.sync.read(buffer);
  return `${png.width}x${png.height}`;
}

/**
 * The design-state differ writes `state/diff.json`; surface its headline
 * numbers and accepted differences in the same report so pixel review and
 * property review live together.
 */
async function designState() {
  const file = join(outDir, "state", "diff.json");
  if (!(await exists(file))) return { stats: "", section: "" };
  const diff = JSON.parse(await readFile(file, "utf8"));
  const rows = [...diff.differences, ...diff.accepted].map((entry) => {
    const status = entry.kind === "value" ? (entry.reason ? "accepted" : "to fix") : entry.kind;
    return `<tr><td><code>${entry.key}</code></td><td><code>${entry.source ?? "—"}</code></td><td><code>${entry.aether ?? "—"}</code></td><td>${status}${entry.reason ? ` — ${entry.reason}` : ""}</td></tr>`;
  });
  const stats = `
    <div class="stat"><b>${diff.summary.matched}/${diff.summary.compared}</b><span>State matches</span></div>
    <div class="stat"><b>${diff.summary.failures}</b><span>State to fix</span></div>`;
  const section = `
  <section class="state">
    <header>
      <h3>Design state</h3>
      <span class="priority">computed values, not pixels</span>
    </header>
    <p class="notes">The demo's computed styles and the compositor's own <code>design-manifest</code> are diffed key by key. Numbers allow 0.75 px, colours 2/255 per channel. Accepted differences are pinned in <code>state/baseline.json</code> with a reason; anything else exits non-zero. Raw: <a href="state/diff.md">state/diff.md</a>.</p>
    <table>
      <thead><tr><th>key</th><th>source</th><th>aether</th><th>status</th></tr></thead>
      <tbody>${rows.join("")}</tbody>
    </table>
  </section>`;
  return { stats, section };
}

/**
 * Scene coverage from `state/scene-diff.json`: how many source text runs each
 * surface's Aether frame actually paints.
 */
async function sceneState() {
  const file = join(outDir, "state", "scene-diff.json");
  if (!(await exists(file))) return { stats: "", section: "", bySurface: new Map() };
  const diff = JSON.parse(await readFile(file, "utf8"));
  const bySurface = new Map(diff.surfaces.map((surface) => [surface.id, surface]));
  const worst = diff.surfaces
    .filter((surface) => surface.coverage !== null)
    .sort((left, right) => left.coverage - right.coverage)
    .slice(0, 12);
  const stats = `
    <div class="stat"><b>${Math.round(diff.summary.coverage * 100)}%</b><span>UI text coverage</span></div>`;
  const rows = worst.map((surface) => {
    const missing = surface.missing
      .slice(0, 4)
      .map((text) => `<code>${text.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</code>`)
      .join(", ");
    return `<tr><td><code>${surface.id}</code></td><td>${surface.matched}/${surface.source}</td><td>${Math.round(surface.coverage * 100)}%</td><td>${missing}</td></tr>`;
  });
  const section = `
  <section class="state">
    <header>
      <h3>UI scene</h3>
      <span class="priority">source DOM vs Aether render chunk</span>
    </header>
    <p class="notes">Every visible text run is matched between the demo's DOM and the labels the compositor actually painted. Numbers are excluded because they are live data (CPU %, IPs, dates). Missing runs are real content the frame does not draw. Raw: <a href="state/scene-diff.md">state/scene-diff.md</a>.</p>
    <table>
      <thead><tr><th>surface</th><th>matched</th><th>coverage</th><th>missing (first)</th></tr></thead>
      <tbody>${rows.join("")}</tbody>
    </table>
  </section>`;
  return { stats, section, bySurface };
}

/**
 * DESIGN.md contract diff: token/component paths that differ between the
 * source contract and the compositor's.
 */
async function designContract() {
  const file = join(outDir, "state", "design-md-diff.json");
  if (!(await exists(file))) return { stats: "", section: "" };
  const document = JSON.parse(await readFile(file, "utf8"));
  const rows = document.paths.map((entry) => {
    const accepted = document.accepted?.[entry.path];
    const open = document.open?.[entry.path];
    const note = accepted
      ? `accepted — ${accepted}`
      : open
        ? `open — ${open}`
        : "unaccounted";
    return `<tr><td><code>${entry.path}</code></td><td>${entry.state}</td><td>${note}</td></tr>`;
  });
  const open = document.paths.filter((entry) => document.open?.[entry.path]).length;
  const stats = `
    <div class="stat"><b>${document.unaccounted?.length ?? 0}</b><span>Contract drift</span></div>`;
  const section = `
  <section class="state">
    <header>
      <h3>Design contract</h3>
      <span class="priority">DESIGN.md · source vs Aether</span>
    </header>
    <p class="notes">Both sides publish a DESIGN.md token and component contract. Source is measured from the demo; Aether is generated from the compositor manifest. ${open} path(s) are open implementation debt; accepted paths carry a reason and bound. Raw: <a href="state/design-md-diff.md">state/design-md-diff.md</a> · <a href="../../design/source/DESIGN.md">source</a> · <a href="../../design/aether/DESIGN.md">Aether</a>.</p>
    <table>
      <thead><tr><th>path</th><th>state</th><th>note</th></tr></thead>
      <tbody>${rows.join("")}</tbody>
    </table>
  </section>`;
  return { stats, section };
}

/**
 * Pixel regression summary from `state/pixel-diff.json`.
 */
async function pixelState() {
  const file = join(outDir, "state", "pixel-diff.json");
  if (!(await exists(file))) return { stats: "", section: "" };
  const document = JSON.parse(await readFile(file, "utf8"));
  const measured = document.surfaces
    .filter((surface) => surface.ratio !== null)
    .sort((left, right) => right.ratio - left.ratio)
    .slice(0, 12);
  const stats = `
    <div class="stat"><b>${(document.summary.ratio * 100).toFixed(1)}%</b><span>Pixels differ</span></div>`;
  const rows = measured.map(
    (surface) =>
      `<tr><td><code>${surface.id}</code></td><td>${surface.width}×${surface.height}</td><td>${(surface.ratio * 100).toFixed(2)}%</td><td>${(surface.baseline * 100).toFixed(2)}%</td></tr>`,
  );
  const section = `
  <section class="state">
    <header>
      <h3>Painted pixels</h3>
      <span class="priority">pixelmatch · same-size crops</span>
    </header>
    <p class="notes">Actual pixels, not layout: each source/Aether crop is compared with pixelmatch and baselined. Overall ${(document.summary.ratio * 100).toFixed(2)}% of ${document.summary.totalPixels.toLocaleString()} pixels differ across ${document.summary.surfaces} same-size surfaces; ${document.summary.skipped} pairs were skipped because their crops differ in size. Raw: <a href="state/pixel-diff.md">state/pixel-diff.md</a>.</p>
    <table>
      <thead><tr><th>surface</th><th>size</th><th>diff</th><th>baseline</th></tr></thead>
      <tbody>${rows.join("")}</tbody>
    </table>
  </section>`;
  return { stats, section };
}

async function buildReport() {
  await mkdir(outDir, { recursive: true });
  const counts = {
    implemented: 0,
    partial: 0,
    missing: 0,
    "aether-only": 0,
    "out-of-scope": 0,
  };
  for (const surface of SURFACES) counts[surface.status] += 1;
  const state = await designState();
  const scene = await sceneState();
  const contract = await designContract();
  const pixels = await pixelState();

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
      : surface.aether
        ? `<figure class="empty"><div>Capture missing — rerun <code>bun run parity:apps</code></div><figcaption>invalid capture</figcaption></figure>`
        : `<figure class="empty"><div>Not implemented yet</div><figcaption>roadmap</figcaption></figure>`;
    const sceneMatch = surface.aether ? scene.bySurface.get(surface.aether) : null;
    const coveragePill = sceneMatch?.coverage != null
      ? `<span class="pill pill--coverage">text ${Math.round(sceneMatch.coverage * 100)}%</span>`
      : "";
    cards.push(`
      <section class="surface">
        <header>
          <h3>${surface.title}</h3>
          <span class="priority">${surface.priority}</span>
          ${coveragePill}
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
  .pill--out-of-scope { color:var(--mute); border-color:color-mix(in srgb,var(--mute) 40%,transparent); background:color-mix(in srgb,var(--mute) 12%,transparent); }
  .pill--coverage { color:var(--accent); border-color:color-mix(in srgb,var(--accent) 40%,transparent); background:color-mix(in srgb,var(--accent) 10%,transparent); font-variant-numeric:tabular-nums; }
  .pair { display:grid; grid-template-columns:repeat(auto-fit,minmax(340px,1fr)); gap:14px; align-items:start; }
  figure { margin:0; border:1px solid var(--line); border-radius:10px; overflow:hidden; background:#041c26; display:flex; flex-direction:column; }
  figure img { flex:1 1 auto; object-fit:contain; }
  figure img { display:block; width:auto; max-width:100%; height:auto; margin:0 auto; }
  figcaption { padding:5px 9px; font-size:10px; color:var(--mute); border-top:1px solid var(--line); font-variant-numeric:tabular-nums; }
  figure.empty div { display:grid; place-items:center; min-height:150px; color:var(--mute); font-size:12px; border-bottom:1px dashed var(--line); }
  .notes { margin:10px 0 0; color:var(--dim); font-size:12.5px; }
  .state { border:1px solid var(--line); border-radius:14px; background:rgba(7,39,51,.55); padding:14px 16px; margin:0 0 22px; }
  .state header { display:flex; align-items:center; gap:10px; margin-bottom:4px; }
  .state h3 { margin:0; font-size:16px; }
  .state table { width:100%; border-collapse:collapse; margin-top:12px; font-size:12px; }
  .state th { text-align:left; color:var(--mute); font-weight:600; padding:6px 8px; border-bottom:1px solid var(--line); }
  .state td { padding:6px 8px; border-bottom:1px solid rgba(127,230,255,.08); vertical-align:top; color:var(--dim); }
  .state td code { color:var(--text); }
  .state a { color:var(--accent); }
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
    <div class="stat"><b>${counts["out-of-scope"]}</b><span>Out of scope</span></div>
    <div class="stat"><b>${counts["aether-only"]}</b><span>Aether only</span></div>
    ${state.stats}
    ${scene.stats}
    ${contract.stats}
    ${pixels.stats}
  </div>
  ${state.section}
  ${scene.section}
  ${contract.section}
  ${pixels.section}
  ${cards.join("\n")}
  <footer>Generated by <code>scripts/screenshots/app-parity.mjs</code>. Path to 100%: <a href="PARITY-100.md"><code>PARITY-100.md</code></a> · Roadmap: <code>ROADMAP.md</code> · Visual spec: <code>VISUAL-SPEC.md</code> · Design state: <code>state/diff.md</code>.</footer>
</body>
</html>
`;
  await writeFile(join(outDir, "index.html"), html);

  const lines = [
    "# Water OS app parity · roadmap",
    "",
    `Source: \`${SOURCE.revision}\` (${SOURCE.origin})`,
    "Visual spec (source vs target, difference register): [`VISUAL-SPEC.md`](VISUAL-SPEC.md).",
    "Parity target and remaining work: [`PARITY-100.md`](PARITY-100.md).",
    "",
    `Aether: \`${AETHER_REVISION}\``,
    "",
    `Implemented **${counts.implemented}** · partial **${counts.partial}** · missing **${counts.missing}** · out of scope **${counts["out-of-scope"]}** · Aether-only **${counts["aether-only"]}**.`,
    "",
    "**Status key** — implemented: matches the source surface · partial: present but a different scope or layout · missing: no Aether surface · out of scope: intentionally not an Aether app · Aether only: no source counterpart.",
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
    "Data Cloud Compare and App Studio are **out of scope by decision** and excluded from the gap.",
    "",
    "1. **Explorer wiring** (P1): make Hidden, Everywhere, Filter and the navigation buttons functional.",
    "2. **Command overlay** (P2): align the palette with the demo's grouped command surface.",
    "3. **Keyboard shortcuts** (P2): group the cheatsheet by app like the demo.",
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
    closeServer = served.close;
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
closeServer?.();
// The capture only needs the server while Playwright is running. Exit
// explicitly so a lingering socket cannot keep the process alive.
process.exit(process.exitCode ?? 0);