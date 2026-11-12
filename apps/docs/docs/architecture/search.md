---
id: search
title: Search and indexing
sidebar_position: 7
description: Tokenizer, BM25F scoring, prefix expansion, fuzzy matching, ranking boosts and measured performance.
---

# Search and indexing

`aether-index` is an in-memory search engine purpose-built for the palette. It
indexes applications, files, extracted content, commands, agents and settings, and
answers in single-digit milliseconds at ten thousand documents.

The reference implementation is `packages/indexer/src/search-index.ts`.

## Pipeline

```text
documents → tokenizer → weighted terms → inverted index (postings)
query     → tokenizer → term matching → BM25F + boosts → rank → top-k → highlights
```

## Tokenization

Rules that matter for a machine full of code and filenames:

| Input | Tokens |
| --- | --- |
| `getUserProfile` | `get`, `user`, `profile` |
| `HTTPRequest` | `http`, `request` |
| `snake_case-name` | `snake`, `case`, `name` |
| `/work/aether/README.md` | `work`, `aether`, `readme`, `md` |
| `Café Zürich` | `cafe`, `zurich` |
| `日本語` | `日本語` |

Implementation details:

- NFKD normalisation, combining marks removed, lowercased.
- Camel and acronym boundaries split before folding.
- Tokens shorter than two characters are dropped (`a`, `x` are noise). Queries that
  tokenize to nothing fall back to subsequence search over titles.
- Every folded character keeps a map back to its UTF-16 offset, so highlight ranges
  point at the original string even when folding changes length (`é` → `e`).

## Field-weighted terms (BM25F-lite)

| Field | Weight |
| --- | --- |
| Title | 5 |
| Tags | 3 |
| Subtitle / path | 2 |
| Body | 1 |

Term frequency is the sum of weighted occurrences; document length is the sum of
weighted tokens. This keeps one BM25 pass while letting a title match dominate a
body mention.

## Scoring

For each query term with document frequency `df`, document count `N`, weighted
frequency `tf` and normalised length `dl/avgdl`:

```text
idf = ln(1 + (N - df + 0.5) / (df + 0.5))
score = idf * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl / avgdl))
```

Defaults: `k1 = 1.2`, `b = 0.75`.

Then additive boosts:

| Boost | Value |
| --- | --- |
| Kind prior (command, app, agent, file, setting…) | 0.05–0.35 |
| Exact title | +1.5 |
| Title starts with query | +0.75 |
| All terms present in title | +0.35 |
| Recency (`0.35 · 0.5^(ageDays/30)`) | 0–0.35 |
| Usage (`0.15 · ln(1 + uses)`) | 0+ |

The prior table lives in `packages/protocol` so the compositor, docs and tests
share one definition.

## Prefix expansion

If `prefix` is enabled (default), the last token expands against the sorted
vocabulary:

```text
"sett" → settings, setting, setter, ...
```

- Up to 24 expansions per term (configurable).
- Expanded postings are discounted to 55% of their term frequency.
- Expansion happens only for terms of two or more characters.

This is what makes one-keystroke-later search feel instant without a separate
autocomplete service.

## Fuzzy matching

When a term has no exact or prefix matches, bounded Levenshtein runs against
vocabulary entries sharing the first character:

| Term length | Edit budget |
| --- | --- |
| ≤ 3 | 0 |
| 4–5 | 1 |
| ≥ 6 | 2 |

- Discount: 45% for distance 1, 32% for distance 2.
- Candidate scan is capped (4,000 by default).
- Only the first character bucket is scanned; a typo in the first character will
  not match. This is a documented limitation, not a bug. A trigram index is the
  planned fix if measurements justify it.

## AND semantics

Multi-term queries require every term to match (after prefix/fuzzy expansion). This
is deliberate: `compositor audio` should not return every document that mentions
either word. Term contributions are summed, so a document matching both ranks above
one matching either.

## Highlights

Only the top `limit` results pay for highlight computation — a 4× improvement in
query p50 during development. Ranges are:

- exact token → whole token
- prefix → only the typed prefix
- fuzzy → matched character positions, merged when adjacent

Ranges are sorted and coalesced, and always slice correctly from the original title.

## Complexity

| Operation | Complexity |
| --- | --- |
| Upsert | O(tokens in document) |
| Remove | O(unique tokens) |
| Query | O(terms × postings + vocabulary scan for prefix/fuzzy + n log n sort) |
| Highlight | O(results × title length) |

The full result set is sorted today. A top-k heap is a P3 optimisation if 10k
documents push p95 above budget.

## API

```ts
const index = buildIndex(documents, { k1: 1.2, b: 0.75 });

index.search("sett", {
  limit: 12,
  kinds: ["setting", "command"],
  prefix: true,
  fuzzy: true,
  now: Date.now(), // injectable for deterministic tests
});

index.upsert(document);
index.remove(id);
index.snapshot(); // JSON-serialisable
SearchIndex.fromSnapshot(snapshot);
```

All ranking inputs are explicit; there is no hidden global clock. That is why every
ordering assertion in the test suite is stable.

## Measured performance

Reference host (Apple M3, Bun 1.4), 5,000 documents, queries that match every
document:

| Metric | Value |
| --- | --- |
| Build | 80 ms |
| Query p50 | 1.65 ms |
| Query p95 | 2.23 ms |
| Palette p50 (1k items) | 0.78 ms |

The target machine is slower per core; the P1 budget for 10k documents is p95
< 10 ms. The committed benchmark already catches relative regressions.

## Persistence

The daemon persists a snapshot, not a write-ahead log:

- On shutdown or every N minutes, write a JSON snapshot atomically.
- On boot, load the snapshot and replay inotify events since the snapshot mtime.
- If the snapshot is corrupt, discard it and rebuild from disk without blocking the
  shell; the status bar shows “index rebuilding”.

SQLite FTS5 was considered for persistence. It is a good store but a worse hot path:
the palette needs the index in memory anyway, and keeping one copy avoids a second
serialization boundary. See [ADR-0005](/adr/0005-shared-memory-ipc) for the
shared-memory reasoning.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Watcher overflow | Full rescan of the affected root; status shows progress |
| Snapshot corrupt | Discard, rebuild; no crash |
| Body too large | Truncated at 20k characters at ingest |
| Binary file | Indexed by name/path only; body skipped |
| Unicode folding mismatch | Highlight falls back to the nearest mapped range; never panics |
| Vocabulary explosion | Prefix/fuzzy caps bound the scan; results remain sorted |

## Tuning knobs

| Option | Default | Effect |
| --- | --- | --- |
| `k1` | 1.2 | Term saturation |
| `b` | 0.75 | Length normalisation |
| `maxBodyChars` | 20,000 | Memory bound per document |
| `maxPrefixExpansions` | 24 | Recall vs. scan cost |
| `maxFuzzyCandidates` | 4,000 | Typo tolerance vs. latency |
| `limit` | 20 | Results returned |

Changing a default requires a benchmark note and a test update.
