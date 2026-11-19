#!/usr/bin/env node
/**
 * Generate DESIGN.md contracts for both sides and diff them.
 *
 *   node scripts/screenshots/design-docs.mjs
 *   node scripts/screenshots/design-docs.mjs --check   # fail on unaccepted drift
 *
 * The source document is the contract: tokens and component specs measured from
 * the demo. The Aether document is the same schema filled from the compositor's
 * design manifest, so a missing component or a changed token is visible as a
 * contract difference instead of being buried in prose.
 *
 * Outputs
 *   design/source/DESIGN.md
 *   design/aether/DESIGN.md
 *   reports/os-app-parity/state/design-md-diff.json
 *   reports/os-app-parity/state/design-md-diff.md
 *
 * `design/open.json` lists known-missing paths (debt, not accepted parity).
 * `design/accepted.json` lists deliberate differences with a reason and bound.
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const stateDir = join(root, "reports", "os-app-parity", "state");
const designDir = join(root, "design");
const check = process.argv.includes("--check");

const source = JSON.parse(await readFile(join(stateDir, "source.json"), "utf8"));
const aether = JSON.parse(await readFile(join(stateDir, "aether.json"), "utf8"));

const weight = (name) => (name === "bold" ? 700 : name === "medium" ? 550 : 400);
const px = (value) => (value === undefined || value === null ? null : `${value}px`);
/** Geometry is declared in whole pixels; the demo's computed boxes are fractional. */
const wholePx = (value) => (value === undefined || value === null ? null : `${Math.round(value)}px`);

/** Drop the alpha suffix when the colour is fully opaque. */
function colorHex(value) {
  if (!value) return null;
  const match = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(value);
  if (!match) return value;
  if (match[2] === undefined || match[2].toLowerCase() === "ff") {
    return `#${match[1].toLowerCase()}`;
  }
  return `#${match[1].toLowerCase()}${match[2].toLowerCase()}`;
}
const quote = (value) =>
  typeof value === "number" ? String(value) : `"${String(value).replace(/"/g, '\\"')}"`;

function get(side, key) {
  return (side === "source" ? source : aether).values[key];
}

/** Token sections filled from the measured state. */
function tokensFor(side) {
  const rawColors = {
    bg: "token.bg",
    surface: "token.surface",
    popover: "token.popover",
    "menubar-fill": "token.menubar-fill",
    text: "token.text",
    "text-2": "token.text-2",
    "text-3": "token.text-3",
    accent: "token.accent",
    line: "token.line",
    "line-strong": "token.line-strong",
    success: "token.green",
  };
  const colors = Object.fromEntries(
    Object.entries(rawColors).map(([name, key]) => [name, colorHex(get(side, key))]),
  );
  colors.primary = colors.accent;

  const type = {
    "menubar-app": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.app.font-px")),
      fontWeight: weight(get(side, "menubar.app.weight")),
      letterSpacing: px(get(side, "menubar.app.tracking")),
    },
    "menubar-menu": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.menu.font-px")),
      fontWeight: weight(get(side, "menubar.menu.weight")),
    },
    "menubar-sync": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.sync.font-px")),
      fontWeight: 400,
    },
    clock: {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.clock.font-px")),
      fontWeight: 400,
    },
    "clock-time": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.clock.font-px")),
      fontWeight: weight(get(side, "menubar.clock.time-weight")),
    },
    "resource-label": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.resources.label-px")),
      fontWeight: weight(get(side, "menubar.resources.label-weight")),
      letterSpacing: px(get(side, "menubar.resources.label-tracking")),
    },
    "resource-value": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menubar.resources.value-px")),
      fontWeight: weight(get(side, "menubar.resources.value-weight")),
    },
    "window-title": {
      fontFamily: "Inter",
      fontSize: px(get(side, "window.title-px")),
      fontWeight: weight(get(side, "window.title-weight")),
      letterSpacing: px(get(side, "window.title-tracking")),
    },
    "window-subtitle": {
      fontFamily: "Inter",
      fontSize: px(get(side, "window.subtitle-px")),
      fontWeight: 400,
    },
    "menu-row": {
      fontFamily: "Inter",
      fontSize: px(get(side, "menu.row-font-px")),
      fontWeight: weight(get(side, "menu.row-weight")),
    },
  };

  type["label-caps"] = {
    fontFamily: "Inter",
    fontSize: px(get(side, "type.13.px")),
    fontWeight: 650,
    letterSpacing: "0.82px",
  };

  const rounded = {
    xs: "5px",
    md: px(get(side, "menu.radius")) ?? "10px",
    lg: px(get(side, "window.radius")) ?? "14px",
    dock: px(get(side, "dock.radius")) ?? "17px",
    pill: "999px",
  };

  const spacing = {
    menu: px(get(side, "menu.padding")),
    dock: px(get(side, "dock.pad")),
    bar: px(get(side, "menubar.pad-x")),
  };

  return { colors, type, rounded, spacing };
}

