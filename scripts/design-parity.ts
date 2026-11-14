/**
 * Fails when compositor chrome constants drift from the design system.
 *
 *   bun run test:design
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const tokens = readFileSync(join(root, "packages/design-system/tokens.css"), "utf8");
const design = readFileSync(
  join(root, "os/compositor/crates/aether-shell/src/design.rs"),
  "utf8",
);
const explorerCss = readFileSync(join(root, "packages/ui/src/ui.css"), "utf8");

function token(name: string): number {
  const match = tokens.match(new RegExp(`--ds-${name}:\\s*#([0-9a-fA-F]{6})`));
  if (match === null) throw new Error(`missing token --ds-${name}`);
  return Number.parseInt(match[1], 16);
}

function constant(name: string): number {
  const match = design.match(new RegExp(`pub const ${name}: u32 = 0x([0-9A-Fa-f]{6});`));
  if (match === null) throw new Error(`missing design constant ${name}`);
  return Number.parseInt(match[1], 16);
}

const pairs: Array<[string, string]> = [
  ["bg", "BG"],
  ["surface-solid", "PANEL"],
  ["popover", "HEADER"],
  ["accent", "ACCENT"],
  ["text", "TEXT"],
  ["text-2", "TEXT_DIM"],
];

let failed = false;
for (const [tokenName, constantName] of pairs) {
  const web = token(tokenName);
  const rust = constant(constantName);
  if (web !== rust) {
    console.error(
      `mismatch --ds-${tokenName} #${web.toString(16)} != ${constantName} #${rust.toString(16)}`,
    );
    failed = true;
  }
}

const cssRow = explorerCss.match(/\.aether-pane__row\s*\{[^}]*height:\s*(\d+)px/);
const rustRow = design.match(/EXPLORER_ROW_HEIGHT: u32 = (\d+);/);
if (cssRow === null || rustRow === null || Number(cssRow[1]) !== Number(rustRow[1])) {
  console.error(`row height mismatch: css ${cssRow?.[1]} vs rust ${rustRow?.[1]}`);
  failed = true;
}

if (failed) process.exit(1);
console.log(
  `design parity ok: ${pairs.length} colors, ${rustRow?.[1]}px explorer rows`,
);
