import { unlink } from "node:fs/promises";
import { delimiter, relative } from "node:path";
import {
  DEFAULT_IGNORES,
  readFileDocument,
  scanDirectory,
} from "../../../packages/indexer/src/fs";
import { watchTree } from "../../../packages/indexer/src/watch";
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

const watchers: Array<() => void> = [];

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

if (process.env.AETHER_INDEX_WATCH !== "0") {
  for (const root of roots) {
    watchers.push(
      watchTree(root, {
        onPath(path) {
          void updatePath(path, root).catch((error) => {
            console.error("aether-index: update failed", error);
          });
        },
        onError(error) {
          console.error(`aether-index: cannot watch ${root}`, error);
        },
      }),
    );
  }
}

const shutdown = (): void => {
  for (const stop of watchers) stop();
  server.stop();
  void unlink(socketPath).catch(() => undefined);
  process.exit(0);
};
process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
