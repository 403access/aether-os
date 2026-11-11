import {
  KIND_PRIOR,
  clamp,
  type DocumentKind,
  type HighlightRange,
  type IndexDocument,
  type MatchSource,
  type SearchHit,
} from "@aether/protocol";
import { boundedLevenshtein, editBudget } from "./fuzzy";
import {
  fold,
  foldWithMap,
  subsequencePositions,
  subsequenceScore,
  tokenizeQuery,
  weightTerms,
  type WeightedTerms,
} from "./tokenize";

export interface SearchIndexOptions {
  /** BM25 term-frequency saturation. */
  k1?: number;
  /** BM25 length normalisation, 0 = off, 1 = full. */
  b?: number;
  /** Hard cap on indexed body characters per document. */
  maxBodyChars?: number;
}

export interface SearchOptions {
  limit?: number;
  kinds?: readonly DocumentKind[];
  /** Expand the last token by prefix ("sett" -> "settings"). Default true. */
  prefix?: boolean;
  /** Allow edit-distance matches for unmatched tokens. Default true. */
  fuzzy?: boolean;
  /** Injectable clock for deterministic ranking tests. */
  now?: number;
  /** Return recent/high-prior documents for an empty query. Default false. */
  allowEmpty?: boolean;
  maxPrefixExpansions?: number;
  maxFuzzyCandidates?: number;
}

export interface IndexStats {
  documents: number;
  terms: number;
  averageLength: number;
}

export interface SerializedIndex {
  version: 1;
  documents: IndexDocument[];
}

interface Entry {
  doc: IndexDocument;
  frequencies: Map<string, number>;
  length: number;
  titleFolded: string;
  titleTokens: Set<string>;
  modifiedAt: number | undefined;
  uses: number;
}

const DAY_MS = 86_400_000;
const WORD = /[\p{L}\p{N}]+/gu;

function lowerBound(sorted: readonly string[], target: string): number {
  let low = 0;
  let high = sorted.length;
  while (low < high) {
    const mid = (low + high) >>> 1;
    const value = sorted[mid];
    if (value !== undefined && value < target) low = mid + 1;
    else high = mid;
  }
  return low;
}

/**
 * In-memory search index.
 *
 * Ranking is BM25F-lite (field-weighted term frequencies) plus explicit boosts
 * for exact title matches, recency and usage. All boosts are additive and
 * documented in `apps/docs/docs/architecture/search.md`.
 *
 * Complexity: upsert O(tokens in doc), query O(terms × postings + vocabulary
 * scan for prefix/fuzzy). A trigram accelerator for fuzzy matching is a
 * roadmap item; today fuzzy is capped by `maxFuzzyCandidates`.
 */
export class SearchIndex {
  readonly #entries = new Map<string, Entry>();
  readonly #postings = new Map<string, Map<string, number>>();
  readonly #k1: number;
  readonly #b: number;
  readonly #maxBodyChars: number;
  #totalLength = 0;
  #vocabulary: string[] | null = null;

  constructor(options: SearchIndexOptions = {}) {
    this.#k1 = options.k1 ?? 1.2;
    this.#b = options.b ?? 0.75;
    this.#maxBodyChars = options.maxBodyChars ?? 20_000;
  }

  get size(): number {
    return this.#entries.size;
  }

  has(id: string): boolean {
    return this.#entries.has(id);
  }

  get(id: string): IndexDocument | undefined {
    return this.#entries.get(id)?.doc;
  }

