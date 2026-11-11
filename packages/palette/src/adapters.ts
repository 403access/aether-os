import type { DocumentKind, SearchHit } from "@aether/protocol";
import type { SearchIndex } from "@aether/indexer";
import type { PaletteItem, PaletteMode, PaletteProvider } from "./index";

/**
 * Adapters that turn an `@aether/indexer` corpus into palette results. This is
 * the bridge the shell uses: the palette never owns the index, it only queries
 * it through a provider.
 */

export const DEFAULT_CATEGORY: Readonly<Record<DocumentKind, string>> = {
  app: "Applications",
  command: "Commands",
  file: "Files",
  content: "Content",
  setting: "Settings",
  agent: "Agents",
  contact: "Contacts",
};

export const DEFAULT_ICON: Readonly<Record<DocumentKind, string>> = {
  app: "▤",
  command: "⌘",
  file: "▢",
  content: "▦",
  setting: "⚙",
  agent: "◈",
  contact: "◉",
};

const MODE_KINDS: Readonly<Record<Exclude<PaletteMode, "all">, readonly DocumentKind[]>> = {
  command: ["command"],
  file: ["file", "content"],
  agent: ["agent"],
  setting: ["setting"],
};

export interface SearchProviderOptions {
  id?: string;
  priority?: number;
  limit?: number;
  /** Override display categories per document kind. */
  categories?: Partial<Record<DocumentKind, string>>;
  iconFor?: (hit: SearchHit) => string;
  actionFor?: (hit: SearchHit) => string | undefined;
  /** Show recent items for an empty query. Default true. */
  allowEmpty?: boolean;
  /** Injectable clock for deterministic ranking. */
  now?: () => number;
}

export function createSearchProvider(
  index: SearchIndex,
  options: SearchProviderOptions = {},
): PaletteProvider {
  return {
    id: options.id ?? "search-index",
    priority: options.priority ?? 0,
    search(term, context) {
      const kinds =
        context.mode === "all" ? undefined : MODE_KINDS[context.mode];
      const hits = index.search(term, {
        limit: options.limit ?? context.limit,
        ...(kinds === undefined ? {} : { kinds }),
        allowEmpty: options.allowEmpty ?? true,
        ...(options.now === undefined ? {} : { now: options.now() }),
      });

      return hits.map((hit): PaletteItem => {
        const category = options.categories?.[hit.kind] ?? DEFAULT_CATEGORY[hit.kind];
        const action = options.actionFor?.(hit) ?? `open:${hit.id}`;
        const keywords = [hit.subtitle, hit.path, hit.kind].filter(Boolean).join(" ");
        return {
          id: hit.id,
          title: hit.title,
          ...(hit.subtitle === undefined ? {} : { subtitle: hit.subtitle }),
          category,
          icon: options.iconFor?.(hit) ?? DEFAULT_ICON[hit.kind],
          keywords,
          score: hit.score,
          highlights: hit.highlights,
          action,
          mode: modeForKind(hit.kind),
        };
      });
    },
  };
}

export function modeForKind(kind: DocumentKind): PaletteMode | undefined {
  switch (kind) {
    case "command":
      return "command";
    case "file":
    case "content":
      return "file";
    case "agent":
      return "agent";
    case "setting":
      return "setting";
    default:
      return undefined;
  }
}

/** A provider over a fixed list; the engine still ranks and filters it. */
export function createStaticProvider(
  id: string,
  items: readonly PaletteItem[],
  options: { priority?: number; modes?: readonly PaletteMode[] } = {},
): PaletteProvider {
  const provider: PaletteProvider = {
    id,
    search: () => [...items],
  };
  if (options.priority !== undefined) provider.priority = options.priority;
  if (options.modes !== undefined) provider.modes = options.modes;
  return provider;
}
