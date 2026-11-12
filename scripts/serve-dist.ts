/**
 * Static server for the assembled site. Used by Playwright and by
 * `bun run preview` when you want to inspect docs + marketing together.
 *
 *   PORT=4173 bun run scripts/serve-dist.ts
 */
import { extname, join, normalize, resolve, sep } from "node:path";
import { stat } from "node:fs/promises";

const root = resolve(import.meta.dir, "..", "apps", "web", "dist");
const port = Number(process.env.PORT ?? 4173);

async function isFile(path: string): Promise<boolean> {
  try {
    const info = await stat(path);
    return info.isFile();
  } catch {
    return false;
  }
}

async function resolvePath(pathname: string): Promise<string | null> {
  const decoded = decodeURIComponent(pathname);
  const safe = normalize(decoded).replace(/^(\.\.[/\\])+/, "");
  const target = resolve(join(root, safe));
  if (target !== root && !target.startsWith(root + sep)) return null;

  if (decoded.endsWith("/")) {
    const index = join(target, "index.html");
    return (await isFile(index)) ? index : null;
  }

  if (await isFile(target)) return target;

  if (extname(target) === "") {
    const asDirectory = join(target, "index.html");
    if (await isFile(asDirectory)) return asDirectory;
    const asHtml = `${target}.html`;
    if (await isFile(asHtml)) return asHtml;
  }

  return null;
}

const server = Bun.serve({
  port,
  async fetch(request) {
    const url = new URL(request.url);
    const filePath = await resolvePath(url.pathname);

    if (filePath === null) {
      const notFound = join(root, "404.html");
      const body = (await isFile(notFound)) ? await Bun.file(notFound).text() : "Not found";
      return new Response(body, { status: 404, headers: { "content-type": "text/html" } });
    }

    const file = Bun.file(filePath);
    const cache = filePath.includes(`${sep}assets${sep}`)
      ? "public, max-age=31536000, immutable"
      : "no-cache";
    return new Response(file, { headers: { "cache-control": cache } });
  },
});

console.log(`serving ${root} at http://localhost:${server.port}/`);
