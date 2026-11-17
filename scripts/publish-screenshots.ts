/**
 * Publish the generated screenshots into the site and docs trees and write a
 * manifest for the capture run.
 *
 *   bun run scripts/publish-screenshots.ts
 *   bun run scripts/publish-screenshots.ts --shell-only
 *   bun run scripts/publish-screenshots.ts --web-only --strict
 *
 * Captures stay in artifacts/screenshots (gitignored). Only the curated,
 * referenced images are copied into apps/web/public/screenshots and
 * apps/docs/static/img.
 */
import { chmod, copyFile, mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const shellDir = join(root, "artifacts", "screenshots", "shell");
const webDir = join(root, "artifacts", "screenshots", "web");
const manifestPath = join(root, "artifacts", "screenshots", "manifest.json");
const contactSheetPath = join(root, "artifacts", "screenshots", "index.html");

const args = new Set(process.argv.slice(2));
const shellOnly = args.has("--shell-only");
const webOnly = args.has("--web-only");
const strict = args.has("--strict");
const both = !shellOnly && !webOnly;

if (shellOnly && webOnly) {
  console.error("error: choose --shell-only or --web-only, not both");
  process.exit(1);
}

type Copy = { source: string; target: string; fallback?: string };

const gallery: Copy[] = [
  { source: "shell/palette-default.png", target: "apps/web/public/screenshots/palette.png" },
  { source: "shell/explorer.png", target: "apps/web/public/screenshots/explorer.png" },
  { source: "shell/chat.png", target: "apps/web/public/screenshots/chat.png" },
  { source: "shell/audio.png", target: "apps/web/public/screenshots/audio.png" },
  { source: "shell/themes.png", target: "apps/web/public/screenshots/themes.png" },
  { source: "shell/task-manager.png", target: "apps/web/public/screenshots/task-manager.png" },
  { source: "shell/shortcuts.png", target: "apps/web/public/screenshots/shortcuts.png" },
  { source: "web/remote-window.png", target: "apps/web/public/screenshots/remote.png" },
];

const docsImages: Copy[] = [
  { source: "shell/palette-default.png", target: "apps/docs/static/img/smithay-palette.png" },
  { source: "shell/palette-window.png", target: "apps/docs/static/img/smithay-palette-window.png" },
  { source: "shell/palette-window.png", target: "apps/docs/static/img/smithay-overlay.png" },
  { source: "shell/explorer.png", target: "apps/docs/static/img/smithay-explorer.png" },
  { source: "shell/explorer-filtered.png", target: "apps/docs/static/img/smithay-explorer-filtered.png" },
  { source: "shell/chat.png", target: "apps/docs/static/img/smithay-chat.png" },
  { source: "shell/audio.png", target: "apps/docs/static/img/smithay-audio.png" },
  { source: "shell/audio-preset.png", target: "apps/docs/static/img/smithay-audio-preset.png" },
  { source: "shell/themes.png", target: "apps/docs/static/img/smithay-themes.png" },
  { source: "shell/themes-tone.png", target: "apps/docs/static/img/smithay-themes-tone.png" },
  { source: "shell/shortcuts.png", target: "apps/docs/static/img/smithay-shortcuts.png" },
  { source: "shell/topbar.png", target: "apps/docs/static/img/smithay-topbar.png" },
  { source: "shell/topbar-terminal.png", fallback: "shell/topbar.png", target: "apps/docs/static/img/smithay-terminal.png" },
  { source: "shell/client-window.png", target: "apps/docs/static/img/smithay-client.png" },
  { source: "shell/client-window.png", target: "apps/docs/static/img/smithay-status.png" },
  { source: "shell/window-move.png", target: "apps/docs/static/img/smithay-window-move.png" },
  { source: "shell/window-resize.png", target: "apps/docs/static/img/smithay-window-resize.png" },
  { source: "shell/window-snap.png", target: "apps/docs/static/img/smithay-window-snap.png" },
  { source: "shell/window-maximized.png", target: "apps/docs/static/img/smithay-window-maximized.png" },
  { source: "shell/fullscreen.png", target: "apps/docs/static/img/smithay-fullscreen.png" },
  { source: "shell/workspace-picker.png", target: "apps/docs/static/img/smithay-workspace-picker.png" },
  { source: "shell/software-cursor.png", target: "apps/docs/static/img/smithay-software-cursor.png" },
  { source: "shell/partial-damage.png", target: "apps/docs/static/img/smithay-partial-damage.png" },
  { source: "shell/clipboard-paste.png", target: "apps/docs/static/img/smithay-clipboard.png" },
  { source: "shell/clipboard-primary.png", target: "apps/docs/static/img/smithay-clipboard-primary.png" },
  { source: "shell/clipboard-persist.png", target: "apps/docs/static/img/smithay-clipboard-persist.png" },
  { source: "shell/provider-refresh.png", target: "apps/docs/static/img/smithay-provider-refresh.png" },
  { source: "shell/provider-new-terminal.png", target: "apps/docs/static/img/smithay-new-terminal.png" },
  { source: "shell/provider-terminal-open.png", target: "apps/docs/static/img/smithay-new-terminal-open.png" },
  { source: "shell/provider-desktop.png", target: "apps/docs/static/img/smithay-desktop-app.png" },
  { source: "shell/provider-desktop-open.png", target: "apps/docs/static/img/smithay-desktop-app-open.png" },
  { source: "shell/provider-file.png", target: "apps/docs/static/img/smithay-file-provider.png" },
  { source: "shell/provider-action.png", target: "apps/docs/static/img/smithay-desktop-action.png" },
  { source: "shell/provider-agent.png", target: "apps/docs/static/img/smithay-agent-launcher.png" },
  { source: "shell/provider-agent-ipc.png", target: "apps/docs/static/img/smithay-agent-ipc.png" },
  { source: "shell/provider-index.png", target: "apps/docs/static/img/smithay-index-service.png" },
  { source: "shell/provider-index-live.png", target: "apps/docs/static/img/smithay-index-live.png" },
  { source: "shell/remote.png", target: "apps/docs/static/img/smithay-remote.png" },
  { source: "shell/task-manager.png", target: "apps/docs/static/img/task-manager.png" },
  { source: "shell/task-manager-users.png", target: "apps/docs/static/img/task-manager-users.png" },
  { source: "shell/task-manager-resources.png", target: "apps/docs/static/img/task-manager-resources.png" },
  { source: "web/marketing-remote-live.png", target: "apps/docs/static/img/remote-viewer.png" },
];

function pngSize(buffer: Buffer): { width: number; height: number } | null {
  if (buffer.length < 24) return null;
  if (buffer[0] !== 0x89 || buffer.toString("ascii", 1, 4) !== "PNG") return null;
  return { width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

async function collect(dir: string): Promise<string[]> {
  if (!existsSync(dir)) return [];
  const entries = await readdir(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith(".png")) continue;
    files.push(join(dir, entry.name));
  }
  return files.sort();
}

async function publish(copies: Copy[], label: string): Promise<number> {
  let count = 0;
  for (const copy of copies) {
    let source = join(root, "artifacts", "screenshots", copy.source);
    let used = copy.source;
    if (!existsSync(source) && copy.fallback !== undefined) {
      const fallback = join(root, "artifacts", "screenshots", copy.fallback);
      if (existsSync(fallback)) {
        source = fallback;
        used = copy.fallback;
      }
    }
    if (!existsSync(source)) {
      const message = `skip ${label}: ${copy.source} is missing`;
      if (strict) {
        throw new Error(message);
      }
      console.log(message);
      continue;
    }
    const target = join(root, copy.target);
    await mkdir(dirname(target), { recursive: true });
    await copyFile(source, target);
    await chmod(target, 0o644);
    const info = await stat(target);
    console.log(`  ${used} -> ${copy.target} (${info.size} B)`);
    count += 1;
  }
  return count;
}

function hash(buffer: Buffer): string {
  return createHash("sha256").update(buffer).digest("hex").slice(0, 16);
}

async function toRecord(file: string, kind: "shell" | "web") {
  const info = await stat(file);
  const buffer = Buffer.from(await Bun.file(file).arrayBuffer());
  const size = pngSize(buffer);
  return {
    kind,
    file: relative(join(root, "artifacts", "screenshots"), file),
    width: size?.width ?? null,
    height: size?.height ?? null,
    bytes: info.size,
    sha256: hash(buffer),
  };
}

function escape(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

async function writeContactSheet(shell: string[], web: string[]): Promise<void> {
  const sections: Array<[string, string[]]> = [
    ["Compositor surfaces", shell],
    ["Browser surfaces", web],
  ];
  const body = sections
    .map(([title, files]) => {
      if (files.length === 0) return "";
      const cards = files
        .map((file) => {
          const src = relative(dirname(contactSheetPath), file).split("\\").join("/");
          const name = relative(join(root, "artifacts", "screenshots"), file);
          return `<figure><a href="${src}"><img loading="lazy" src="${src}" alt="${escape(name)}"></a><figcaption>${escape(name)}</figcaption></figure>`;
        })
        .join("\n");
      return `<section><h2>${escape(title)} <span>${files.length}</span></h2><div class="grid">${cards}</div></section>`;
    })
    .join("\n");

  const html = `<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Aether screenshot index</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  :root { color-scheme: dark; }
  body { margin: 0; padding: 2rem; background: #04161d; color: #dff7ff; font: 14px/1.5 ui-sans-serif, system-ui, sans-serif; }
  h1 { margin-top: 0; font-weight: 600; letter-spacing: -0.02em; }
  h2 { display: flex; gap: 0.6rem; align-items: baseline; margin: 2rem 0 1rem; font-size: 1rem; text-transform: uppercase; letter-spacing: 0.12em; color: #8cc7d8; }
  h2 span { color: #38d6ff; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1rem; }
  figure { margin: 0; border: 1px solid #0d3d4d; border-radius: 10px; overflow: hidden; background: #06232c; }
  img { display: block; width: 100%; height: auto; }
  figcaption { padding: 0.5rem 0.7rem; color: #9fd7e5; font-family: ui-monospace, monospace; font-size: 0.75rem; }
  a { color: inherit; text-decoration: none; }
</style>
<h1>Aether screenshot index</h1>
<p>Generated ${new Date().toISOString()}. Open a capture for the full-size PNG.</p>
${body}
</html>`;
  await writeFile(contactSheetPath, html);
}

const shell = await collect(shellDir);
const web = await collect(webDir);

if (shell.length === 0 && web.length === 0) {
  console.error("error: no captures found; run a capture script first");
  process.exit(1);
}

const copies: Copy[] = [];
if (both || (!webOnly && shell.length > 0)) copies.push(...gallery.filter((c) => c.source.startsWith("shell/")));
if (both || (!shellOnly && web.length > 0)) copies.push(...gallery.filter((c) => c.source.startsWith("web/")));
if (both || (!webOnly && shell.length > 0)) copies.push(...docsImages.filter((c) => c.source.startsWith("shell/")));
if (both || (!shellOnly && web.length > 0)) copies.push(...docsImages.filter((c) => c.source.startsWith("web/")));

console.log("publishing screenshots");
const published = await publish(copies, "image");

const records = [
  ...(await Promise.all(shell.map((file) => toRecord(file, "shell")))),
  ...(await Promise.all(web.map((file) => toRecord(file, "web")))),
];

const manifest = {
  generated_at: new Date().toISOString(),
  git: {
    commit: (await Bun.$`git rev-parse HEAD`.text()).trim(),
    dirty: (await Bun.$`git status --porcelain`.text()).trim().length > 0,
  },
  directories: {
    shell: relative(root, shellDir),
    web: relative(root, webDir),
  },
  published,
  captures: records,
};
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
await writeContactSheet(shell, web);

console.log(`\nwrote ${relative(root, manifestPath)}`);
console.log(`wrote ${relative(root, contactSheetPath)}`);
console.log(`published ${published} image(s)`);
console.log("\nnext: bun run build   # fold the new images into apps/web/dist");