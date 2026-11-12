import { useEffect, useMemo, useState } from "react";
import { usePalette } from "@aether/ui";
import { PaletteEngine, createSearchProvider, createStaticProvider } from "@aether/palette";
import { buildIndex } from "@aether/indexer";
import { CommandPalette } from "@aether/ui";
import { DEMO_APPS, DEMO_COMMANDS, DEMO_DOCUMENTS, LINKS, withBase } from "../data";

/**
 * Live palette: the real engine, the real index, the real ranking. Only the
 * corpus is a demo fixture.
 */
export function PaletteDemo() {
  const engine = useMemo(() => {
    const index = buildIndex(DEMO_DOCUMENTS);
    return new PaletteEngine({
      providers: [
        createStaticProvider("launcher", DEMO_APPS, { priority: 10 }),
        createStaticProvider("palette-commands", DEMO_COMMANDS, { priority: 6 }),
        createSearchProvider(index, {
          id: "search",
          priority: 0,
          allowEmpty: true,
          now: () => Date.UTC(2026, 10, 12),
        }),
      ],
      limit: 12,
    });
  }, []);

  const [lastAction, setLastAction] = useState("Nothing dispatched yet.");
  const palette = usePalette({
    engine,
    onAction: (item) => setLastAction(`${item.action ?? item.id} → aetherd`),
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        focusPalette();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <section className="section section--palette" id="palette">
      <div className="section__head">
        <p className="eyebrow">Command palette</p>
        <h2>The whole machine, one keystroke.</h2>
        <p className="section__lede">
          Applications, files, content, agents and system commands share one ranked
          surface. Type <code className="inline">/</code> for files, <code className="inline">&gt;</code>{" "}
          for commands, <code className="inline">@</code> for agents, or{" "}
          <code className="inline">~</code> for settings.
        </p>
      </div>

      <div className="palette-layout">
        <div className="palette-card">
          <div className="palette-card__chrome">
            <span className="traffic" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
            <span className="palette-card__title">aether-palette</span>
            <button type="button" className="ds-btn ds-btn--small" onClick={focusPalette}>
              Focus <kbd className="ds-kbd">⌘K</kbd>
            </button>
          </div>

          <CommandPalette
            input={palette.input}
            result={palette.result}
            onInputChange={palette.setInput}
            onSelect={(item) => setLastAction(`${item.action ?? item.id} → aetherd`)}
            onKeyDown={palette.onKeyDown}
            loading={palette.loading}
            error={palette.error}
            onDismiss={() => palette.setInput("")}
            placeholder="Search apps, files, commands…"
            autoFocus={false}
          />

          <div className="palette-card__actions">
            <span className="palette-card__label">Try:</span>
            {[
              { label: "files", value: "/index" },
              { label: "commands", value: ">agent" },
              { label: "agents", value: "@refactor" },
              { label: "typo", value: "compositr" },
            ].map((chip) => (
              <button
                key={chip.value}
                type="button"
                className="chip"
                onClick={() => {
                  palette.setInput(chip.value);
                  focusPalette();
                }}
              >
                {chip.label}
              </button>
            ))}
            <output className="palette-card__result" aria-live="polite">
              {lastAction}
            </output>
          </div>
        </div>

        <aside className="palette-notes">
          <div className="note">
            <h3>Ranking, not filtering</h3>
            <p>
              Field-weighted BM25F over title, tags, path and content, plus explicit
              boosts for exact titles, recency and usage. Prefix expansion and bounded
              edit distance handle the way people actually type.
            </p>
          </div>
          <div className="note">
            <h3>One animation budget</h3>
            <p>
              The palette is a compositor layer, not an app window. It fades in at
              190 ms, never blocks the frame, and returns keystrokes in the same
              event loop tick.
            </p>
          </div>
          <div className="note">
            <h3>Deterministic by construction</h3>
            <p>
              Ranking takes an injectable clock. Every ordering in this demo is
              covered by unit tests, including ties, typos and stale queries.
            </p>
          </div>
          <a className="text-link" href={`${LINKS.docs}architecture/search`}>
            How the index works →
          </a>
          <a className="text-link" href={withBase("design-system/")}>
            Water design system →
          </a>
        </aside>
      </div>
    </section>
  );
}

function focusPalette(): void {
  const input = document.getElementById("aether-palette-input");
  input?.scrollIntoView({ behavior: "smooth", block: "center" });
  requestAnimationFrame(() => input?.focus());
}
