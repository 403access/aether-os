---
id: index-daemon
title: Index daemon (aether-index)
sidebar_position: 12
description: Rust inotify index daemon with Unix-socket queries and a file-backed change ring.
---

# Index daemon (`aether-index`)

P3 promotes the Bun query service to a Linux daemon. `aether-index` scans the
configured roots, keeps the index fresh with inotify and answers palette queries
over a Unix socket.

## Process

```text
roots -> recursive scan -> in-memory index
                            |  ^
             inotify events -+  | socket queries
                            v  |
                    file-backed event ring
```

- **Scan:** depth- and count-bounded; skips dotfiles, `node_modules`, `target`,
  `.git`, `__pycache__`, `.cache`, `.direnv`.
- **Live updates:** recursive inotify watches for create, close-write, move,
  delete and attribute changes. New directories are watched and scanned.
- **Queries:** newline-delimited JSON on `AETHER_INDEX_SOCKET`:
  ```json
  {"query":"notes","limit":8}
  {"results":[{"title":"notes.md","subtitle":"/home/u/notes  ·  md","path":"/home/u/notes.md"}]}
  ```
- **Ring:** when `AETHER_INDEX_RING` is set, every change is appended to a
  memory-mapped event file. `aether-index --ring-tail <path>` prints it for
  operations and tests.

## Ring layout

```text
header: magic[8] "AETHERIX", version u64, head u64, dropped u64
record: total u32, kind u8 ('i' indexed / 'd' removed), reserved[3],
        path bytes, zero padding to 8 bytes
```

A zero `total` is a wrap marker; readers jump to offset 32. The writer drops an
event that cannot fit and increments `dropped`.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `AETHER_INDEX_ROOTS` | `$HOME` | colon-separated scan roots |
| `AETHER_INDEX_LIMIT` | `20000` | maximum documents |
| `AETHER_INDEX_DEPTH` | `12` | scan depth |
| `AETHER_INDEX_SOCKET` | `$XDG_RUNTIME_DIR/aether-index.sock` | query socket |
| `AETHER_INDEX_RING` | unset | event ring path |
| `AETHER_INDEX_WATCH` | `1` | set `0` to disable inotify |
| `AETHER_INDEX_DELAY_MS` | `0` | test hook: delay each response |

## Systemd

`systemd.user.services.aether-index` runs the daemon with the user's home as the
root, `%t/aether-index.sock` and `%t/aether-index.ring`. The compositor's index
worker queries the socket off-thread and discards stale generations.

The Bun `apps/indexer-service` remains in the tree for the reference indexer
tests and the marketing demo; the installed session uses `aether-index`.

## Memory

The index stores one `Document` per file: path, title, subtitle, a pre-folded
lowercase title and a lowercase path for matching. At the 20k limit that is
roughly 8–10 MiB. Titles are folded once at insert rather than per query, and a
query selects top-k with `select_nth_unstable_by` before sorting only the page,
so a keystroke does not allocate once per document. The daemon still opens one
connection per query; persistent connections are tracked in the
[memory scenarios](/roadmap/memory-scenarios).

Budget definition, harness and the rest of the session footprint:
[memory footprint](/architecture/memory-footprint).
