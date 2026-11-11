import { describe, expect, test } from "bun:test";
import { buildIndex } from "@aether/indexer";
import type { IndexDocument } from "@aether/protocol";
import {
  PaletteEngine,
  createSearchProvider,
  createStaticProvider,
  modeForKind,
} from "../src/index";

const NOW = Date.UTC(2026, 10, 12);
const DAY = 86_400_000;

const documents: IndexDocument[] = [
  {
    id: "app:settings",
    kind: "setting",
    title: "Settings",
    subtitle: "System preferences",
    modifiedAt: NOW - DAY,
    uses: 8,
  },
  {
    id: "file:/work/project/src/settings.rs",
    kind: "file",
    title: "settings.rs",
    subtitle: "/work/project/src",
    path: "/work/project/src/settings.rs",
    modifiedAt: NOW - 2 * DAY,
  },
  {
    id: "command:open-settings",
    kind: "command",
    title: "Open Settings",
    subtitle: "Aether",
  },
  {
    id: "app:browser",
    kind: "app",
    title: "Browser",
    subtitle: "WebKitGTK shell",
    modifiedAt: NOW - 10 * DAY,
  },
  {
    id: "agent:refactor",
    kind: "agent",
    title: "Refactor worker",
    subtitle: "Untrusted · Landlock",
    modifiedAt: NOW - DAY,
  },
];

function freshEngine(): { engine: PaletteEngine; index: ReturnType<typeof buildIndex> } {
  const index = buildIndex(documents);
  const provider = createSearchProvider(index, { now: () => NOW });
  return { engine: new PaletteEngine({ providers: [provider] }), index };
}

describe("createSearchProvider", () => {
  test("maps index hits to palette items with category, icon and action", async () => {
    const { engine } = freshEngine();
    const result = await engine.query("settings");
    expect(result.items.length).toBeGreaterThan(0);

    const settings = result.items.find((entry) => entry.id === "app:settings");
    expect(settings).toBeDefined();
    expect(settings?.category).toBe("Settings");
    expect(settings?.icon).toBe("⚙");
    expect(settings?.action).toBe("open:app:settings");
    expect(settings?.highlights.length).toBeGreaterThan(0);
    expect(settings?.score).toBeGreaterThan(0);
  });

  test("prefix modes filter the index by document kind", async () => {
    const { engine } = freshEngine();
    const files = await engine.query("/settings");
    expect(files.mode).toBe("file");
    expect(files.items.every((entry) => entry.mode === "file")).toBe(true);
    expect(files.items.map((entry) => entry.id)).toContain(
      "file:/work/project/src/settings.rs",
    );

    const commands = await engine.query(">settings");
    expect(commands.mode).toBe("command");
    expect(commands.items.map((entry) => entry.id)).toEqual(["command:open-settings"]);

    const agents = await engine.query("@refactor");
    expect(agents.mode).toBe("agent");
    expect(agents.items.map((entry) => entry.id)).toEqual(["agent:refactor"]);
  });

  test("empty query returns recent items when allowEmpty is on", async () => {
    const { engine } = freshEngine();
    const result = await engine.query("");
    expect(result.items.length).toBe(documents.length);
    expect(result.items[0]?.id).toBe("app:settings");
  });

  test("allowEmpty can be disabled", async () => {
    const index = buildIndex(documents);
    const engine = new PaletteEngine({
      providers: [createSearchProvider(index, { now: () => NOW, allowEmpty: false })],
    });
    expect((await engine.query("")).items).toEqual([]);
  });

  test("categories, icons and actions are overridable", async () => {
    const index = buildIndex(documents);
    const engine = new PaletteEngine({
      providers: [
        createSearchProvider(index, {
          now: () => NOW,
          categories: { setting: "System" },
          iconFor: () => "*",
          actionFor: (hit) => (hit.kind === "setting" ? "toggle:settings" : undefined),
        }),
      ],
    });
    const result = await engine.query("settings");
    const settings = result.items.find((entry) => entry.id === "app:settings");
    expect(settings?.category).toBe("System");
    expect(settings?.icon).toBe("*");
    expect(settings?.action).toBe("toggle:settings");
  });

  test("modeForKind maps document kinds to palette modes", () => {
    expect(modeForKind("command")).toBe("command");
    expect(modeForKind("file")).toBe("file");
    expect(modeForKind("content")).toBe("file");
    expect(modeForKind("agent")).toBe("agent");
    expect(modeForKind("setting")).toBe("setting");
    expect(modeForKind("app")).toBeUndefined();
  });
});

describe("multiple providers", () => {
  test("app launcher and search results merge into one ranked list", async () => {
    const index = buildIndex(documents);
    const apps = createStaticProvider(
      "launcher",
      [
        {
          id: "app:terminal",
          title: "Terminal",
          category: "Applications",
          icon: "▤",
          action: "launch:terminal",
        },
        {
          id: "app:settings",
          title: "Settings",
          category: "Applications",
          icon: "▤",
          action: "launch:settings",
        },
      ],
      { priority: 5 },
    );
    const engine = new PaletteEngine({
      providers: [apps, createSearchProvider(index, { now: () => NOW })],
    });

    const result = await engine.query("sett");
    expect(result.items[0]?.id).toBe("app:settings");
    // The duplicate settings document collapses to the higher-ranked item.
    expect(result.items.filter((entry) => entry.id === "app:settings").length).toBe(1);
    expect(result.groups.map((group) => group.label)).toContain("Applications");
  });
});
