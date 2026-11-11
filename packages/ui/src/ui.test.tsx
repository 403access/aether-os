import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PaletteEngine, createStaticProvider, type PaletteResult } from "@aether/palette";
import { ChatTranscript } from "../src/Chat";
import { Meter, SystemBar } from "../src/Chrome";
import { FilePane, type ExplorerEntry } from "../src/Explorer";
import { CommandPalette } from "../src/Palette";
import { usePalette } from "../src/usePalette";

afterEach(cleanup);

function paletteResult(buttonCount = 2): PaletteResult {
  const items = Array.from({ length: buttonCount }, (_, index) => ({
    id: `item-${index}`,
    title: index === 0 ? "Settings" : "Sessions",
    category: "Applications",
    icon: "▤",
    score: 3 - index,
    highlights: index === 0 ? [{ start: 0, end: 4 }] : [],
    providerId: "apps",
    action: `open:${index}`,
  }));
  return {
    query: "sett",
    mode: "all",
    term: "sett",
    items,
    groups: [{ id: "applications", label: "Applications", items }],
    selectedIndex: 0,
    elapsedMs: 1.2,
    stale: false,
    errors: [],
  };
}

describe("ChatTranscript", () => {
  test("renders roles, models and streaming state", () => {
    render(
      <ChatTranscript
        messages={[
          { id: "1", role: "user", content: "Build me an indexer", at: "12:01" },
          {
            id: "2",
            role: "assistant",
            content: "Use **BM25** with `FTS5`.\n```ts\nexport const x = 1;\n```",
            model: "deepseek-v4",
            streaming: true,
          },
        ]}
      />,
    );

    expect(screen.getByText("You")).toBeDefined();
    expect(screen.getByText("DeepSeek")).toBeDefined();
    expect(screen.getByText("deepseek-v4")).toBeDefined();
    expect(screen.getByText("BM25").tagName).toBe("STRONG");
    expect(screen.getByText("FTS5").tagName).toBe("CODE");
    expect(screen.getByLabelText("Streaming")).toBeDefined();
  });

  test("renders an empty state", () => {
    render(<ChatTranscript messages={[]} empty={<p>No messages yet</p>} />);
    expect(screen.getByText("No messages yet")).toBeDefined();
  });
});

describe("CommandPalette", () => {
  test("renders groups, highlights and the active row", () => {
    const result = paletteResult();
    const { container } = render(
      <CommandPalette
        input="sett"
        result={result}
        onInputChange={() => {}}
        onSelect={() => {}}
        loading={false}
        autoFocus={false}
      />,
    );

    expect(screen.getByRole("dialog")).toBeDefined();
    expect(screen.getByText("Applications")).toBeDefined();
    expect(container.querySelector("mark")?.textContent).toBe("Sett");

    const options = screen.getAllByRole("option");
    expect(options.length).toBe(2);
    expect(options[0]?.getAttribute("aria-selected")).toBe("true");
    expect(options[1]?.getAttribute("aria-selected")).toBe("false");
  });

  test("clicking an item selects it", () => {
    const selected: string[] = [];
    render(
      <CommandPalette
        input="sett"
        result={paletteResult()}
        onInputChange={() => {}}
        onSelect={(item) => selected.push(item.id)}
        autoFocus={false}
      />,
    );
    fireEvent.click(screen.getByText("Sessions"));
    expect(selected).toEqual(["item-1"]);
  });

  test("reports the empty state", () => {
    render(
      <CommandPalette
        input="zzz"
        result={{ ...paletteResult(0), query: "zzz", term: "zzz", items: [], groups: [] }}
        onInputChange={() => {}}
        onSelect={() => {}}
        autoFocus={false}
      />,
    );
    expect(screen.getByText(/No matches/)).toBeDefined();
  });
});

