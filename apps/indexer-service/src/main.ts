import { watch } from "node:fs";
import { unlink } from "node:fs/promises";
import { delimiter, join, relative } from "node:path";
import {
  DEFAULT_IGNORES,
  readFileDocument,
  scanDirectory,
} from "../../../packages/indexer/src/fs";
import { buildIndex, handleRequest } from "./service";

const roots = (
  process.env.AETHER_INDEX_ROOTS ??
  process.env.HOME ??
  ""
)
  .split(delimiter)
  .filter((root) => root.length > 0);
const limit = Number.parseInt(process.env.AETHER_INDEX_LIMIT ?? "20000", 10);
const depth = Number.parseInt(process.env.AETHER_INDEX_DEPTH ?? "12", 10);
const runtime = process.env.XDG_RUNTIME_DIR ?? "/tmp";
const socketPath =
  process.env.AETHER_INDEX_SOCKET ?? `${runtime}/aether-index.sock`;

const documents = (
  await Promise.all(
    roots.map((root) =>
      scanDirectory(root, {
        maxFiles: Number.isFinite(limit) ? limit : 20000,
        maxDepth: Number.isFinite(depth) ? depth : 12,
      }),
    ),
  )
).flat();
const index = buildIndex(documents);

try {
  await unlink(socketPath);
} catch {
  // No stale socket.
}

const buffers = new WeakMap<object, string>();
const server = Bun.listen({
  unix: socketPath,
  socket: {
    data(socket, chunk) {
      const previous = buffers.get(socket) ?? "";
      const lines = (previous + Buffer.from(chunk).toString("utf8")).split("\n");
      buffers.set(socket, lines.pop() ?? "");
      for (const line of lines) {
        if (line.trim().length > 0) {
          socket.write(`${handleRequest(index, line)}\n`);
        }
      }
    },
    open() {},
    close(socket) {
      buffers.delete(socket);
    },
    error(_socket, error) {
      console.error("aether-index: socket error", error);
    },
  },
});

console.log(
  `aether-index: ready at ${socketPath} (${documents.length} documents, ${index.size} indexed)`,
);

const pending = new Map<string, ReturnType<typeof setTimeout>>();
const watchers: ReturnType<typeof watch>[] = [];

function ignored(path: string, root: string): boolean {
  const parts = relative(root, path).split("/");
  return parts.some(
    (part) => part.length === 0 || part.startsWith(".") || DEFAULT_IGNORES.has(part),
  );
}

async function updatePath(path: string, root: string): Promise<void> {
  if (ignored(path, root)) return;
  const document = await readFileDocument(path);
  if (document === null) {
    if (index.remove(`file:${path}`)) {
      console.log(`aether-index: removed ${path}`);
    }
    return;
  }
  index.upsert(document);
  console.log(`aether-index: indexed ${path}`);
}

function scheduleUpdate(path: string, root: string): void {
  const previous = pending.get(path);
  if (previous !== undefined) clearTimeout(previous);
  pending.set(
    path,
    setTimeout(() => {
      pending.delete(path);
      void updatePath(path, root).catch((error) => {
        console.error("aether-index: update failed", error);
      });
    }, 120),
  );
}

if (process.env.AETHER_INDEX_WATCH !== "0") {
  for (const root of roots) {
    try {
      const watcher = watch(root, { recursive: true }, (_event, filename) => {
        if (filename === null) return;
        scheduleUpdate(join(root, filename.toString()), root);
      });
      watchers.push(watcher);
    } catch (error) {
      console.error(`aether-index: cannot watch ${root}`, error);
    }
  }
}

const shutdown = (): void => {
  for (const watcher of watchers) watcher.close();
  for (const timer of pending.values()) clearTimeout(timer);
  server.stop();
  void unlink(socketPath).catch(() => undefined);
  process.exit(0);
};
process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
