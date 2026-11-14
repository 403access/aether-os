import { SearchIndex } from "../../../packages/indexer/src/search-index";
import type { IndexDocument } from "../../../packages/protocol/src/index";

export interface IndexerResult {
  path: string;
  title: string;
  subtitle?: string;
}

export interface IndexerResponse {
  results: IndexerResult[];
  error?: string;
}

/** Build a search index from documents produced by the filesystem scanner. */
export function buildIndex(documents: readonly IndexDocument[]): SearchIndex {
  const index = new SearchIndex();
  for (const document of documents) {
    index.upsert(document);
  }
  return index;
}

/**
 * Handle one newline-delimited JSON request and return a JSON response line.
 * Requests are intentionally tiny: `{ "query": "...", "limit": 8 }`.
 */
export function handleRequest(index: SearchIndex, line: string): string {
  let request: unknown;
  try {
    request = JSON.parse(line);
  } catch {
    return JSON.stringify({ results: [], error: "invalid json" } satisfies IndexerResponse);
  }
  const record =
    typeof request === "object" && request !== null
      ? (request as Record<string, unknown>)
      : {};
  const query = typeof record.query === "string" ? record.query : "";
  const limitValue = typeof record.limit === "number" ? record.limit : 8;
  const limit = Math.max(1, Math.min(Math.trunc(limitValue), 32));
  if (query.trim().length === 0) {
    return JSON.stringify({ results: [] } satisfies IndexerResponse);
  }

  const results = index
    .search(query, { limit, kinds: ["file"] })
    .flatMap((hit) =>
      hit.path === undefined
        ? []
        : [{ path: hit.path, title: hit.title, subtitle: hit.subtitle }],
    );
  return JSON.stringify({ results } satisfies IndexerResponse);
}
