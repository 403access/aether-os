import { describe, expect, test } from "bun:test";
import {
  PaletteEngine,
  createStaticProvider,
  executeSelection,
  highlight,
  moveSelection,
  pageSelection,
  parsePaletteQuery,
  selectedItem,
  type PaletteItem,
  type PaletteProvider,
  type PaletteResult,
} from "../src/index";

const item = (overrides: Partial<PaletteItem> = {}): PaletteItem => ({
  id: "app:settings",
  title: "Settings",
  category: "Applications",
  action: "open:settings",
  ...overrides,
});

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

describe("parsePaletteQuery", () => {
  test("recognises mode prefixes", () => {
    expect(parsePaletteQuery(">open")).toEqual({ mode: "command", term: "open" });
    expect(parsePaletteQuery("@refactor")).toEqual({ mode: "agent", term: "refactor" });
    expect(parsePaletteQuery("/src")).toEqual({ mode: "file", term: "src" });
    expect(parsePaletteQuery("~audio")).toEqual({ mode: "setting", term: "audio" });
  });

  test("defaults to all modes and trims", () => {
    expect(parsePaletteQuery("  settings ")).toEqual({ mode: "all", term: "settings" });
    expect(parsePaletteQuery("")).toEqual({ mode: "all", term: "" });
  });
});

describe("PaletteEngine ranking", () => {
  test("exact title beats prefix beats fuzzy", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("apps", [
          { id: "a", title: "Settings", category: "Applications" },
          { id: "b", title: "Search Settings", category: "Applications" },
          { id: "c", title: "Session Manager", category: "Applications" },
        ]),
      ],
    });
    const result = await engine.query("settings");
    expect(result.items[0]?.id).toBe("a");
    expect(result.items.map((entry) => entry.id)).toContain("b");
  });

  test("provider priority and item weight break ties", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("low", [{ id: "low", title: "Same", category: "X" }], {
          priority: 0,
        }),
        createStaticProvider("high", [{ id: "high", title: "Same", category: "X" }], {
          priority: 10,
        }),
      ],
    });
    const result = await engine.query("same");
    expect(result.items[0]?.id).toBe("high");
  });

  test("deduplicates ids across providers, keeping the best score", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("one", [
          { id: "shared", title: "Shared", category: "A", weight: 0 },
        ]),
        createStaticProvider("two", [
          { id: "shared", title: "Shared", category: "A", weight: 0.9 },
        ]),
      ],
    });
    const result = await engine.query("shared");
    expect(result.items.length).toBe(1);
    expect(result.items[0]?.providerId).toBe("two");
  });

  test("filters out non-matching items but keeps everything for an empty query", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("apps", [
          { id: "a", title: "Terminal", category: "Applications" },
          { id: "b", title: "Browser", category: "Applications" },
        ]),
      ],
    });
    expect((await engine.query("term")).items.map((entry) => entry.id)).toEqual(["a"]);
    expect((await engine.query("")).items.length).toBe(2);
  });

  test("groups results by category in score order", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("all", [
          { id: "f", title: "File thing", category: "Files", keywords: "open" },
          { id: "c", title: "Open thing", category: "Commands", keywords: "open" },
        ]),
      ],
    });
    const result = await engine.query("open");
    expect(result.groups.map((group) => group.label)).toEqual(["Commands", "Files"]);
    expect(result.groups[0]?.id).toBe("commands");
  });

  test("respects the result limit", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider(
          "many",
          Array.from({ length: 30 }, (_, index) => ({
            id: `i${index}`,
            title: `Item ${index}`,
            category: "All",
          })),
        ),
      ],
    });
    expect((await engine.query("item", { limit: 5 })).items.length).toBe(5);
  });
});

describe("modes", () => {
  test("a prefix restricts providers and item mode", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("files", [
          { id: "f", title: "file.ts", category: "Files", mode: "file" },
        ]),
        createStaticProvider("commands", [
          { id: "c", title: "file command", category: "Commands", mode: "command" },
        ]),
      ],
    });

    expect((await engine.query("/file")).items.map((entry) => entry.id)).toEqual(["f"]);
    expect((await engine.query(">file")).items.map((entry) => entry.id)).toEqual(["c"]);
    // In the unfiltered mode, mode-tagged items are still visible.
    const all = await engine.query("file");
    expect(all.items.map((entry) => entry.id).sort()).toEqual(["c", "f"]);
  });
});