  upsert(input: IndexDocument): void {
    if (this.#entries.has(input.id)) this.remove(input.id);

    const body =
      input.body === undefined ? undefined : input.body.slice(0, this.#maxBodyChars);
    const doc: IndexDocument = body === undefined ? { ...input } : { ...input, body };

    const weighted: WeightedTerms = weightTerms(doc.title, doc.subtitle, doc.tags, body);
    const entry: Entry = {
      doc,
      frequencies: weighted.frequencies,
      length: Math.max(weighted.length, 1),
      titleFolded: fold(doc.title),
      titleTokens: new Set(weighted.titleTokens),
      modifiedAt: doc.modifiedAt,
      uses: doc.uses ?? 0,
    };

    this.#entries.set(doc.id, entry);
    this.#totalLength += entry.length;

    for (const [token, frequency] of weighted.frequencies) {
      let posting = this.#postings.get(token);
      if (posting === undefined) {
        posting = new Map<string, number>();
        this.#postings.set(token, posting);
        this.#vocabulary = null;
      }
      posting.set(doc.id, frequency);
    }
  }

  upsertMany(documents: readonly IndexDocument[]): void {
    for (const document of documents) this.upsert(document);
  }

  remove(id: string): boolean {
    const entry = this.#entries.get(id);
    if (entry === undefined) return false;

    for (const token of entry.frequencies.keys()) {
      const posting = this.#postings.get(token);
      if (posting === undefined) continue;
      posting.delete(id);
      if (posting.size === 0) {
        this.#postings.delete(token);
        this.#vocabulary = null;
      }
    }

    this.#entries.delete(id);
    this.#totalLength -= entry.length;
    return true;
  }

  clear(): void {
    this.#entries.clear();
    this.#postings.clear();
    this.#totalLength = 0;
    this.#vocabulary = null;
  }

  stats(): IndexStats {
    const documents = this.#entries.size;
    return {
      documents,
      terms: this.#postings.size,
      averageLength: documents === 0 ? 0 : this.#totalLength / documents,
    };
  }

  search(query: string, options: SearchOptions = {}): SearchHit[] {
    const limit = clamp(options.limit ?? 20, 1, 200);
    const now = options.now ?? Date.now();
    const kindFilter =
      options.kinds === undefined ? null : new Set<DocumentKind>(options.kinds);
    const entries = [...this.#entries.values()].filter(
      (entry) => kindFilter === null || kindFilter.has(entry.doc.kind),
    );

    if (entries.length === 0) return [];

    const terms = tokenizeQuery(query);
    if (terms.length === 0) {
      const trimmed = query.trim();
      if (trimmed.length > 0) return this.#subsequenceSearch(trimmed, entries, limit, now);
      return options.allowEmpty === true ? rankDefault(entries, limit, now) : [];
    }

    const averageLength = this.#averageLength();
    const documentCount = this.#entries.size;
    const prefix = options.prefix ?? true;
    const fuzzy = options.fuzzy ?? true;
    const maxExpansions = options.maxPrefixExpansions ?? 24;
    const maxFuzzyCandidates = options.maxFuzzyCandidates ?? 4000;

    interface TermMatches {
      weights: Map<string, number>;
      documentFrequency: number;
      usedFuzzy: boolean;
    }

    const perTerm: TermMatches[] = [];

    for (const term of terms) {
      const weights = new Map<string, number>();
      let usedFuzzy = false;

      const exact = this.#postings.get(term);
      if (exact !== undefined) {
        for (const [id, tf] of exact) weights.set(id, tf);
      }

      if (prefix && term.length >= 2) {
        const vocabulary = this.#vocabularyList();
        let cursor = lowerBound(vocabulary, term);
        let expansions = 0;
        while (cursor < vocabulary.length && expansions < maxExpansions) {
          const candidate = vocabulary[cursor];
          if (candidate === undefined || !candidate.startsWith(term)) break;
          if (candidate !== term) {
            const posting = this.#postings.get(candidate);
            if (posting !== undefined) {
              for (const [id, tf] of posting) {
                const discounted = tf * 0.55;
                const current = weights.get(id);
                if (current === undefined || discounted > current) weights.set(id, discounted);
              }
            }
            expansions += 1;
          }
          cursor += 1;
        }
      }

      if (fuzzy && weights.size === 0) {
        const budget = editBudget(term);
        if (budget > 0) {
          const vocabulary = this.#vocabularyList();
          const first = term[0] ?? "";
          let cursor = lowerBound(vocabulary, first);
          let scanned = 0;
          while (cursor < vocabulary.length && scanned < maxFuzzyCandidates) {
            const candidate = vocabulary[cursor];
            if (candidate === undefined || !candidate.startsWith(first)) break;
            cursor += 1;
            if (candidate.length < term.length - budget || candidate.length > term.length + budget) {
              continue;
            }
            scanned += 1;
            const distance = boundedLevenshtein(term, candidate, budget);
            if (distance > budget) continue;
            const discount = distance === 1 ? 0.45 : 0.32;
            const posting = this.#postings.get(candidate);
            if (posting === undefined) continue;
            for (const [id, tf] of posting) {
              const discounted = tf * discount;
              const current = weights.get(id);
              if (current === undefined || discounted > current) weights.set(id, discounted);
            }
            usedFuzzy = true;
          }
        }
      }

      perTerm.push({ weights, documentFrequency: weights.size, usedFuzzy });
    }

    const scored: Array<{ entry: Entry; score: number; usedFuzzy: boolean }> = [];

    for (const entry of entries) {
      let relevance = 0;
      let matchedTerms = 0;
      let usedFuzzy = false;

      for (const term of perTerm) {
        const tf = term.weights.get(entry.doc.id);
        if (tf === undefined) continue;
        matchedTerms += 1;
        if (term.usedFuzzy) usedFuzzy = true;
        relevance += bm25(
          tf,
          term.documentFrequency,
          documentCount,
          entry.length,
          averageLength,
          this.#k1,
          this.#b,
        );
      }

      // AND semantics: every query term must match. Fuzzy/prefix expansions
      // happen per term, so typos still satisfy the clause.
      if (matchedTerms !== perTerm.length) continue;

      scored.push({
        entry,
        score:
          relevance +
          KIND_PRIOR[entry.doc.kind] +
          titleBoost(entry.titleFolded, terms) +
          recencyBoost(entry.modifiedAt, now) +
          usageBoost(entry.uses),
        usedFuzzy,
      });
    }

    // Rank first; only the survivors pay for source classification and
    // highlight mapping (the dominant per-hit cost at 5k documents).
    scored.sort((a, b) => compareScored(a, b));
    return scored.slice(0, limit).map(({ entry, score, usedFuzzy }) => {
      const source = matchSource(entry, terms, usedFuzzy);
      return {
        id: entry.doc.id,
        kind: entry.doc.kind,
        title: entry.doc.title,
        ...(entry.doc.subtitle === undefined ? {} : { subtitle: entry.doc.subtitle }),
        ...(entry.doc.path === undefined ? {} : { path: entry.doc.path }),
        score,
        source,
        highlights: highlightTitle(entry.doc.title, terms, source),
      };
    });
  }

  snapshot(): SerializedIndex {
    return {
      version: 1,
      documents: [...this.#entries.values()].map((entry) => ({
        ...entry.doc,
        ...(entry.doc.tags === undefined ? {} : { tags: [...entry.doc.tags] }),
      })),
    };
  }

