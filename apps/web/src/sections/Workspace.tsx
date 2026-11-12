import { useMemo, useState } from "react";
import { FilePane, SystemBar } from "@aether/ui";
import type { ExplorerEntry } from "@aether/ui";
import { EXPLORER, LINKS } from "../data";

interface PaneState {
  path: string;
  entries: ExplorerEntry[];
  query: string;
  setQuery: (value: string) => void;
  selected: string | null;
  select: (entry: ExplorerEntry) => void;
  open: (entry: ExplorerEntry) => void;
  back: (() => void) | undefined;
  title: string;
}

function usePane(root: string, title: string): PaneState {
  const [stack, setStack] = useState<string[]>([root]);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  const path = stack[stack.length - 1] ?? root;
  const all = EXPLORER[path] ?? [];
  const needle = query.trim().toLowerCase();
  const entries = useMemo(
    () =>
      needle.length === 0
        ? all
        : all.filter((entry) => entry.name.toLowerCase().includes(needle)),
    [all, needle],
  );

  const open = (entry: ExplorerEntry) => {
    if (entry.type !== "folder") return;
    const next = `${path}/${entry.name}`;
    if (EXPLORER[next] === undefined) return;
    setStack([...stack, next]);
    setQuery("");
    setSelected(null);
  };

  const back =
    stack.length > 1
      ? () => {
          setStack(stack.slice(0, -1));
          setQuery("");
          setSelected(null);
        }
      : undefined;

  return {
    path,
    entries,
    query,
    setQuery,
    selected,
    select: (entry) => setSelected(entry.id),
    open,
    back,
    title,
  };
}

export function ExplorerDemo() {
  const home = usePane("/home/oliver", "Home");
  const archive = usePane("/mnt/archive", "Archive");

  return (
    <section className="section" id="explorer">
      <div className="section__head">
        <p className="eyebrow">Explorer</p>
        <h2>A file manager that behaves like the filesystem.</h2>
        <p className="section__lede">
          Dense 30-pixel rows, semantic status pills and keyboard-first search. This
          is the design language of the reference explorer, rebuilt as React
          components and shared with the docs, the marketing site and the shell.
        </p>
      </div>

      <div className="explorer-demo">
        <div className="explorer-demo__bar">
          <span className="explorer-demo__crumb">
            <span className="pulse" aria-hidden="true" /> Live demo · fixture data
          </span>
          <span className="explorer-demo__hint">
            Click to select · double-click a folder to open · <kbd className="ds-kbd">⌘K</kbd>{" "}
            for the palette
          </span>
        </div>

        <div className="explorer-demo__panes">
          <FilePane
            title={home.title}
            subtitle="local · XFS · indexer watching"
            path={home.path}
            entries={home.entries}
            selectedId={home.selected}
            onSelect={home.select}
            onOpen={home.open}
            onBack={home.back}
            onUp={home.back}
            query={home.query}
            onQueryChange={home.setQuery}
            emptyLabel="No local files match that search."
            footer={
              <span>
                {home.entries.length} items · 3 watched roots · no polling
              </span>
            }
          />
          <FilePane
            title={archive.title}
            subtitle="remote · read-only · capability-gated"
            path={archive.path}
            entries={archive.entries}
            selectedId={archive.selected}
            onSelect={archive.select}
            onOpen={archive.open}
            onBack={archive.back}
            onUp={archive.back}
            query={archive.query}
            onQueryChange={archive.setQuery}
            emptyLabel="No archived files match that search."
            footer={
              <span>
                {archive.entries.length} items · 4 status classes · signed manifest
              </span>
            }
          />
        </div>

        <SystemBar
          session="aether@kestrel"
          agents={{ running: 2, max: 6 }}
          cpuPercent={14}
          memoryMiB={1_840}
          audio="Speakers · Analog"
          latencyMs={7.2}
          right={<span className="explorer-demo__legend">Local · Same · Partial · Missing</span>}
        />
      </div>

      <div className="explorer-demo__notes">
        <p>
          The pane is deliberately not a tree. Two explicit roots — local and archive
          — with status per row, because comparing two filesystems is the actual job.
          The underlying <code>FilePane</code>, <code>Badge</code> and{" "}
          <code>SystemBar</code> components are covered by DOM tests.
        </p>
        <a className="text-link" href={`${LINKS.docs}architecture/ui-system/`}>
          UI system spec →
        </a>
      </div>
    </section>
  );
}

