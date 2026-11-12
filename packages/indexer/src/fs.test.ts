import { afterAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  DEFAULT_IGNORES,
  deriveFileDocument,
  extensionOf,
  isProbablyText,
  scanDirectory,
} from "../src/fs";
import { buildIndex } from "../src/search-index";

const NOW = Date.UTC(2026, 10, 12);
const roots: string[] = [];

async function fixture(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "aether-index-"));
  roots.push(root);
  await mkdir(join(root, "src"), { recursive: true });
  await mkdir(join(root, "node_modules", "pkg"), { recursive: true });
  await writeFile(
    join(root, "README.md"),
    "# Aether\n\nSingle-target workstation OS with a zero-copy architecture.\n",
  );
  await writeFile(join(root, "src", "main.rs"), "fn main() { println!(\"aether\"); }\n");
  await writeFile(join(root, "node_modules", "pkg", "index.js"), "module.exports = 1;\n");
  await writeFile(join(root, ".hidden.md"), "secret\n");
  await writeFile(join(root, "binary.bin"), new Uint8Array([0x00, 0x01, 0x02, 0x00]));
  return root;
}

afterAll(async () => {
  await Promise.all(roots.map((root) => rm(root, { recursive: true, force: true })));
});

describe("deriveFileDocument", () => {
  test("derives id, title and directory from a path", () => {
    const document = deriveFileDocument({
      path: "/work/aether/README.md",
      content: "hello",
      modifiedAt: NOW,
    });
    expect(document.id).toBe("file:/work/aether/README.md");
    expect(document.kind).toBe("file");
    expect(document.title).toBe("README.md");
    expect(document.subtitle).toBe("/work/aether");
    expect(document.tags).toEqual(["file", "md"]);
    expect(document.body).toBe("hello");
    expect(document.modifiedAt).toBe(NOW);
  });

  test("never indexes content for binary extensions", () => {
    const document = deriveFileDocument({ path: "/media/clip.mp4", content: "not really" });
    expect(document.body).toBeUndefined();
  });

  test("extensionOf is lowercase and dotless", () => {
    expect(extensionOf("A.TSX")).toBe("tsx");
    expect(extensionOf("Makefile")).toBe("");
  });

  test("isProbablyText sniffs NUL bytes", () => {
    expect(isProbablyText(new TextEncoder().encode("plain text"))).toBe(true);
    expect(isProbablyText(new Uint8Array([104, 105, 0, 0]))).toBe(false);
    expect(isProbablyText(new Uint8Array(0))).toBe(true);
  });
});

describe("scanDirectory (integration with the real filesystem)", () => {
  test("indexes text files, skips ignored dirs and hidden files", async () => {
    const root = await fixture();
    const documents = await scanDirectory(root);
    const titles = documents.map((document) => document.title).sort();
    expect(titles).toEqual(["README.md", "binary.bin", "main.rs"]);
    expect(DEFAULT_IGNORES.has("node_modules")).toBe(true);
  });

  test("attaches body text for text files only", async () => {
    const root = await fixture();
    const documents = await scanDirectory(root);
    const readme = documents.find((document) => document.title === "README.md");
    const binary = documents.find((document) => document.title === "binary.bin");
    expect(readme?.body).toContain("zero-copy");
    expect(binary?.body).toBeUndefined();
  });

  test("scanned output feeds the search index end to end", async () => {
    const root = await fixture();
    const index = buildIndex(await scanDirectory(root));
    expect(index.search("zero-copy", { now: NOW }).map((hit) => hit.title)).toEqual([
      "README.md",
    ]);
    expect(index.search("main.rs", { now: NOW }).map((hit) => hit.title)).toEqual(["main.rs"]);
  });

  test("honours maxFiles and maxDepth", async () => {
    const root = await fixture();
    const limited = await scanDirectory(root, { maxFiles: 1 });
    expect(limited.length).toBe(1);
    const shallow = await scanDirectory(root, { maxDepth: 0 });
    expect(shallow.map((document) => document.title)).not.toContain("main.rs");
  });

  test("stops when the abort signal fires", async () => {
    const root = await fixture();
    const controller = new AbortController();
    controller.abort();
    expect(await scanDirectory(root, { signal: controller.signal })).toEqual([]);
  });

  test("missing roots fail soft", async () => {
    expect(await scanDirectory("/definitely/not/a/real/path/aether")).toEqual([]);
  });
});
