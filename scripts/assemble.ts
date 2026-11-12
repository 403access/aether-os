/**
 * Assemble one deployable tree:
 *   dist/               marketing site (Vite)
 *   dist/docs/          Docusaurus build
 *   dist/design-system/ Water component gallery
 *
 * This is the artifact GitHub Pages publishes.
 */
import { copyFile, cp, mkdir, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const webDist = join(root, "apps", "web", "dist");
const docsBuild = join(root, "apps", "docs", "build");
const designSystem = join(root, "packages", "design-system");
const basePath = process.env.BASE_PATH ?? "/";

function log(message: string): void {
  console.log(`[assemble] ${message}`);
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function copyGallery(): Promise<void> {
  const target = join(webDist, "design-system");
  await rm(target, { recursive: true, force: true });
  await mkdir(target, { recursive: true });

  const entries = await readdir(designSystem, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile()) continue;
    if (entry.name === "package.json" || entry.name === "README.md") continue;
    await copyFile(join(designSystem, entry.name), join(target, entry.name));
  }

  // The gallery is vendored as-is except for two path assumptions that differ
  // when it is served from the assembled site root.
  const indexPath = join(target, "index.html");
  let html = await readFile(indexPath, "utf8");
  html = html.replaceAll("../assets/favicon.svg", "../favicon.svg");
  await writeFile(indexPath, html);
  log("copied /design-system");
}

async function write404(): Promise<void> {
  const target = join(webDist, "404.html");
  const html = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta http-equiv="refresh" content="0; url=${basePath}" />
  <title>Aether OS</title>
</head>
<body>
  <p>Redirecting to <a href="${basePath}">Aether OS</a>…</p>
</body>
</html>
`;
  await writeFile(target, html);
  log("wrote 404.html");
}

async function main(): Promise<void> {
  if (!(await exists(join(webDist, "index.html")))) {
    throw new Error("apps/web/dist/index.html is missing; run the web build first");
  }
  if (!(await exists(join(docsBuild, "index.html")))) {
    throw new Error("apps/docs/build/index.html is missing; run the docs build first");
  }

  const docsTarget = join(webDist, "docs");
  await rm(docsTarget, { recursive: true, force: true });
  await cp(docsBuild, docsTarget, { recursive: true });
  log("copied /docs");

  await copyGallery();
  await write404();

  const required = [
    join(webDist, "index.html"),
    join(webDist, "docs", "index.html"),
    join(webDist, "design-system", "index.html"),
  ];
  for (const file of required) {
    if (!(await exists(file))) throw new Error(`missing assembled file: ${file}`);
  }

  log(`ready: ${webDist}`);
  log(`docs:  ${basePath}docs/`);
  log(`gallery: ${basePath}design-system/`);
}

await main();
