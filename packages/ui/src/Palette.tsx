import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import type { PaletteItemResult, PaletteResult } from "@aether/palette";
import type { HighlightRange } from "@aether/protocol";

export interface CommandPaletteProps {
  input: string;
  result: PaletteResult | null;
  onInputChange: (value: string) => void;
  onSelect: (item: PaletteItemResult) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
  onDismiss?: () => void;
  placeholder?: string;
  loading?: boolean;
  error?: string | null;
  footer?: ReactNode;
  autoFocus?: boolean;
  id?: string;
}

export function CommandPalette(props: CommandPaletteProps): ReactNode {
  const {
    input,
    result,
    onInputChange,
    onSelect,
    onKeyDown,
    onDismiss,
    placeholder = "Search apps, files, commands…",
    loading = false,
    error = null,
    footer,
    autoFocus = true,
    id = "aether-palette",
  } = props;

  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (autoFocus) inputRef.current?.focus();
  }, [autoFocus]);

  const listId = `${id}-list`;
  const items = result?.items ?? [];
  const selected = result?.selectedIndex ?? 0;
  const activeId = items[selected] === undefined ? undefined : optionId(id, items[selected]);

  return (
    <div className="ds-scope aether-palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div className="aether-palette__inputRow">
        <span className="aether-palette__prompt" aria-hidden="true">
          ⌘
        </span>
        <input
          ref={inputRef}
          id={`${id}-input`}
          className="aether-palette__input"
          role="combobox"
          aria-expanded={items.length > 0}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={activeId}
          aria-label="Search"
          placeholder={placeholder}
          value={input}
          onChange={(event) => onInputChange(event.target.value)}
          onKeyDown={onKeyDown}
          autoComplete="off"
          spellCheck={false}
        />
        {loading ? <span className="aether-palette__spinner" aria-label="Searching" /> : null}
        {onDismiss === undefined ? null : (
          <button type="button" className="ds-kbd aether-palette__esc" onClick={onDismiss}>
            esc
          </button>
        )}
      </div>

      <div className="aether-palette__list" id={listId} role="listbox" aria-label="Results">
        {result === null && loading ? (
          <p className="aether-palette__empty">Warming the index…</p>
        ) : null}

        {result !== null && items.length === 0 ? (
          <p className="aether-palette__empty">
            No matches for <strong>{result.term || input}</strong>
          </p>
        ) : null}

        {result?.groups.map((group) => (
          <div key={group.id} className="aether-palette__group" role="group" aria-label={group.label}>
            <div className="aether-palette__groupLabel">{group.label}</div>
            {group.items.map((item) => {
              const index = items.findIndex((candidate) => candidate.id === item.id);
              const active = index === selected;
              return (
                <button
                  key={item.id}
                  id={optionId(id, item)}
                  type="button"
                  role="option"
                  aria-selected={active}
                  className={`aether-palette__item${active ? " is-active" : ""}`}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => onSelect(item)}
                >
                  <span className="aether-palette__icon" aria-hidden="true">
                    {item.icon ?? "›"}
                  </span>
                  <span className="aether-palette__text">
                    <span className="aether-palette__title">
                      <Highlighted text={item.title} ranges={item.highlights} />
                    </span>
                    {item.subtitle === undefined ? null : (
                      <span className="aether-palette__subtitle">{item.subtitle}</span>
                    )}
                  </span>
                  {item.action === undefined ? null : (
                    <span className="aether-palette__action" aria-hidden="true">
                      ↵
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </div>

      {error === null ? null : (
        <p className="aether-palette__error" role="alert">
          {error}
        </p>
      )}

      <div className="aether-palette__footer">
        {footer ?? (
          <>
            <span>
              <kbd className="ds-kbd">↑</kbd>
              <kbd className="ds-kbd">↓</kbd> navigate
            </span>
            <span>
              <kbd className="ds-kbd">↵</kbd> open
            </span>
            <span className="aether-palette__hint">
              <kbd className="ds-kbd">&gt;</kbd> commands
              <kbd className="ds-kbd">/</kbd> files
              <kbd className="ds-kbd">@</kbd> agents
            </span>
          </>
        )}
      </div>
    </div>
  );
}

function optionId(base: string, item: PaletteItemResult): string {
  return `${base}-option-${item.id.replace(/[^\w:-]+/g, "_")}`;
}

export function Highlighted({
  text,
  ranges,
}: {
  text: string;
  ranges: readonly HighlightRange[] | undefined;
}): ReactNode {
  if (ranges === undefined || ranges.length === 0) return <>{text}</>;

  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const range of ranges) {
    const start = Math.max(0, Math.min(range.start, text.length));
    const end = Math.max(start, Math.min(range.end, text.length));
    if (start > cursor) parts.push(text.slice(cursor, start));
    if (end > start) {
      parts.push(
        <mark key={`${start}-${end}`} className="aether-mark">
          {text.slice(start, end)}
        </mark>,
      );
    }
    cursor = Math.max(cursor, end);
  }
  if (cursor < text.length) parts.push(text.slice(cursor));
  return <>{parts}</>;
}
