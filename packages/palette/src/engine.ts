import { fold, foldWithMap, mergeRanges, subsequencePositions, subsequenceScore } from "@aether/indexer";
import { clamp, type HighlightRange } from "@aether/protocol";

/**
 * Headless command palette.
 *
 * The engine owns no DOM and no timers: it takes an input string, fans out to
 * providers, merges and ranks results, and returns a plain object that a React
 * component (or the compositor's scene graph) renders. Keyboard navigation is
 * expressed as pure functions over the result so both surfaces behave
 * identically and can be tested without a browser.
 */

export type PaletteMode = "all" | "command" | "file" | "agent" | "setting";

export interface PaletteItem {
  id: string;
  title: string;
  subtitle?: string;
  /** Display group, e.g. "Applications", "Files", "Commands". */
  category: string;
  /** Restrict the item to a mode; omitted means all modes. */
  mode?: PaletteMode;
  icon?: string;
  keywords?: string;
  /** Provider-supplied relevance (e.g. BM25 score). */
  score?: number;
  /** Static bias in [0, 1]. */
  weight?: number;
  /** Action id executed when the item is chosen. */
  action?: string;
  highlights?: HighlightRange[];
}

export interface PaletteContext {
  mode: PaletteMode;
  term: string;
  signal: AbortSignal;
  limit: number;
}

export interface PaletteProvider {
  id: string;
  modes?: readonly PaletteMode[];
  /** Static bias applied to every item from this provider. */
  priority?: number;
  search(query: string, context: PaletteContext): PaletteItem[] | Promise<PaletteItem[]>;
}

export interface PaletteItemResult extends PaletteItem {
  score: number;
  highlights: HighlightRange[];
  providerId: string;
}

export interface PaletteGroup {
  id: string;
  label: string;
  items: PaletteItemResult[];
}

export interface PaletteError {
  providerId: string;
  message: string;
}

export interface PaletteResult {
  query: string;
  mode: PaletteMode;
  term: string;
  items: PaletteItemResult[];
  groups: PaletteGroup[];
  selectedIndex: number;
  elapsedMs: number;
  /** True when a newer query superseded this one; callers should ignore it. */
  stale: boolean;
  errors: PaletteError[];
}

export interface QueryOptions {
  signal?: AbortSignal;
  limit?: number;
  selectedIndex?: number;
  /** Throw AbortError instead of returning a stale result. Default false. */
  rejectStale?: boolean;
}

export interface PaletteEngineOptions {
  providers?: readonly PaletteProvider[];
  limit?: number;
  now?: () => number;
}

const PREFIX_MODE: Readonly<Record<string, PaletteMode>> = {
  ">": "command",
  "@": "agent",
  "/": "file",
  "~": "setting",
};

export function parsePaletteQuery(input: string): { mode: PaletteMode; term: string } {
  const trimmed = input.trimStart();
  const first = trimmed[0];
  if (first !== undefined && PREFIX_MODE[first] !== undefined) {
    return { mode: PREFIX_MODE[first], term: trimmed.slice(1).trim() };
  }
  return { mode: "all", term: trimmed.trim() };
}

export class PaletteEngine {
  readonly #providers = new Map<string, PaletteProvider>();
  readonly #limit: number;
  readonly #now: () => number;
  #request = 0;

  constructor(options: PaletteEngineOptions = {}) {
    for (const provider of options.providers ?? []) this.register(provider);
    this.#limit = clamp(options.limit ?? 12, 1, 100);
    this.#now = options.now ?? (() => performance.now());
  }

  register(provider: PaletteProvider): void {
    this.#providers.set(provider.id, provider);
  }

  unregister(providerId: string): boolean {
    return this.#providers.delete(providerId);
  }

