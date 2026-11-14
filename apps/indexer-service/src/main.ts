import { unlink } from "node:fs/promises";
import { delimiter } from "node:path";
import { scanDirectory } from "../../../packages/indexer/src/fs";
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

const shutdown = (): void => {
  server.stop();
  void unlink(socketPath).catch(() => undefined);
  process.exit(0);
};
process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
