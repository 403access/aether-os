---
id: explorer
title: Explorer
sidebar_position: 2
description: "The two-pane file manager: data model, status semantics, interactions and implementation."
---

# Explorer

The explorer is a **two-root comparison tool**. It is not a tree, not a spatial
file manager, and not a clone of Finder.

Reference implementation and live baseline:
`/Users/olivermolnar/Developer/jottacloud-comparison/app` → `http://localhost:8787/`.

## Why two panes

The operator's real question is rarely “what is in this folder?” It is “is this
folder the same as that one?” Two explicit roots side by side answer that without
navigation, breadcrumb archaeology or a diff dialog.

| Pane | Typical root | Mode |
| --- | --- | --- |
| Left | `/home/oliver` | read/write, indexed, watched |
| Right | `/mnt/archive` | read-only by default, manifest-backed |

Both panes share the same component, keyboard model and status vocabulary.

## Data model

```ts
type ExplorerEntry = {
  id: string;
  name: string;
  type: "folder" | "file";
  status?: { label: string; tone: BadgeTone };
  size?: string;
  modified?: string;
};
```

Status is optional: a local-only pane does not pretend to have a counterpart status.
When a comparison exists, the daemon supplies it; the UI never computes “same” from a
filename.

## Status semantics

| Status | Tone | Meaning |
| --- | --- | --- |
| Same | `same` (green) | Same size and mtime; or same hash when verified |
| Moved | `moved` (violet) | Exists at another path; matched by hash |
| Partial | `partial` (amber) | Present but differs in size, mtime or hash |
| Missing | `missing` (red) | Absent in the counterpart root |
| Local only | `local` (blue) | No counterpart root mounted |

Badges always carry a label and a dot. A greyscale screenshot remains readable.

## Layout

The compositor explorer is a **full-output workspace**, not a floating dialog:

```text
┌──────────────────────────────────────────────────────────────┐
│ ◉ Aether          Files  Palette  Chat        ⌕ Search  ⌘K  │ 62 px app bar
│   Files · indexed system                                     │
├──────────────────────────────────────────────────────────────┤
│  ▣ Files                                    2 files      ×  │ 56 px pane header
│    All indexed roots                                         │
│  NAME                         TYPE              LOCATION     │ 30 px column header
│  ▌ alpha.md                   MD                   notes     │ 30 px rows
│    beta.txt                   TXT                   file      │
│                                                              │
│  2 of 2 files                  Enter open  Esc close  PgUp…  │ 30 px footer
├──────────────────────────────────────────────────────────────┤
│ alpha.md  /srv/…    0 agents  12%  1.7/12 GB  audio  0.0 ms  │ 36 px status bar
└──────────────────────────────────────────────────────────────┘
```

![Explorer full-output layout](/img/smithay-explorer.png)

![Explorer filtered to one file](/img/smithay-explorer-filtered.png)

- 30 px rows, sticky column header, 56 px pane header, 16 px pane inset.
- App bar tabs switch Files / Palette / Chat without losing the explorer query.
- Path is read-only in the pane; editing paths is a palette command, not a text field.
- Footer shows the visible count and pane shortcuts; the shell status bar spans the output.
- Type scale is Inter 11/13/16/20 px; icons are the Water SVG set, not emoji.

## Interactions

| Input | Action |
| --- | --- |
| Click | Select |
| Double-click folder | Open (push path) |
| Double-click file | Open in owning application |
| `↑` / `↓` | Move selection |
| `Enter` | Open selection |
| `Backspace` | Edit the filter (delete a character) |
| Type | Filter current pane |
| `⌘K` | Palette (files mode preselected by context) |
| Drag | Copy/move with a modifier; explicit drop target |

Selection is per pane. Only one pane is “active”; the active pane receives keys and
shows a brighter border.

## Opening files

Folders navigate within the pane. Files are opened by the shell's default handler:

```text
file → mime type → handler registry → capability check → launch sandboxed client
```

An unknown type opens a metadata surface with “open with…” — never an arbitrary
executable by default.

## Search

- Typing filters the current pane's rows (case-insensitive substring on name).
- The palette searches file **content** through the index; the pane's filter does not.
- This separation is intentional: the pane stays instant even when the index is
  rebuilding.

## Implementation

| Concern | Code |
| --- | --- |
| Web pane component | `packages/ui/src/Explorer.tsx` |
| Compositor explorer layer | `os/compositor/aether-shell-wayland/src/render/surfaces/explorer.rs` |
| Compositor geometry | `os/compositor/crates/aether-shell/src/shell/explorer.rs` (`explorer_*`) |
| Fixture data (demo) | `apps/web/src/data.ts` |
| Selection/open/search tests | `packages/ui/src/ui.test.tsx` |
| Real filesystem semantics | `apps/docs/docs/architecture/files-and-storage.md` |
| Reference app | `jottacloud-comparison/app/src/web/components/Pane.tsx` |

The React component is presentational. Data loading, navigation history and
comparison logic live in the host (the `Super+E` shell layer).

## Known gaps vs. the reference

| Gap | Plan |
| --- | --- |
| Column customisation (show/hide/resize) | Future, reusing the `columns` model |
| Analytics sidebar | Not applicable; no telemetry |
| Multi-select and bulk operations | Future (P4 explorer operations) |
| Drag-and-drop between panes | P6 |
| Virtualised 10k-row rendering | ✅ P3 shell explorer |
| Provider switching (Jottacloud/Google Drive) | Not applicable; roots are configured locally |

## Testing

- DOM tests cover selection, double-click open, search input and empty state.
- The marketing demo exercises navigation between fixture folders.
- Playwright asserts the explorer is interactive on the built site.
- Real filesystem tests live in `packages/indexer/src/fs.test.ts` (temp directory,
  ignored dirs, binary sniffing).
