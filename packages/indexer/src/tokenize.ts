/**
 * Tokenization for the indexer.
 *
 * Requirements that shaped this:
 *  - Code and filenames matter: `getUserProfile`, `snake_case`, `kebab-case`
 *    and `path/to/file.ts` must all produce useful tokens.
 *  - Unicode folding must be accent-insensitive ("Café" ~ "cafe").
 *  - Highlight ranges must map back to UTF-16 offsets in the *original* string,
 *    including when folding changes length (NFKD can expand a code point).
 */

export interface FoldedText {
  folded: string;
  /** map[i] = UTF-16 offset in the original string for folded char i. */
  map: number[];
}

const MARKS = /\p{M}+/gu;
const WORD = /[\p{L}\p{N}]+/gu;
const CAMEL_BOUNDARY = /([\p{Ll}\p{N}])(\p{Lu})/gu;
const ACRONYM_BOUNDARY = /(\p{Lu}+)(\p{Lu}\p{Ll})/gu;

/**
 * Case-fold and strip diacritics while retaining a position map. Folding is
 * per-code-point so that the map stays correct for astral characters.
 */
export function foldWithMap(input: string): FoldedText {
  let folded = "";
  const map: number[] = [];
  let offset = 0;

  for (const char of input) {
    const base = char.normalize("NFKD").replace(MARKS, "").toLowerCase();
    for (const out of base) {
      folded += out;
      map.push(offset);
    }
    offset += char.length;
  }

  return { folded, map };
}

export function fold(input: string): string {
  return foldWithMap(input).folded;
}

/**
 * Split identifiers on case and punctuation, fold, and keep tokens of at least
 * two characters. Single characters are noise in an index this small; the
 * palette still matches them through subsequence search on titles.
 */
export function tokenizeText(input: string): string[] {
  const spaced = input
    .replace(CAMEL_BOUNDARY, "$1 $2")
    .replace(ACRONYM_BOUNDARY, "$1 $2");
  const { folded } = foldWithMap(spaced);
  const tokens: string[] = [];
  for (const match of folded.matchAll(WORD)) {
    const token = match[0];
    if (token !== undefined && token.length >= 2) tokens.push(token);
  }
  return tokens;
}

export function tokenizeQuery(input: string): string[] {
  return tokenizeText(input);
}

/** Field weights for BM25F-lite: title dominates, body is evidence. */
export const FIELD_WEIGHT = {
  title: 5,
  tags: 3,
  subtitle: 2,
  body: 1,
} as const;

export interface WeightedTerms {
  /** token -> weighted term frequency across all fields */
  frequencies: Map<string, number>;
  /** Sum of weighted frequencies; used for BM25 length normalisation. */
  length: number;
  titleTokens: string[];
}

/**
 * Build weighted term frequencies for one document. The body is capped by the
 * caller (see `SearchIndexOptions.maxBodyChars`) to keep memory bounded.
 */
export function weightTerms(
  title: string,
  subtitle: string | undefined,
  tags: readonly string[] | undefined,
  body: string | undefined,
): WeightedTerms {
  const frequencies = new Map<string, number>();
  let length = 0;

  const add = (tokens: readonly string[], weight: number): void => {
    for (const token of tokens) {
      frequencies.set(token, (frequencies.get(token) ?? 0) + weight);
      length += weight;
    }
  };

  const titleTokens = tokenizeText(title);
  add(titleTokens, FIELD_WEIGHT.title);
  if (subtitle) add(tokenizeText(subtitle), FIELD_WEIGHT.subtitle);
  if (tags) add(tokensOfList(tags), FIELD_WEIGHT.tags);
  if (body) add(tokenizeText(body), FIELD_WEIGHT.body);

  return { frequencies, length, titleTokens };
}

function tokensOfList(values: readonly string[]): string[] {
  const tokens: string[] = [];
  for (const value of values) tokens.push(...tokenizeText(value));
  return tokens;
}

/**
 * Subsequence match in the style of fuzzy finders, with bonuses for contiguous
 * runs and word boundaries. Returns a score in roughly [0, 1], or null.
 */
export function subsequenceScore(query: string, target: string): number | null {
  if (query.length === 0) return null;
  const foldedQuery = fold(query);
  const foldedTarget = fold(target);
  if (foldedQuery.length > foldedTarget.length) return null;

  let score = 0;
  let targetIndex = 0;
  let previousMatch = -2;

  for (const char of foldedQuery) {
    let found = -1;
    while (targetIndex < foldedTarget.length) {
      if (foldedTarget[targetIndex] === char) {
        found = targetIndex;
        break;
      }
      targetIndex += 1;
    }
    if (found === -1) return null;

    const boundary = found === 0 || /[\s/_\-.:]/.test(foldedTarget[found - 1] ?? "");
    const contiguous = found === previousMatch + 1;
    score += 1 + (boundary ? 0.8 : 0) + (contiguous ? 0.5 : 0);
    previousMatch = found;
    targetIndex = found + 1;
  }

  const gaps = foldedTarget.length - foldedQuery.length;
  const normalized = Math.max(0, score - gaps * 0.02) / (foldedQuery.length * 1.5);
  return Math.min(1, normalized);
}

/** Character positions of a subsequence match, or null when not a match. */
export function subsequencePositions(query: string, target: string): number[] | null {
  const foldedQuery = fold(query);
  const foldedTarget = fold(target);
  const positions: number[] = [];
  let targetIndex = 0;

  for (const char of foldedQuery) {
    let found = -1;
    while (targetIndex < foldedTarget.length) {
      if (foldedTarget[targetIndex] === char) {
        found = targetIndex;
        break;
      }
      targetIndex += 1;
    }
    if (found === -1) return null;
    positions.push(found);
    targetIndex = found + 1;
  }
  return positions;
}
