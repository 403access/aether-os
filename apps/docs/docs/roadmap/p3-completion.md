---
id: p3-completion
title: P3 completion plan
sidebar_position: 3
description: Shell-surface tasks for P3 with acceptance criteria, evidence and commit boundaries.
---

# P3 completion plan

**Goal.** Palette, chat and explorer become compositor layers; the index daemon
ingests changes; the shell keeps 120 Hz under streaming and 10k-row workloads.

Each task is a separate commit and updates this ledger. A task is done only when
its acceptance check is demonstrated and its evidence is committed.

## Gate

- Palette p95 **< 10 ms** at 10k documents.
- Explorer scrolls **10k rows at 120 Hz**.
- **No dropped frames** while chat tokens stream.

## Tasks

| ID | Task | Acceptance | Evidence |
| --- | --- | --- | --- |
| P3-T1 | `aether-index` Rust daemon: recursive scan, inotify ingestion, Unix-socket queries, file-backed event ring | Live create/modify/remove updates search results; ring reader sees the same events | daemon tests + `artifacts/p3/index.log` |
| P3-T2 | Palette parity and scale: prefix modes (`>`, `@`, `/`, `~`), subsequence scoring, shared TS/Rust fixture, 10k benchmark | TS and Rust rank the fixture identically; 10k p95 < 10 ms | `packages/palette/fixtures/parity.json`, `artifacts/p3/palette.json` |
| P3-T3 | Explorer layer: virtualised file list, keyboard navigation, scrollbar | 10k rows; only visible rows enter the render plan; scroll step p95 < 1 ms | shell tests + `artifacts/p3/explorer.json` + screenshot |
| P3-T4 | Chat drawer: streaming markdown (headings, lists, inline code/bold), autoscroll | 1k streamed tokens render incrementally without unbounded line work | chat tests + stream smoke |
| P3-T5 | Keyboard map and MRU focus stack | One keymap table drives shortcuts; `Super+Tab` cycles MRU focus | keymap tests + docs |
| P3-T6 | Design parity, status-bar audit and screenshots | Row height/colors match the design tokens; palette/explorer/chat smokes capture frames | `scripts/design-parity.ts`, screenshots, docs |

## Sequencing

P3-T1 → P3-T2 → P3-T3 → P3-T4 → P3-T5 → P3-T6. T3/T4 depend on the shell state
introduced by T2 only for scoring reuse; they can land independently.

## Non-goals

- Replacing the GLES renderer.
- A full HTML/CSS design system in the compositor.
- Network-backed chat; the drawer consumes a local token stream.

## Execution ledger

| Task | Status | Commit |
| --- | --- | --- |
| Plan | ✅ | this document |
| P3-T1 | ⏳ | |
| P3-T2 | ⏳ | |
| P3-T3 | ⏳ | |
| P3-T4 | ⏳ | |
| P3-T5 | ⏳ | |
| P3-T6 | ⏳ | |
