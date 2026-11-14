import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { subsequenceScore } from "@aether/indexer";
import { parsePaletteQuery } from "./engine";

interface Fixture {
  modes: Array<{ input: string; mode: string; term: string }>;
  ranks: Array<{ term: string; title: string; rank: number | null }>;
}

const fixture: Fixture = JSON.parse(
  readFileSync(join(import.meta.dir, "../fixtures/parity.json"), "utf8"),
);

function rank(term: string, title: string): number | null {
  const query = term.toLowerCase();
  const folded = title.toLowerCase();
  if (query.length === 0) return 0;
  if (folded === query) return 0;
  if (folded.startsWith(query)) return 1;
  if (folded.includes(query)) return 2;
  return subsequenceScore(query, folded) === null ? null : 3;
}

describe("palette parity fixture", () => {
  test("prefix modes match the Rust parser", () => {
    for (const entry of fixture.modes) {
      const parsed = parsePaletteQuery(entry.input);
      expect(parsed.mode as string).toBe(entry.mode);
      expect(parsed.term).toBe(entry.term);
    }
  });

  test("rank buckets match the Rust scorer", () => {
    for (const entry of fixture.ranks) {
      expect(rank(entry.term, entry.title)).toBe(entry.rank);
    }
  });
});