  get providers(): PaletteProvider[] {
    return [...this.#providers.values()];
  }

  async query(input: string, options: QueryOptions = {}): Promise<PaletteResult> {
    const started = this.#now();
    const { mode, term } = parsePaletteQuery(input);
    const limit = clamp(options.limit ?? this.#limit, 1, 100);
    const signal = options.signal ?? new AbortController().signal;
    const requestId = ++this.#request;
    const errors: PaletteError[] = [];

    const active = [...this.#providers.values()].filter(
      (provider) => provider.modes === undefined || provider.modes.includes(mode),
    );

    const context: PaletteContext = { mode, term, signal, limit };
    const settled = await Promise.all(
      active.map(async (provider) => {
        try {
          return { provider, items: await provider.search(term, context), error: null };
        } catch (error) {
          return { provider, items: [] as PaletteItem[], error };
        }
      }),
    );

    if (signal.aborted) throw abortError();

    const candidates: PaletteItemResult[] = [];
    for (const outcome of settled) {
      if (outcome.error !== null) {
        errors.push({
          providerId: outcome.provider.id,
          message:
            outcome.error instanceof Error ? outcome.error.message : String(outcome.error),
        });
        continue;
      }
      const { provider, items } = outcome;
      for (const item of items) {
        if (mode !== "all" && item.mode !== undefined && item.mode !== mode) continue;
        const score = scoreItem(item, term, provider);
        if (score === null) continue;
        candidates.push({
          ...item,
          score,
          highlights: item.highlights ?? [],
          providerId: provider.id,
        });
      }
    }

    const deduped = dedupe(candidates, (providerId) => this.#providers.get(providerId)?.priority ?? 0);
    deduped.sort(compareItems);

    // Highlight only what will actually be rendered.
    const items = deduped.slice(0, limit).map((item) => ({
      ...item,
      highlights:
        item.highlights.length > 0 ? item.highlights : highlight(item.title, term),
    }));
    const groups = groupItems(items);
    const stale = requestId !== this.#request;
    if (stale && options.rejectStale === true) throw abortError();

    return {
      query: input,
      mode,
      term,
      items,
      groups,
      selectedIndex: items.length === 0 ? 0 : clamp(options.selectedIndex ?? 0, 0, items.length - 1),
      elapsedMs: Math.max(0, this.#now() - started),
      stale,
      errors,
    };
  }
}

/** Move the selection by `delta`, wrapping at both ends. */
export function moveSelection(result: PaletteResult, delta: number): PaletteResult {
  const size = result.items.length;
  if (size === 0) return result;
  const next = (((result.selectedIndex + delta) % size) + size) % size;
  return { ...result, selectedIndex: next };
}

/** Page through the list (default five rows per page). */
export function pageSelection(result: PaletteResult, direction: 1 | -1, pageSize = 5): PaletteResult {
  return moveSelection(result, direction * pageSize);
}

export function selectedItem(result: PaletteResult): PaletteItemResult | null {
  return result.items[result.selectedIndex] ?? null;
}

export function executeSelection(result: PaletteResult): PaletteItemResult | null {
  return selectedItem(result);
}

// ---------------------------------------------------------------------------
// Ranking
// ---------------------------------------------------------------------------

function scoreItem(
  item: PaletteItem,
  term: string,
  provider: PaletteProvider,
): number | null {
  const providerBias = provider.priority ?? 0;
  const weight = item.weight ?? 0;
  const supplied = item.score === undefined ? 0 : Math.min(Math.abs(item.score), 10) * 0.15;

  if (term.length === 0) return providerBias * 0.05 + weight + supplied;

  const haystack = `${item.title} ${item.keywords ?? ""} ${item.subtitle ?? ""}`;
  const fuzzy = subsequenceScore(term, haystack);
  if (fuzzy === null) return null;

  const title = fold(item.title);
  const query = fold(term);

  let boost = 0;
  if (title === query) boost += 1.25;
  else if (title.startsWith(query)) boost += 0.5;
  else if (title.includes(query)) boost += 0.25;

  return providerBias * 0.05 + weight + fuzzy * 2 + boost + supplied;
}

/**
 * Collapse duplicate ids. A higher-priority provider wins outright (an explicit
 * launcher entry overrides an indexed file); otherwise the better score wins.
 */
function dedupe(
  items: readonly PaletteItemResult[],
  priorityOf: (providerId: string) => number,
): PaletteItemResult[] {
  const best = new Map<string, PaletteItemResult>();
  for (const item of items) {
    const existing = best.get(item.id);
    if (existing === undefined) {
      best.set(item.id, item);
      continue;
    }
    const priorityDelta = priorityOf(item.providerId) - priorityOf(existing.providerId);
    if (priorityDelta > 0 || (priorityDelta === 0 && item.score > existing.score)) {
      best.set(item.id, item);
    }
  }
  return [...best.values()];
}

function compareItems(a: PaletteItemResult, b: PaletteItemResult): number {
  if (b.score !== a.score) return b.score - a.score;
  const category = compareText(a.category, b.category);
  if (category !== 0) return category;
  return compareText(a.title, b.title);
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function groupItems(items: readonly PaletteItemResult[]): PaletteGroup[] {
  const groups = new Map<string, PaletteGroup>();
  for (const item of items) {
    let group = groups.get(item.category);
    if (group === undefined) {
      group = { id: slug(item.category), label: item.category, items: [] };
      groups.set(item.category, group);
    }
    group.items.push(item);
  }
  return [...groups.values()];
}

function slug(value: string): string {
  return fold(value).replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

export function highlight(title: string, term: string): HighlightRange[] {
  if (term.length === 0 || title.length === 0) return [];
  const query = fold(term);
  const { folded, map } = foldWithMap(title);
  const offsetAt = (index: number): number =>
    index >= map.length ? title.length : (map[index] ?? title.length);

  const direct = folded.indexOf(query);
  if (direct >= 0) {
    return [{ start: offsetAt(direct), end: offsetAt(direct + query.length) }];
  }

  const positions = subsequencePositions(term, folded);
  if (positions === null) return [];
  return mergeRanges(
    positions.map((position) => ({ start: offsetAt(position), end: offsetAt(position + 1) })),
  );
}

function abortError(): Error {
  const error = new Error("Palette query aborted");
  error.name = "AbortError";
  return error;
}