const BUDGETS = [
  { metric: "Idle RAM", budget: "≤ 200 MB", gate: "boot test + procps snapshot" },
  { metric: "Idle CPU", budget: "< 0.5 %", gate: "60 s sampling, mean + p95" },
  { metric: "Frame time", budget: "8.3 ms @ 120 Hz", gate: "compositor frame trace" },
  { metric: "Input → paint", budget: "< 16 ms", gate: "evdev timestamp → damage commit" },
  { metric: "Index p95", budget: "< 10 ms @ 10k docs", gate: "bench --check" },
  { metric: "Palette p50", budget: "< 2 ms", gate: "bench --check" },
  { metric: "Userspace boot", budget: "< 1.5 s", gate: "systemd-analyze, cold cache" },
  { metric: "Agent ceiling", budget: "6 concurrent", gate: "admission controller tests" },
];

export function Performance() {
  return (
    <section className="section" id="performance">
      <div className="section__head">
        <p className="eyebrow">Performance ledger</p>
        <h2>Budgets with gates, not adjectives.</h2>
        <p className="section__lede">
          Every headline number has an owner, a measurement method and a failure mode.
          If a claim cannot be gated in CI yet, it is labelled a target — never a
          result.
        </p>
      </div>

      <div className="budget-table" role="table" aria-label="Performance budgets">
        <div className="budget-table__row budget-table__head" role="row">
          <span role="columnheader">Metric</span>
          <span role="columnheader">Budget</span>
          <span role="columnheader">Gate</span>
        </div>
        {BUDGETS.map((row) => (
          <div className="budget-table__row" role="row" key={row.metric}>
            <span role="cell" className="budget-table__metric">
              {row.metric}
            </span>
            <span role="cell" className="budget-table__budget">
              {row.budget}
            </span>
            <span role="cell" className="budget-table__gate">
              {row.gate}
            </span>
          </div>
        ))}
      </div>

      <div className="grid grid--2">
        <article className="panel">
          <h3>Reference measurements (dev host)</h3>
          <ul className="measure-list">
            <li>
              <span>Index query p50, 5k docs</span>
              <strong>1.65 ms</strong>
            </li>
            <li>
              <span>Index query p95, 5k docs</span>
              <strong>2.23 ms</strong>
            </li>
            <li>
              <span>SPSC ring throughput</span>
              <strong>4.5 M msgs/s</strong>
            </li>
            <li>
              <span>Capability verify</span>
              <strong>9.2 M ops/s</strong>
            </li>
            <li>
              <span>Palette p50, 1k items</span>
              <strong>0.78 ms</strong>
            </li>
          </ul>
          <p className="panel__note">
            Apple M3 development machine, Bun 1.4. The committed baseline gates
            relative regressions; absolute numbers on the 7700K are a P1 deliverable.
          </p>
        </article>
        <article className="panel">
          <h3>How the numbers stay honest</h3>
          <ul className="check-list check-list--keep">
            <li>Committed benchmark baseline; CI fails on a 25% regression.</li>
            <li>Ranking uses an injectable clock so tests are deterministic.</li>
            <li>Every subsystem page separates measured, budgeted and speculative.</li>
            <li>Marketing copy never turns a target into a result.</li>
          </ul>
          <a className="text-link" href={`${LINKS.docs}reference/claims-and-evidence/`}>
            Read claims &amp; evidence →
          </a>
        </article>
      </div>
    </section>
  );
}
