---
id: 0003-xfs-root
title: ADR-0003 — XFS root, ext4 scratch
sidebar_position: 4
---

# ADR-0003 — XFS root, ext4 scratch

**Status:** Accepted · **Date:** 2026-11-11

## Context

The workstation has one NVMe drive, a frequently updated code tree, an in-memory
index and a disposable scratch area. The filesystem must not add memory pressure or
copy-on-write write amplification.

## Decision

XFS for `/`, `/nix` and `/home` with `noatime,largeio,inode64`. ext4 for `/scratch`
with `noatime,commit=60,data=writeback`. tmpfs for `/var/index`. No Btrfs, no ZFS.

## Consequences

- Strong parallel I/O and metadata performance without CoW overhead.
- XFS cannot shrink; partitions are sized statically.
- No built-in checksums; archive verification hashes explicitly.
- `commit=60` on scratch can lose up to 60 seconds of disposable data on power loss.

## Alternatives

- **Btrfs** — snapshots and subvolumes, but CoW fragmentation and write amplification
  under database/index churn. Rejected.
- **ZFS** — excellent integrity, but the ARC consumes memory and the module adds
  kernel complexity; conflicts with the idle budget. Rejected.
- **ext4 everywhere** — viable, but XFS is better for large parallel writes and
  dynamic inode allocation. ext4 retained for scratch.