  static fromSnapshot(snapshot: SerializedIndex, options: SearchIndexOptions = {}): SearchIndex {
    const index = new SearchIndex(options);
    index.upsertMany(snapshot.documents);
    return index;
  }

  #averageLength(): number {
    return this.#entries.size === 0 ? 0 : this.#totalLength / this.#entries.size;
  }

  #vocabularyList(): string[] {
    if (this.#vocabulary === null) {
      this.#vocabulary = [...this.#postings.keys()].sort();
    }
    return this.#vocabulary;
  }

  /**
   * Fallback for queries that tokenize to nothing (single characters, emoji):
   * subsequence match against titles. Keeps one-keypress search responsive.
   */
  #subsequenceSearch(
    query: string,
    entries: readonly Entry[],
    limit: number,
    now: number,
  ): SearchHit[] {
    const hits: SearchHit[] = [];
    for (const entry of entries) {
      const score = subsequenceScore(query, entry.doc.title);
      if (score === null) continue;
      hits.push({
        id: entry.doc.id,
        kind: entry.doc.kind,
        title: entry.doc.title,
        ...(entry.doc.subtitle === undefined ? {} : { subtitle: entry.doc.subtitle }),
        ...(entry.doc.path === undefined ? {} : { path: entry.doc.path }),
        score: score * 3 + KIND_PRIOR[entry.doc.kind] + recencyBoost(entry.modifiedAt, now),
        source: "fuzzy",
        highlights: highlightTitle(entry.doc.title, [query], "fuzzy"),
      });
    }
    hits.sort(compareHits);
    return hits.slice(0, limit);
  }
}

export function buildIndex(
  documents: readonly IndexDocument[],
  options: SearchIndexOptions = {},
): SearchIndex {
  const index = new SearchIndex(options);
  index.upsertMany(documents);
  return index;
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

function bm25(
  tf: number,
  documentFrequency: number,
  documentCount: number,
  documentLength: number,
  averageLength: number,
  k1: number,
  b: number,
): number {
  if (documentCount === 0 || documentFrequency === 0) return 0;
  const idf = Math.log(
    1 + (documentCount - documentFrequency + 0.5) / (documentFrequency + 0.5),
  );
  const normalised = averageLength === 0 ? 1 : documentLength / averageLength;
  const saturation = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * normalised));
  return Math.max(0, idf) * saturation;
}

export function recencyBoost(modifiedAt: number | undefined, now: number): number {
  if (modifiedAt === undefined) return 0;
  const ageDays = Math.max(0, now - modifiedAt) / DAY_MS;
  return 0.35 * Math.pow(0.5, ageDays / 30);
}