/**
 * Components shared by both documents. Each value is a token reference or a
 * measured dimension. A component is omitted on a side when a required value
 * is missing, which surfaces as "removed" in the DESIGN.md diff.
 */
const COMPONENTS = [
  {
    name: "menubar",
    props: (side) => ({
      backgroundColor: "{colors.menubar-fill}",
      height: wholePx(get(side, "menubar.height")),
      padding: wholePx(get(side, "menubar.pad-x")),
    }),
  },
  {
    name: "menubar-app",
    props: (side) => ({
      typography: "{typography.menubar-app}",
      height: wholePx(get(side, "menubar.menu.item-height")),
      width: wholePx(get(side, "menubar.app.max-width")),
    }),
  },
  {
    name: "menubar-menu",
    props: (side) => ({
      typography: "{typography.menubar-menu}",
      textColor: "{colors.text-2}",
      height: wholePx(get(side, "menubar.menu.item-height")),
      padding: wholePx(get(side, "menubar.menu.pad-x")),
      rounded: "{rounded.xs}",
    }),
  },
  {
    name: "sync-pill",
    props: (side) => ({
      typography: "{typography.menubar-sync}",
      height: wholePx(get(side, "menubar.sync.height")),
      rounded: "{rounded.xs}",
    }),
  },
  {
    name: "resources",
    props: (side) => ({
      height: wholePx(get(side, "menubar.resources.height")),
      padding: wholePx(get(side, "menubar.resources.pad-x")),
    }),
  },
  {
    name: "resource-meter",
    props: (side) => ({
      backgroundColor: "{colors.accent}",
      width: wholePx(get(side, "menubar.resources.meter-w")),
      height: wholePx(get(side, "menubar.resources.meter-h")),
      rounded: "{rounded.pill}",
    }),
  },
  {
    name: "clock",
    props: (side) => ({
      typography: "{typography.clock}",
      height: wholePx(get(side, "menubar.clock.height")),
      padding: wholePx(get(side, "menubar.clock.pad-x")),
    }),
  },
  {
    name: "desktop",
    props: () => ({
      backgroundColor: "{colors.bg}",
      width: "1280px",
      height: "800px",
    }),
  },
  {
    name: "status-dot",
    props: () => ({
      backgroundColor: "{colors.success}",
      size: "6px",
      rounded: "{rounded.pill}",
    }),
  },
  {
    name: "window",
    props: (side) => ({
      backgroundColor: "{colors.surface}",
      rounded: "{rounded.lg}",
    }),
  },
  {
    name: "window-titlebar",
    props: (side) => ({
      height: wholePx(get(side, "window.titlebar-height")),
      typography: "{typography.window-title}",
      textColor: "{colors.text}",
    }),
  },
  {
    name: "titlebar-subtitle",
    props: () => ({
      typography: "{typography.window-subtitle}",
      textColor: "{colors.text-3}",
    }),
  },
  {
    name: "hairline",
    props: () => ({
      backgroundColor: "{colors.line}",
      width: "1248px",
      height: "1px",
    }),
  },
  {
    name: "divider",
    props: () => ({
      backgroundColor: "{colors.line-strong}",
      width: "1248px",
      height: "1px",
    }),
  },
  {
    name: "settings-profile",
    props: () => ({ width: "217px", height: "48px" }),
  },
  {
    name: "settings-nav-group-label",
    props: () => ({ typography: "{typography.label-caps}" }),
  },
  {
    name: "settings-nav-item",
    props: () => ({ height: "30px", rounded: "{rounded.md}" }),
  },
  {
    name: "settings-footer",
    props: () => ({ width: "237px", height: "35px" }),
  },
  {
    name: "traffic-light",
    props: (side) => ({
      size: wholePx(get(side, "window.light-size")),
      rounded: "{rounded.pill}",
    }),
  },
  {
    name: "menu-popover",
    props: (side) => ({
      backgroundColor: "{colors.popover}",
      rounded: "{rounded.md}",
      padding: wholePx(get(side, "menu.padding")),
      width: wholePx(get(side, "menu.min-width")),
    }),
  },
  {
    name: "menu-row",
    props: (side) => ({
      typography: "{typography.menu-row}",
      height: wholePx(get(side, "menu.row-height")),
    }),
  },
  {
    name: "dock",
    props: (side) => ({
      backgroundColor: "{colors.surface}",
      height: wholePx(get(side, "dock.height")),
      padding: wholePx(get(side, "dock.pad")),
      rounded: "{rounded.dock}",
    }),
  },
  {
    name: "dock-item",
    props: (side) => ({ size: wholePx(get(side, "dock.item")) }),
  },
  {
    name: "dock-tile",
    props: (side) => ({
      size: wholePx(get(side, "dock.tile")),
      rounded: "{rounded.md}",
    }),
  },
  ...["network", "devices", "battery", "sound", "calendar"].map((tray) => ({
    name: `tray-${tray}`,
    props: (side) => ({
      backgroundColor: "{colors.popover}",
      width: wholePx(get(side, `tray.${tray}.width`)),
      height: wholePx(get(side, `tray.${tray}.height`)),
      rounded: "{rounded.lg}",
    }),
  })),
];

