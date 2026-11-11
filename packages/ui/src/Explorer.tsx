import type { ReactNode } from "react";
import { Badge, type BadgeTone } from "./Chrome";

export interface ExplorerEntry {
  id: string;
  name: string;
  type: "folder" | "file";
  status?: { label: string; tone: BadgeTone };
  size?: string;
  modified?: string;
}

export interface FilePaneProps {
  title: string;
  subtitle?: string;
  path: string;
  entries: readonly ExplorerEntry[];
  selectedId?: string | null;
  onSelect?: (entry: ExplorerEntry) => void;
  onOpen?: (entry: ExplorerEntry) => void;
  query?: string;
  onQueryChange?: (value: string) => void;
  onBack?: () => void;
  onUp?: () => void;
  footer?: ReactNode;
  emptyLabel?: string;
  ariaLabel?: string;
}

/**
 * Dense two-pane file explorer pane, lifted from the Tank ⇄ Jottacloud
 * explorer and rebuilt with the Water design-system primitives. 30px rows,
 * semantic status pills, glass chrome.
 */
export function FilePane(props: FilePaneProps): ReactNode {
  const {
    title,
    subtitle,
    path,
    entries,
    selectedId = null,
    onSelect,
    onOpen,
    query,
    onQueryChange,
    onBack,
    onUp,
    footer,
    emptyLabel = "Nothing here.",
    ariaLabel,
  } = props;

  return (
    <section className="aether-pane ds-scope" aria-label={ariaLabel ?? title}>
      <header className="aether-pane__head">
        <div className="aether-pane__identity">
          <span className="aether-pane__glyph" aria-hidden="true">
            ▤
          </span>
          <div className="aether-pane__titles">
            <h2 className="aether-pane__title">{title}</h2>
            {subtitle === undefined ? null : <p className="aether-pane__subtitle">{subtitle}</p>}
          </div>
        </div>

        <div className="aether-pane__nav">
          <button
            type="button"
            className="aether-icon-btn"
            aria-label="Back"
            disabled={onBack === undefined}
            onClick={onBack}
          >
            ‹
          </button>
          <button
            type="button"
            className="aether-icon-btn"
            aria-label="Up one level"
            disabled={onUp === undefined}
            onClick={onUp}
          >
            ↑
          </button>
          <span className="aether-pane__path" title={path}>
            {path}
          </span>
        </div>

        {onQueryChange === undefined ? null : (
          <label className="aether-pane__search">
            <span aria-hidden="true">⌕</span>
            <input
              type="search"
              value={query ?? ""}
              placeholder={`Search ${title.toLowerCase()}…`}
              aria-label={`Search ${title}`}
              onChange={(event) => onQueryChange(event.target.value)}
            />
          </label>
        )}
      </header>

      <div className="aether-pane__columns" aria-hidden="true">
        <span>Name</span>
        <span>Status</span>
        <span>Size</span>
        <span>Modified</span>
      </div>

      <div className="aether-pane__rows" role="listbox" aria-label={`${title} entries`}>
        {entries.length === 0 ? <p className="aether-pane__empty">{emptyLabel}</p> : null}
        {entries.map((entry) => {
          const selected = entry.id === selectedId;
          return (
            <button
              key={entry.id}
              type="button"
              role="option"
              aria-selected={selected}
              className={`aether-pane__row${selected ? " is-selected" : ""}`}
              onClick={() => onSelect?.(entry)}
              onDoubleClick={() => onOpen?.(entry)}
            >
              <span className="aether-pane__name">
                <span
                  className={`aether-pane__type aether-pane__type--${entry.type}`}
                  aria-hidden="true"
                >
                  {entry.type === "folder" ? "●" : "▢"}
                </span>
                <span className="aether-pane__filename">{entry.name}</span>
              </span>
              <span className="aether-pane__status">
                {entry.status === undefined ? (
                  <span className="aether-pane__muted">—</span>
                ) : (
                  <Badge tone={entry.status.tone}>{entry.status.label}</Badge>
                )}
              </span>
              <span className="aether-pane__size">{entry.size ?? "—"}</span>
              <span className="aether-pane__modified">{entry.modified ?? "—"}</span>
            </button>
          );
        })}
      </div>

      <footer className="aether-pane__foot">
        {footer ?? (
          <span>
            {entries.length} {entries.length === 1 ? "item" : "items"}
          </span>
        )}
      </footer>
    </section>
  );
}
