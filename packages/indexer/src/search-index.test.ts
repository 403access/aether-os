import { describe, expect, test } from "bun:test";
import type { IndexDocument } from "@aether/protocol";
import {
  buildIndex,
  highlightTitle,
  mergeRanges,
  recencyBoost,
  SearchIndex,
  usageBoost,
} from "../src/index";

const NOW = Date.UTC(2026, 10, 12);
const DAY = 86_400_000;

const documents: IndexDocument[] = [
  {
    id: "app:settings",
    kind: "setting",
    title: "Settings",
    subtitle: "System preferences",
    body: "display audio network keyboard",
    modifiedAt: NOW - 2 * DAY,
    uses: 4,
  },
  {
    id: "app:search-settings",
    kind: "app",
    title: "Search Settings",
    subtitle: "Configure the indexer",
    body: "settings for the file search index",
    modifiedAt: NOW - 100 * DAY,
  },
  {
    id: "file:/work/project/src/indexer.ts",
    kind: "file",
    title: "indexer.ts",
    subtitle: "/work/project/src",
    body: "export class SearchIndex { bm25 postings }",
    modifiedAt: NOW - DAY,
    uses: 10,
  },
  {
    id: "file:/work/project/src/settings.rs",
    kind: "file",
    title: "settings.rs",
    subtitle: "/work/project/src",
    body: "settings persistence for the index",
    modifiedAt: NOW - DAY,
  },
  {
    id: "command:open-settings",
    kind: "command",
    title: "Open Settings",
    subtitle: "Aether",
    modifiedAt: NOW - 300 * DAY,
  },
  {
    id: "file:/media/holiday.jpg",
    kind: "file",
    title: "holiday.jpg",
    subtitle: "/media",
    path: "/media/holiday.jpg",
    modifiedAt: NOW - 400 * DAY,
  },
];

function freshIndex(): SearchIndex {
  return buildIndex(documents);
}

describe("SearchIndex ranking", () => {
  test("exact title match beats body fragment", () => {
    const hits = freshIndex().search("settings", { now: NOW });
    expect(hits.length).toBeGreaterThan(0);
    expect(hits[0]?.title).toBe("Settings");
    const bodyHit = hits.find((hit) => hit.id.includes("settings.rs"));
    expect(bodyHit).toBeDefined();
    expect(hits[0]?.score ?? 0).toBeGreaterThan(bodyHit?.score ?? 0);
  });

  test("kind prior lifts commands over files for equal text", () => {
    const index = buildIndex([
      { id: "file:/a/search", kind: "file", title: "search", subtitle: "/a" },
      { id: "command:search", kind: "command", title: "search" },
    ]);
    const hits = index.search("search", { now: NOW });
    expect(hits[0]?.kind).toBe("command");
  });

  test("multi-term queries require all terms (AND semantics)", () => {
    const hits = freshIndex().search("search settings", { now: NOW });
    expect(hits.map((hit) => hit.id)).toEqual(["app:search-settings"]);
  });

  test("recency and usage boost ranks recently used items first among equals", () => {
    const hits = freshIndex().search("settings", { now: NOW });
    const settings = hits.find((hit) => hit.id === "app:settings");
    const openSettings = hits.find((hit) => hit.id === "command:open-settings");
    expect(settings).toBeDefined();
    expect(openSettings).toBeDefined();
    // Exact title + 4 uses + 2 days old beats an older exact-ish title.
    expect(settings?.score ?? 0).toBeGreaterThan(openSettings?.score ?? 0);
  });

  test("search is deterministic and stable for ties", () => {
    const index = buildIndex([
      { id: "b", kind: "app", title: "Same" },
      { id: "a", kind: "app", title: "Same" },
      { id: "c", kind: "app", title: "Same" },
    ]);
    const first = index.search("same", { now: NOW }).map((hit) => hit.id);
    const second = index.search("same", { now: NOW }).map((hit) => hit.id);
    expect(first).toEqual(second);
    expect(first).toEqual(["a", "b", "c"]);
  });

  test("query with no match returns empty", () => {
    expect(freshIndex().search("zzzznope", { now: NOW })).toEqual([]);
  });

  test("filters by kind", () => {
    const hits = freshIndex().search("settings", { now: NOW, kinds: ["command"] });
    expect(hits.map((hit) => hit.kind)).toEqual(["command"]);
  });

  test("respects the limit", () => {
    expect(freshIndex().search("settings", { now: NOW, limit: 1 }).length).toBe(1);
  });
});

describe("prefix and fuzzy matching", () => {
  test("prefix expansion finds settings from 'sett'", () => {
    const hits = freshIndex().search("sett", { now: NOW });
    expect(hits.map((hit) => hit.title)).toContain("Settings");
  });

  test("prefix can be disabled", () => {
    const hits = freshIndex().search("sett", { now: NOW, prefix: false, fuzzy: false });
    expect(hits).toEqual([]);
  });

  test("single-edit typo finds the indexer document", () => {
    const hits = freshIndex().search("indxer", { now: NOW });
    expect(hits.map((hit) => hit.id)).toContain("file:/work/project/src/indexer.ts");
  });

  test("fuzzy can be disabled", () => {
    const hits = freshIndex().search("indxer", { now: NOW, fuzzy: false });
    expect(hits).toEqual([]);
  });

  test("single character queries fall back to subsequence title search", () => {
    const hits = freshIndex().search("s", { now: NOW });
    expect(hits.length).toBeGreaterThan(0);
    expect(hits.every((hit) => hit.title.toLowerCase().includes("s"))).toBe(true);
  });
});

