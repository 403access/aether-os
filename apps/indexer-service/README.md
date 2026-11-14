# Aether index service

A small Bun service that exposes the TypeScript `@aether/indexer` over a Unix
socket for the compositor palette.

```bash
AETHER_INDEX_ROOTS="$HOME" \
AETHER_INDEX_SOCKET="${XDG_RUNTIME_DIR:-/tmp}/aether-index.sock" \
bun run apps/indexer-service/src/main.ts
```

Environment:

| Variable | Default | Meaning |
| --- | --- | --- |
| `AETHER_INDEX_ROOTS` | `$HOME` | Colon-separated roots to scan |
| `AETHER_INDEX_SOCKET` | `$XDG_RUNTIME_DIR/aether-index.sock` | Unix socket |
| `AETHER_INDEX_LIMIT` | `20000` | Max files per root |
| `AETHER_INDEX_DEPTH` | `12` | Max directory depth |

Protocol: newline-delimited JSON. Request `{"query":"...","limit":8}`, response
`{"results":[{"title","subtitle","path"}]}`. Empty queries return no results.

The scan is a startup snapshot; incremental updates and inotify are future work.
The compositor falls back to `aether-files` when `AETHER_INDEX_SOCKET` is unset.
