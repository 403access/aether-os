import { describe, expect, test } from "bun:test";
import {
  boundedLevenshtein,
  editBudget,
  fold,
  foldWithMap,
  subsequencePositions,
  subsequenceScore,
  tokenizeText,
  weightTerms,
  FIELD_WEIGHT,
} from "../src/index";

describe("foldWithMap", () => {
  test("strips diacritics and lowercases", () => {
    expect(fold("Café Zürich")).toBe("cafe zurich");
  });

  test("position map points at original UTF-16 offsets", () => {
    const { folded, map } = foldWithMap("Café");
    expect(folded).toBe("cafe");
    // é is a single UTF-16 unit, so offsets stay aligned.
    expect(map).toEqual([0, 1, 2, 3]);
  });

  test("keeps astral characters addressable", () => {
    const text = "a😀b";
    const { folded, map } = foldWithMap(text);
    expect(folded).toBe("a😀b");
    expect(map).toEqual([0, 1, 3]);
  });

  test("folded map never exceeds the source length", () => {
    const source = "Straße Ærø 日本語";
    const { map } = foldWithMap(source);
    for (const offset of map) {
      expect(offset).toBeGreaterThanOrEqual(0);
      expect(offset).toBeLessThan(source.length);
    }
  });
});

describe("tokenizeText", () => {
  test("splits camelCase and PascalCase", () => {
    expect(tokenizeText("getUserProfile")).toEqual(["get", "user", "profile"]);
    expect(tokenizeText("HTTPServer")).toEqual(["http", "server"]);
  });

  test("splits snake, kebab and paths", () => {
    expect(tokenizeText("snake_case-name")).toEqual(["snake", "case", "name"]);
    expect(tokenizeText("/work/aether/README.md")).toEqual([
      "work",
      "aether",
      "readme",
      "md",
    ]);
  });

  test("folds accents and drops single characters", () => {
    expect(tokenizeText("Café à la carte")).toEqual(["cafe", "la", "carte"]);
  });

  test("handles unicode scripts", () => {
    expect(tokenizeText("日本語 テスト")).toEqual(["日本語", "テスト"]);
  });

  test("empty and punctuation-only input yields no tokens", () => {
    expect(tokenizeText("")).toEqual([]);
    expect(tokenizeText("  ---  ")).toEqual([]);
    expect(tokenizeText("a b c")).toEqual([]);
  });

  test("weights title above body", () => {
    const weighted = weightTerms("Settings", "System preferences", ["ui"], "settings settings");
    const titleTf = weighted.frequencies.get("settings") ?? 0;
    expect(titleTf).toBeGreaterThan(FIELD_WEIGHT.body * 2);
    expect(weighted.titleTokens).toEqual(["settings"]);
  });
});

describe("subsequence matching", () => {
  test("matches ordered characters with gaps", () => {
    expect(subsequenceScore("idx", "Indexer")).not.toBeNull();
    expect(subsequencePositions("idx", "Indexer")).toEqual([0, 2, 4]);
  });

  test("rejects out-of-order characters", () => {
    expect(subsequenceScore("xdi", "Indexer")).toBeNull();
  });

  test("contiguous and boundary matches score higher", () => {
    const contiguous = subsequenceScore("open", "open settings");
    const scattered = subsequenceScore("open", "out of place entry");
    expect(contiguous).not.toBeNull();
    expect(scattered).not.toBeNull();
    expect(contiguous ?? 0).toBeGreaterThan(scattered ?? 0);
  });

  test("scores stay within [0, 1]", () => {
    for (const [query, target] of [
      ["a", "a"],
      ["sett", "Settings"],
      ["srch", "Search files and content"],
    ] as const) {
      const score = subsequenceScore(query, target);
      expect(score ?? 0).toBeGreaterThanOrEqual(0);
      expect(score ?? 0).toBeLessThanOrEqual(1);
    }
  });
});

describe("boundedLevenshtein", () => {
  test("computes known distances", () => {
    expect(boundedLevenshtein("indexer", "indexer", 2)).toBe(0);
    expect(boundedLevenshtein("indxer", "indexer", 2)).toBe(1);
    expect(boundedLevenshtein("kitten", "sitting", 3)).toBe(3);
  });

  test("fails fast beyond the budget", () => {
    expect(boundedLevenshtein("abcdef", "uvwxyz", 1)).toBe(2);
    expect(boundedLevenshtein("a", "abcdefgh", 1)).toBe(2);
  });

  test("edit budget scales with term length", () => {
    expect(editBudget("ab")).toBe(0);
    expect(editBudget("abcd")).toBe(1);
    expect(editBudget("indexer")).toBe(2);
  });
});
