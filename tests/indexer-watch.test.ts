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
  const count = (): number => seen.filter((entry) => entry === path).length;

  // The platform watcher may not be attached immediately, so keep retrying the
  // write until one is observed. Each attempt waits longer than the debounce
  // window, which also proves several events collapse into one callback.
  const deadline = Date.now() + 5_000;
  while (count() === 0 && Date.now() < deadline) {
    await writeFile(path, "hello");
    await Bun.sleep(200);
  }
  expect(count()).toBe(1);

  stop();
  await writeFile(join(root, "after-stop.txt"), "ignored");
  await Bun.sleep(500);
  expect(seen.some((entry) => entry.endsWith("after-stop.txt"))).toBe(false);
  await rm(root, { recursive: true, force: true });
});
