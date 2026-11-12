---
id: files-and-storage
title: Files and storage
sidebar_position: 13
description: XFS root, ext4 scratch, in-memory index, two-pane explorer semantics and integrity rules.
---

# Files and storage

Storage choices are made for one NVMe drive and one workstation workload: many small
files, frequent metadata churn, and an index that lives in RAM.

## Why XFS

| Filesystem | Verdict | Reason |
| --- | --- | --- |
| **XFS** | **chosen for root** | Extent allocation, dynamic inodes, strong parallel I/O, low CPU |
| ext4 | chosen for `/scratch` | Small-file create/delete throughput; legacy tools |
| Btrfs | rejected | Copy-on-write write amplification and fragmentation on SQLite/index churn |
| ZFS | rejected | ARC memory conflicts with the 200 MB idle budget; kernel module tax |
| tmpfs | chosen for `/var/index` | The index is a rebuildable cache |

XFS cannot shrink, which is irrelevant for a statically partitioned personal machine.
It handles large parallel writes and metadata-heavy trees without tail-latency spikes.

## Layout

| Mount | FS | Size (reference) | Options |
| --- | --- | --- | --- |
| `/` | XFS | 80 GB | `noatime,largeio,inode64` |
| `/nix` | XFS | 120 GB | `noatime` |
| `/home` | XFS | remainder | `noatime` |
| `/scratch` | ext4 | 60 GB | `noatime,commit=60,data=writeback` |
| `/var/index` | tmpfs | 2 GB | `size=2G,mode=0700` |
| `/mnt/archive` | external | — | read-only by default |

Backups are explicit. There is no snapshots-on-write story; the archive pane can
point at a NAS or an external drive.

## I/O scheduling

NVMe has hardware queues; software scheduling is mostly overhead:

```bash
# udev rule
ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/scheduler}="none"
ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/nr_requests}="1024"
```

`none` is the default for NVMe on modern kernels. `bfq` is a fallback if
interactive latency suffers under heavy writes; the choice is measured, not assumed.

## Explorer model

The file manager is a **two-root comparison tool**, not a tree:

```ts
type ExplorerEntry = {
  id: string;
  name: string;
  type: "folder" | "file";
  status?: { label: string; tone: "same" | "moved" | "partial" | "missing" | "local" };
  size?: string;
  modified?: string;
};
```

Status classes:

| Status | Meaning |
| --- | --- |
| Same | Exists in both roots with identical size and mtime |
| Moved | Exists in the other root under a different path (matched by hash) |
| Partial | Present but different size, mtime or checksum |
| Missing | Absent in the other root |
| Local only | No counterpart root mounted |

Colour is never the only signal; the badge carries a label and a dot.

## Operations

| Operation | Semantics |
| --- | --- |
| Open | Folders navigate; files open in the owning application |
| Copy | Streamed with `copy_file_range`; progress in the status bar |
| Move | `renameat2` within a filesystem; copy+unlink across |
| Rename | Atomic within the filesystem; refuses to clobber |
| Delete | Moves to a per-root trash directory, not `unlink` |
| Reveal | Opens the parent and selects the entry |

Every operation reports bytes and duration. Long operations are cancellable and
resumable within the session.

## Index integration

The index daemon watches roots with inotify (fanotify for large trees where
available). Events are debounced (50 ms) and coalesced by path:

```text
create/modify → upsert (read up to 20k chars for text extensions)
delete        → remove
move          → remove old id, upsert new id
overflow      → schedule a rescan of the affected root
```

`packages/indexer/src/fs.ts` is the reference ingestion policy. It skips dotfiles and
build directories by default (`node_modules`, `.git`, `dist`, `target`, …) and never
indexes the body of a binary file.

## Integrity

- `noatime` and `nodiratime` avoid metadata writes on reads.
- `commit=60` on scratch trades up to 60 s of writes for throughput; scratch is
  disposable by definition.
- Root is not `data=writeback`; correctness wins on the system volume.
- Checksums are not built into XFS. Archive comparisons hash files explicitly
  (BLAKE3) when a user requests verification.
- Trash retention is 7 days by default, configurable.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Disk full | Writes fail with a clear error; index tmpfs evicts snapshots |
| XFS corruption | Boot into rescue generation; `xfs_repair` |
| Power loss | Journal replay; scratch may lose up to 60 s by design |
| Moved file during copy | Operation aborts and reports the path |
| Watcher overflow | Rescan; status shows “reindexing” |
| Archive unmounted | Pane shows empty state with “root unavailable” |

## Verification

| Test | Phase |
| --- | --- |
| Path classification and binary sniffing | ✅ unit + integration now |
| Index reflects a real temp directory | ✅ integration now |
| copy/move/rename semantics | P3 |
| Watcher overflow recovery | P3 |
| Disk-full behaviour | P3 |
| XFS mount options in the flake | P1 |