/**
 * Settings chrome the demo draws around every pane. The Aether document omits
 * these until the compositor paints them; they are tracked as open debt.
 */
const SOURCE_ONLY_COMPONENTS = [];

function componentsFor(side) {
  const out = [];
  for (const component of COMPONENTS) {
    const props = component.props(side);
    if (Object.values(props).some((value) => value === null || value === undefined)) continue;
    out.push([component.name, props]);
  }
  if (side === "source") {
    for (const component of SOURCE_ONLY_COMPONENTS) out.push([component.name, component.props]);
  }
  return out;
}

function yamlMap(entries, indent = 0) {
  const pad = " ".repeat(indent);
  const lines = [];
  for (const [key, value] of entries) {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      lines.push(`${pad}${key}:`);
      lines.push(yamlMap(Object.entries(value), indent + 2));
    } else {
      lines.push(`${pad}${key}: ${quote(value)}`);
    }
  }
  return lines.join("\n");
}

function documentFor(side) {
  const { colors, type, rounded, spacing } = tokensFor(side);
  const title = side === "source" ? "Water OS" : "Aether OS";
  const components = componentsFor(side);
  return `---
version: alpha
name: ${title}
description: Chrome contract generated from the ${side === "source" ? "demo's computed styles" : "compositor's design manifest"}.
colors:
${yamlMap(Object.entries(colors), 2)}
typography:
${yamlMap(Object.entries(type), 2)}
rounded:
${yamlMap(Object.entries(rounded), 2)}
spacing:
${yamlMap(Object.entries(spacing), 2)}
---

## Overview

${side === "source" ? "The Water OS concept: a tiled glass desktop over a flowing sea, with fractional type and a 2.5 px tide rim." : "Aether's implementation of the same desktop. Tokens come from the compositor's design manifest, so this file describes what the renderer actually paints."} The source document is the contract; differences are reviewed as token paths, not prose.

## Colors

Surfaces, text roles, accent and hairlines measured from the running interface.

## Typography

Fractional OS sizes (the demo scales design tokens by 1.14) with the menu-bar, window and popover roles.

## Layout

Menu bar and window chrome share a 26 px item rhythm; the dock sits 8 px above the bottom edge.

## Elevation & Depth

Windows use a layered drop shadow and an inset top highlight; the focused window adds the tide rim.

## Shapes

Radii run from 5 px controls to 14 px windows and 17 px dock.

## Motion & Micro-interactions

The tide rim cycles every 8 seconds (linear, no easing). Interface transitions stay under 200 ms and ease out.

## Components

Component values are the contract; variants are separate entries when they exist. Missing components in the Aether document are implementation gaps.

\`\`\`yaml
components:
${components.map(([name, props]) => yamlMap([[name, props]], 2)).join("\n")}
\`\`\`

## Do's and Don'ts

- Do reference tokens instead of literal colours.
- Do keep item heights on the 26/30/32 px rhythm.
- Don't add a surface without declaring its component here.
`;
}

