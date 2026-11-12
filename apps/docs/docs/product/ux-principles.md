---
id: ux-principles
title: UX principles
sidebar_position: 4
description: The interaction rules that keep Aether fast, dense and legible.
---

# UX principles

The interface is a workstation, not a destination. These rules resolve design
arguments before they start.

## 1. One surface, one shortcut

The compositor owns the palette, chat, explorer chrome and status bar. There is no
“app switcher plus launcher plus spotlight plus settings search.” `⌘K` opens one
surface, and that surface can reach everything.

**Consequence:** no duplicate search boxes in the file manager, browser or settings.
Those views delegate to the palette engine.

## 2. Keyboard first, pointer never required

Every action has a key path. The pointer is an accelerator, not a requirement.

| Context | Keys |
| --- | --- |
| Global | `⌘K` palette, `⌘Tab` surface cycle, `⌘1..9` workspaces |
| Palette | arrows, `PageUp/Down`, `Home/End`, `Enter`, `Esc` |
| Explorer | arrows, `Enter` open, `Backspace` up, type-to-filter |
| Chat | `Enter` send, `Shift+Enter` newline, `Esc` stop stream |
| Media | `Space` play/pause, arrows seek, `F` fullscreen |

**Consequence:** focus is always visible, and focus is never trapped in a web view.

## 3. Density is a feature

13 px base type, 30 px rows, 62 px chrome. A file list shows 25 rows, not 12.
Whitespace is used to group, not to decorate.

**Consequence:** no marketing-sized headings inside the shell; no card grids where
a table works.

## 4. Latency is visible and honest

The UI never shows a spinner for local work. If an operation takes longer than one
frame, it shows progress with a measured number (“indexing 4,812 files”), not an
indeterminate animation.

- Palette results update in the same event loop tick.
- Agents show elapsed time and token count, not a pulsing dot.
- Remote display shows glass-to-glass latency when it exceeds the budget.

## 5. Colour never carries meaning alone

Status is always a colour **plus** a label plus a dot. This is inherited from the
Water design system and enforced in the UI kit.

**Consequence:** `Partial` is readable in greyscale and for colour-blind users.

## 6. Motion explains, never entertains

Interface motion is 160–340 ms and only communicates origin, destination or state.
Ambient motion (water caustics, slow waves) is decorative, slow, and fully
disableable. `prefers-reduced-motion` disables all of it.

## 7. Every claim has a source

Numbers in the UI are either measured live, a budget from the ledger, or absent.
No “blazing fast” copy. The status bar says `1.65 ms` or nothing.

## 8. Failure is explicit

- A denied capability names the capability and the resource.
- A blocked isolation plan shows the blocker, not “something went wrong”.
- A crashed browser tab says which tab and offers reload; the shell stays up.
- The index reports staleness when the watcher falls behind.

## 9. No hidden state

Agent state, audio routes, mounted roots and index coverage are inspectable from
the palette (`~` mode) and the status bar. If the system is doing something
expensive, the user can see it and stop it.

## 10. The user is the only tenant

No onboarding, no account, no sync, no telemetry consent. First boot lands in a
usable shell. Settings are files; the UI edits them.

## Applying the rules

When a design is ambiguous, apply this order:

1. Remove a step.
2. Make the remaining step keyboard-reachable.
3. Make its latency explicit.
4. Make its trust boundary visible.
5. Only then make it prettier.

The design system provides the vocabulary; these principles provide the grammar.
