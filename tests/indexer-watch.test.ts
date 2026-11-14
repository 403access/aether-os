import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { watchTree } from "../packages/indexer/src/watch";

test("watchTree debounces changed paths and stops cleanly", async () => {
  const root = await mkdtemp(join(tmpdir(), "aether-watch-"));
  const seen: string[] = [];
  const stop = watchTree(root, {
    debounceMs: 80,
    onPath(path) {
      seen.push(path);
    },
  });

  const path = join(root, "note.txt");
  await writeFile(path, "hello");
  await Bun.sleep(400);
  expect(seen.filter((entry) => entry === path).length).toBe(1);

  stop();
  await writeFile(join(root, "after-stop.txt"), "ignored");
  await Bun.sleep(400);
  expect(seen.some((entry) => entry.endsWith("after-stop.txt"))).toBe(false);
  await rm(root, { recursive: true, force: true });
});
