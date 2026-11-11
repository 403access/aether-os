import { describe, expect, test } from "bun:test";
import { parseMarkdownLite } from "../src/markdown";

describe("parseMarkdownLite", () => {
  test("plain text is one segment", () => {
    expect(parseMarkdownLite("hello world")).toEqual([{ type: "text", text: "hello world" }]);
  });

  test("parses a fenced code block with a language", () => {
    const segments = parseMarkdownLite("before\n```ts\nconst x = 1;\n```\nafter");
    expect(segments).toEqual([
      { type: "text", text: "before\n" },
      { type: "code", text: "const x = 1;", language: "ts" },
      { type: "text", text: "\nafter" },
    ]);
  });

  test("an unclosed fence renders as code while streaming", () => {
    const segments = parseMarkdownLite("look:\n```rust\nfn main() {}");
    expect(segments[1]).toEqual({ type: "code", text: "fn main() {}", language: "rust" });
  });

  test("parses inline code, bold, italic and links", () => {
    const segments = parseMarkdownLite("Use `cargo` and **PGO**, see [docs](https://x.dev).");
    expect(segments).toContainEqual({ type: "inlineCode", text: "cargo" });
    expect(segments).toContainEqual({ type: "bold", text: "PGO" });
    expect(segments).toContainEqual({ type: "link", text: "docs", href: "https://x.dev" });
  });

  test("empty input yields no segments", () => {
    expect(parseMarkdownLite("")).toEqual([]);
  });
});