export function usageBoost(uses: number): number {
  return 0.15 * Math.log1p(Math.max(0, uses));
}

function titleBoost(titleFolded: string, terms: readonly string[]): number {
  if (terms.length === 0) return 0;
  let boost = 0;
  const query = terms.join(" ");
  if (titleFolded === query) boost += 1.5;
  else if (titleFolded.startsWith(query)) boost += 0.75;
  if (terms.every((term) => titleFolded.includes(term))) boost += 0.35;
  return boost;
}

function matchSource(entry: Entry, terms: readonly string[], usedFuzzy: boolean): MatchSource {
  const query = terms.join(" ");
  if (entry.titleFolded === query) return "exact";
  for (const term of terms) {
    if (entry.titleTokens.has(term)) return "exact";
  }
  if (entry.titleFolded.startsWith(query)) return "prefix";
  for (const term of terms) {
    for (const token of entry.titleTokens) {
      if (token.startsWith(term)) return "prefix";
    }
  }
  if (usedFuzzy) return "fuzzy";
  return "fragment";
}

function compareHits(a: SearchHit, b: SearchHit): number {
  if (b.score !== a.score) return b.score - a.score;
  const titleOrder = compareText(a.title, b.title);
  if (titleOrder !== 0) return titleOrder;
  return compareText(a.id, b.id);
}

function compareScored(
  a: { entry: Entry; score: number },
  b: { entry: Entry; score: number },
): number {
  if (b.score !== a.score) return b.score - a.score;
  const titleOrder = compareText(a.entry.doc.title, b.entry.doc.title);
  if (titleOrder !== 0) return titleOrder;
  return compareText(a.entry.doc.id, b.entry.doc.id);
}

/** Code-unit comparison: deterministic and much cheaper than localeCompare. */
function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function rankDefault(entries: readonly Entry[], limit: number, now: number): SearchHit[] {
  const hits = entries.map((entry) => ({
    id: entry.doc.id,
    kind: entry.doc.kind,
    title: entry.doc.title,
    ...(entry.doc.subtitle === undefined ? {} : { subtitle: entry.doc.subtitle }),
    ...(entry.doc.path === undefined ? {} : { path: entry.doc.path }),
    score:
      KIND_PRIOR[entry.doc.kind] +
      recencyBoost(entry.modifiedAt, now) +
      usageBoost(entry.uses),
    source: "fragment" as const,
    highlights: [] as HighlightRange[],
  }));
  hits.sort(compareHits);
  return hits.slice(0, limit);
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

/**
 * Map query terms onto UTF-16 ranges in the original title. Handles exact
 * word matches, prefix matches and subsequence (fuzzy) matches.
 */
export function highlightTitle(
  title: string,
  terms: readonly string[],
  source: MatchSource,
): HighlightRange[] {
  if (terms.length === 0 || title.length === 0) return [];
  const { folded, map } = foldWithMap(title);
  const ranges: HighlightRange[] = [];

  const offsetAt = (foldedIndex: number): number =>
    foldedIndex >= map.length ? title.length : (map[foldedIndex] ?? title.length);

  for (const term of terms) {
    if (term.length === 0) continue;
    let matched = false;
    for (const match of folded.matchAll(WORD)) {
      const token = match[0];
      const start = match.index;
      if (token === undefined || start === undefined) continue;

      if (token === term) {
        ranges.push({ start: offsetAt(start), end: offsetAt(start + token.length) });
        matched = true;
        break;
      }
      if (token.startsWith(term)) {
        ranges.push({ start: offsetAt(start), end: offsetAt(start + term.length) });
        matched = true;
        break;
      }
    }
    if (!matched && source === "fuzzy") {
      const positions = subsequencePositions(term, folded);
      if (positions !== null) {
        for (const position of positions) {
          ranges.push({ start: offsetAt(position), end: offsetAt(position + 1) });
        }
        matched = true;
      }
    }
  }

  return mergeRanges(ranges);
}

/** Sort and coalesce overlapping or adjacent half-open ranges. */
export function mergeRanges(ranges: readonly HighlightRange[]): HighlightRange[] {
  if (ranges.length === 0) return [];
  const sorted = [...ranges]
    .filter((range) => range.end > range.start)
    .sort((a, b) => a.start - b.start || a.end - b.end);

  const merged: HighlightRange[] = [];
  for (const range of sorted) {
    const last = merged[merged.length - 1];
    if (last !== undefined && range.start <= last.end) {
      last.end = Math.max(last.end, range.end);
    } else {
      merged.push({ start: range.start, end: range.end });
    }
  }
  return merged;
}