describe("resilience", () => {
  test("a failing provider is reported without taking down the palette", async () => {
    const broken: PaletteProvider = {
      id: "broken",
      search: () => {
        throw new Error("provider exploded");
      },
    };
    const engine = new PaletteEngine({
      providers: [broken, createStaticProvider("ok", [item()])],
    });
    const result = await engine.query("settings");
    expect(result.items.length).toBe(1);
    expect(result.errors).toEqual([{ providerId: "broken", message: "provider exploded" }]);
  });

  test("a superseded query is marked stale", async () => {
    const gate = deferred<PaletteItem[]>();
    let calls = 0;
    const slow: PaletteProvider = {
      id: "slow",
      search: () => {
        calls += 1;
        return calls === 1 ? gate.promise : Promise.resolve([]);
      },
    };
    const engine = new PaletteEngine({
      providers: [slow, createStaticProvider("fast", [item()])],
    });

    const first = engine.query("settings");
    const second = await engine.query("settings");
    expect(second.stale).toBe(false);

    gate.resolve([{ id: "late", title: "Late", category: "Applications" }]);
    const firstResult = await first;
    expect(firstResult.stale).toBe(true);
  });

  test("an aborted signal rejects the query", async () => {
    const engine = new PaletteEngine({
      providers: [createStaticProvider("apps", [item()])],
    });
    const controller = new AbortController();
    const promise = engine.query("settings", { signal: controller.signal });
    controller.abort();
    await expect(promise).rejects.toThrow(/aborted/i);
  });

  test("provider registration is replaceable", async () => {
    const engine = new PaletteEngine();
    engine.register(createStaticProvider("p", [item()]));
    expect((await engine.query("set")).items.length).toBe(1);
    expect(engine.unregister("p")).toBe(true);
    expect(engine.unregister("p")).toBe(false);
    expect((await engine.query("set")).items.length).toBe(0);
  });
});

describe("keyboard navigation", () => {
  const result = {
    query: "",
    mode: "all",
    term: "",
    items: [
      { id: "a", title: "A", category: "X", score: 3, highlights: [], providerId: "p" },
      { id: "b", title: "B", category: "X", score: 2, highlights: [], providerId: "p" },
      { id: "c", title: "C", category: "X", score: 1, highlights: [], providerId: "p" },
    ],
    groups: [],
    selectedIndex: 0,
    elapsedMs: 0,
    stale: false,
    errors: [],
  } satisfies PaletteResult;

  test("moves and wraps in both directions", () => {
    expect(moveSelection(result, 1).selectedIndex).toBe(1);
    expect(moveSelection(result, 2).selectedIndex).toBe(2);
    expect(moveSelection(result, 3).selectedIndex).toBe(0);
    expect(moveSelection(result, -1).selectedIndex).toBe(2);
  });

  test("pages by page size with wrap", () => {
    expect(pageSelection(result, 1, 2).selectedIndex).toBe(2);
    expect(pageSelection(result, -1, 2).selectedIndex).toBe(1);
  });

  test("empty results keep selection at zero and execute nothing", () => {
    const empty = { ...result, items: [], selectedIndex: 0 };
    expect(moveSelection(empty, 1).selectedIndex).toBe(0);
    expect(selectedItem(empty)).toBeNull();
    expect(executeSelection(empty)).toBeNull();
  });

  test("executes the selected item's action", () => {
    expect(executeSelection({ ...result, selectedIndex: 1 })?.id).toBe("b");
  });
});

describe("highlight", () => {
  test("highlights a direct substring", () => {
    expect(highlight("Settings", "sett")).toEqual([{ start: 0, end: 4 }]);
    expect(highlight("Search Settings", "sett")).toEqual([{ start: 7, end: 11 }]);
  });

  test("falls back to a subsequence for non-contiguous queries", () => {
    const ranges = highlight("Settings", "stg");
    expect(ranges.length).toBe(3);
    expect(ranges[0]).toEqual({ start: 0, end: 1 });
  });

  test("returns nothing for unrelated text", () => {
    expect(highlight("Settings", "zzz")).toEqual([]);
    expect(highlight("", "a")).toEqual([]);
  });
});
