---
id: 0015-index-service-socket
title: ADR-0015 — Bun index service over a Unix socket
sidebar_position: 16
---

# ADR-0015 — Bun index service over a Unix socket

**Status:** Accepted · **Date:** 2026-11-14

## Context

The system index (`packages/indexer`) is TypeScript: BM25, prefix and fuzzy
matching with filesystem ingestion. The compositor is Rust. The palette needs
content search, not just the compositor's bounded filename scan.

## Decision

Run the indexer as a small Bun service and query it from the compositor over a
Unix socket:

- startup scan of `AETHER_INDEX_ROOTS` (default `$HOME`);
- newline-delimited JSON requests/responses;
- per-query synchronous client with a short timeout;
- results are pre-filtered by the service and shown as-is by the shell;
- fall back to `aether-files` when `AETHER_INDEX_SOCKET` is unset.

## Consequences

- Content search reuses the tested TypeScript ranking instead of duplicating it
  in Rust.
- The socket protocol is easy to inspect and test without FFI.
- The service is a startup snapshot; incremental updates, inotify and live
  reindexing are future work.
- Query latency is a socket round trip; the client timeout keeps the palette
  responsive but drops results if the service stalls.
- The compositor still ships a bounded local scanner for environments without
  the service.

## Alternatives

- **FFI/embedded Rust index** — would duplicate ranking and ingestion work.
- **HTTP/TCP** — more exposure and configuration than a per-user Unix socket.
- **Index only in the compositor** — loses body-content ranking and the shared
  TypeScript implementation.