describe("empty query and default ranking", () => {
  test("empty query returns nothing unless allowEmpty", () => {
    expect(freshIndex().search("", { now: NOW })).toEqual([]);
    expect(freshIndex().search("   ", { now: NOW })).toEqual([]);
  });

  test("allowEmpty ranks by prior, recency and usage", () => {
    const hits = freshIndex().search("", { now: NOW, allowEmpty: true });
    expect(hits.length).toBe(documents.length);
    for (let index = 1; index < hits.length; index += 1) {
      expect(hits[index - 1]?.score ?? 0).toBeGreaterThanOrEqual(hits[index]?.score ?? 0);
    }
    expect(hits[0]?.highlights).toEqual([]);
  });

  test("a heavily used recent item outranks an unused stale twin", () => {
    const index = buildIndex([
      { id: "old", kind: "app", title: "Same App", modifiedAt: NOW - 365 * DAY, uses: 0 },
      { id: "used", kind: "app", title: "Same App", modifiedAt: NOW - DAY, uses: 50 },
    ]);
    const hits = index.search("", { now: NOW, allowEmpty: true });
    expect(hits.map((hit) => hit.id)).toEqual(["used", "old"]);
  });
});

describe("incremental updates", () => {
  test("upsert replaces and remove deletes", () => {
    const index = freshIndex();
    expect(index.size).toBe(documents.length);
    expect(index.remove("app:settings")).toBe(true);
    expect(index.remove("app:settings")).toBe(false);
    expect(index.search("settings", { now: NOW }).map((hit) => hit.id)).not.toContain(
      "app:settings",
    );

    index.upsert({ id: "app:settings", kind: "setting", title: "Settings" });
    expect(index.size).toBe(documents.length);
    expect(index.search("settings", { now: NOW }).map((hit) => hit.id)).toContain("app:settings");
  });

  test("upsert does not leak stale terms from the previous version", () => {
    const index = new SearchIndex();
    index.upsert({ id: "x", kind: "file", title: "AlphaReport" });
    index.upsert({ id: "x", kind: "file", title: "BetaReport" });
    expect(index.search("alpha", { now: NOW })).toEqual([]);
    expect(index.search("beta", { now: NOW }).length).toBe(1);
  });

  test("stats track vocabulary and average length", () => {
    const index = freshIndex();
    const stats = index.stats();
    expect(stats.documents).toBe(documents.length);
    expect(stats.terms).toBeGreaterThan(10);
    expect(stats.averageLength).toBeGreaterThan(0);
  });

  test("clear empties the index", () => {
    const index = freshIndex();
    index.clear();
    expect(index.size).toBe(0);
    expect(index.stats().terms).toBe(0);
    expect(index.search("settings", { now: NOW })).toEqual([]);
  });
});

describe("snapshots", () => {
  test("round-trips documents and ranking", () => {
    const index = freshIndex();
    const restored = SearchIndex.fromSnapshot(index.snapshot());
    expect(restored.size).toBe(index.size);
    expect(restored.search("settings", { now: NOW }).map((hit) => hit.id)).toEqual(
      index.search("settings", { now: NOW }).map((hit) => hit.id),
    );
  });

  test("caps indexed body size", () => {
    const index = new SearchIndex({ maxBodyChars: 64 });
    index.upsert({ id: "big", kind: "file", title: "big.log", body: "x".repeat(10_000) });
    expect(index.snapshot().documents[0]?.body?.length).toBe(64);
    expect(index.get("big")?.body?.length).toBe(64);
  });
});

describe("highlights", () => {
  test("exact token highlights the whole word", () => {
    expect(highlightTitle("Open Settings", ["settings"], "exact")).toEqual([
      { start: 5, end: 13 },
    ]);
  });

  test("prefix highlights only the typed prefix", () => {
    expect(highlightTitle("Settings", ["sett"], "prefix")).toEqual([{ start: 0, end: 4 }]);
  });

  test("fuzzy highlights a subsequence", () => {
    const ranges = highlightTitle("Indexer", ["indxer"], "fuzzy");
    expect(ranges.length).toBeGreaterThan(0);
    expect(ranges[0]?.start).toBe(0);
  });

  test("diacritics map back to the original string", () => {
    expect(highlightTitle("Café Menu", ["cafe"], "exact")).toEqual([{ start: 0, end: 4 }]);
  });

  test("no highlights for fragments", () => {
    expect(highlightTitle("Unrelated title", ["settings"], "fragment")).toEqual([]);
  });

  test("ranges merge when overlapping or adjacent", () => {
    expect(
      mergeRanges([
        { start: 5, end: 8 },
        { start: 0, end: 2 },
        { start: 2, end: 5 },
        { start: 8, end: 10 },
        { start: 12, end: 14 },
      ]),
    ).toEqual([
      { start: 0, end: 10 },
      { start: 12, end: 14 },
    ]);
  });

  test("highlight ranges always slice the original title", () => {
    const title = "Café Indexer Settings";
    const ranges = highlightTitle(title, ["cafe", "indxer", "sett"], "fuzzy");
    expect(ranges.length).toBeGreaterThan(0);
    for (const range of ranges) {
      expect(title.slice(range.start, range.end).length).toBeGreaterThan(0);
    }
  });
});

describe("boost helpers", () => {
  test("recency decays with a 30 day half-life", () => {
    expect(recencyBoost(NOW, NOW)).toBeCloseTo(0.35, 5);
    expect(recencyBoost(NOW - 30 * DAY, NOW)).toBeCloseTo(0.175, 5);
    expect(recencyBoost(undefined, NOW)).toBe(0);
    expect(recencyBoost(NOW + DAY, NOW)).toBeCloseTo(0.35, 5);
  });

  test("usage is logarithmic and never negative", () => {
    expect(usageBoost(0)).toBe(0);
    expect(usageBoost(10)).toBeGreaterThan(usageBoost(1));
    expect(usageBoost(-5)).toBe(0);
  });
});
