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

The service watches the roots through `packages/indexer/src/watch.ts`, which uses
the platform watcher (`inotify` on Linux) with per-path debouncing and upserts or
removes files as they change (set `AETHER_INDEX_WATCH=0` to disable). The compositor falls back to
`aether-files` when `AETHER_INDEX_SOCKET` is unset.

## NixOS

The flake packages it as `.#index-service` (standalone Bun binary). The VM
target runs it as a user service:

```nix
systemd.user.services.aether-index = {
  wantedBy = ["default.target"];
  serviceConfig = {
    ExecStart = "${pkgs.aether-index-service}/bin/aether-index-service";
    Environment = [
      "AETHER_INDEX_SOCKET=%t/aether-index.sock"
      "AETHER_INDEX_ROOTS=%h"
    ];
    Restart = "on-failure";
  };
};
```

Point the compositor's `AETHER_INDEX_SOCKET` at the same `%t` path.