describe("usePalette + CommandPalette", () => {
  function Harness({
    engine,
    onAction,
  }: {
    engine: PaletteEngine;
    onAction: (id: string) => void;
  }) {
    const palette = usePalette({ engine, onAction: (item) => onAction(item.id) });
    return (
      <CommandPalette
        input={palette.input}
        result={palette.result}
        onInputChange={palette.setInput}
        onSelect={(item) => onAction(item.id)}
        onKeyDown={palette.onKeyDown}
        loading={palette.loading}
        error={palette.error}
        autoFocus={false}
      />
    );
  }

  test("queries as you type and executes the keyboard selection", async () => {
    const engine = new PaletteEngine({
      providers: [
        createStaticProvider("apps", [
          { id: "app:settings", title: "Settings", category: "Applications" },
          { id: "app:sessions", title: "Sessions", category: "Applications" },
        ]),
      ],
    });
    const actions: string[] = [];
    render(<Harness engine={engine} onAction={(id) => actions.push(id)} />);

    const input = screen.getByLabelText("Search");
    fireEvent.change(input, { target: { value: "s" } });

    await waitFor(() => {
      expect(screen.getAllByRole("option").length).toBeGreaterThan(1);
    });

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(actions.at(-1)).toBeDefined();

    fireEvent.keyDown(input, { key: "Escape" });
  });

  test("shows the loading state before results arrive", async () => {
    const engine = new PaletteEngine({ providers: [] });
    render(<Harness engine={engine} onAction={() => {}} />);
    expect(screen.getByLabelText("Searching")).toBeDefined();
    await waitFor(() => {
      expect(screen.queryByLabelText("Searching")).toBeNull();
    });
  });
});

describe("FilePane", () => {
  const entries: ExplorerEntry[] = [
    {
      id: "dir:projects",
      name: "projects",
      type: "folder",
      status: { label: "Local", tone: "local" },
      modified: "Nov 12",
    },
    {
      id: "file:notes.md",
      name: "notes.md",
      type: "file",
      status: { label: "Synced", tone: "same" },
      size: "4.2 KB",
      modified: "Nov 11",
    },
  ];

  test("renders rows, statuses and selection", () => {
    render(
      <FilePane
        title="Home"
        path="/home/oliver"
        entries={entries}
        selectedId="file:notes.md"
        query=""
        onQueryChange={() => {}}
      />,
    );

    expect(screen.getByText("projects")).toBeDefined();
    expect(screen.getByText("Synced")).toBeDefined();
    expect(screen.getByText("4.2 KB")).toBeDefined();
    expect(screen.getByText("2 items")).toBeDefined();

    const options = screen.getAllByRole("option");
    expect(options[1]?.getAttribute("aria-selected")).toBe("true");
  });

  test("emits select, open and search events", () => {
    const events: string[] = [];
    render(
      <FilePane
        title="Home"
        path="/home/oliver"
        entries={entries}
        onSelect={(entry) => events.push(`select:${entry.id}`)}
        onOpen={(entry) => events.push(`open:${entry.id}`)}
        query=""
        onQueryChange={(value) => events.push(`query:${value}`)}
      />,
    );

    fireEvent.click(screen.getByText("projects"));
    fireEvent.doubleClick(screen.getByText("notes.md"));
    fireEvent.change(screen.getByLabelText("Search Home"), { target: { value: "note" } });

    expect(events).toEqual([
      "select:dir:projects",
      "open:file:notes.md",
      "query:note",
    ]);
  });

  test("shows an empty label", () => {
    render(
      <FilePane title="Empty" path="/empty" entries={[]} emptyLabel="No files here" />,
    );
    expect(screen.getByText("No files here")).toBeDefined();
  });
});

describe("chrome", () => {
  test("SystemBar reports agents, CPU, memory and audio", () => {
    render(
      <SystemBar
        session="aether@kestrel"
        agents={{ running: 2, max: 6 }}
        cpuPercent={12.4}
        memoryMiB={1536}
        audio="Speakers · Analog"
        latencyMs={7.2}
      />,
    );
    expect(screen.getByText("2/6 agents")).toBeDefined();
    expect(screen.getByText("12% CPU")).toBeDefined();
    expect(screen.getByText("1.5 GiB")).toBeDefined();
    expect(screen.getByText("♪ Speakers · Analog")).toBeDefined();
    expect(screen.getByText("7.2 ms frame")).toBeDefined();
  });

  test("Meter exposes accessible values", () => {
    render(<Meter label="Idle RAM" value={148} max={200} detail="148 / 200 MB" tone="green" />);
    const meter = screen.getByRole("meter");
    expect(meter.getAttribute("aria-valuenow")).toBe("148");
    expect(meter.getAttribute("aria-valuemax")).toBe("200");
    expect(screen.getByText("148 / 200 MB")).toBeDefined();
  });
});