async function writeDesign() {
  await mkdir(join(designDir, "source"), { recursive: true });
  await mkdir(join(designDir, "aether"), { recursive: true });
  await writeFile(join(designDir, "source", "DESIGN.md"), documentFor("source"));
  await writeFile(join(designDir, "aether", "DESIGN.md"), documentFor("aether"));
}

function cli(args) {
  try {
    return { ok: true, output: execFileSync("npx", ["designmd", ...args], { encoding: "utf8" }) };
  } catch (error) {
    return { ok: false, output: `${error.stdout ?? ""}${error.stderr ?? ""}` };
  }
}

async function loadList(name) {
  try {
    return JSON.parse(await readFile(join(designDir, name), "utf8"));
  } catch {
    return {};
  }
}

function flattenDiff(diff) {
  const paths = [];
  for (const [category, groups] of Object.entries(diff.tokens ?? {})) {
    for (const [state, names] of Object.entries(groups)) {
      for (const name of names) paths.push({ path: `${category}.${name}`, state });
    }
  }
  return paths;
}

async function main() {
  await writeDesign();

  const lint = {};
  for (const side of ["source", "aether"]) {
    const result = cli(["lint", join(designDir, side, "DESIGN.md")]);
    lint[side] = JSON.parse(result.output || "{}");
    if (!result.ok) {
      console.error(`lint failed for ${side}`);
      console.error(result.output);
      if (check) process.exitCode = 1;
    }
  }

  const diffResult = cli([
    "diff",
    join(designDir, "source", "DESIGN.md"),
    join(designDir, "aether", "DESIGN.md"),
  ]);
  const diff = JSON.parse(diffResult.output || "{}");
  const paths = flattenDiff(diff);

  const open = await loadList("open.json");
  const accepted = await loadList("accepted.json");
  const unaccounted = paths.filter(
    (entry) => entry.state !== "modified" ? !open[entry.path] && !accepted[entry.path] : !accepted[entry.path],
  );

  await mkdir(stateDir, { recursive: true });
  await writeFile(
    join(stateDir, "design-md-diff.json"),
    `${JSON.stringify({ lint, diff, paths, unaccounted, open, accepted }, null, 2)}\n`,
  );

  const lines = [
    "# DESIGN.md contract diff",
    "",
    `Source: \`design/source/DESIGN.md\` · Aether: \`design/aether/DESIGN.md\`.`,
    "",
    `| path | state | note |`,
    `| --- | --- | --- |`,
    ...paths.map(
      (entry) =>
        `| \`${entry.path}\` | ${entry.state} | ${accepted[entry.path] ? `accepted: ${accepted[entry.path]}` : open[entry.path] ? `open: ${open[entry.path]}` : "**unaccounted**"} |`,
    ),
    "",
    `Lint: source ${lint.source?.summary?.errors ?? "?"} errors / ${lint.source?.summary?.warnings ?? "?"} warnings · ` +
      `Aether ${lint.aether?.summary?.errors ?? "?"} errors / ${lint.aether?.summary?.warnings ?? "?"} warnings.`,
    "",
  ];
  await writeFile(join(stateDir, "design-md-diff.md"), lines.join("\n"));

  console.log(
    `design.md: ${paths.length} token changes · ${unaccounted.length} unaccounted · ` +
      `lint ${lint.source?.summary?.errors ?? "?"}/${lint.aether?.summary?.errors ?? "?"} errors`,
  );
  for (const entry of unaccounted) {
    console.error(`unaccounted ${entry.path} (${entry.state})`);
  }
  if (check && unaccounted.length > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(`error: ${error.stack ?? error.message}`);
  process.exit(1);
});