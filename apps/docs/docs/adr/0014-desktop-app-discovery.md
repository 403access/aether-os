---
id: 0014-desktop-app-discovery
title: ADR-0014 — XDG desktop-entry app discovery
sidebar_position: 15
---

# ADR-0014 — XDG desktop-entry app discovery

**Status:** Accepted · **Date:** 2026-11-14

## Context

The command palette should launch installed applications, not only the built-in
terminal command. Aether needs a provider that works with normal Linux packages
without depending on a full desktop session, an icon theme or D-Bus activation.

## Decision

Add `aether-desktop`, a small crate that scans `applications/*.desktop` below the
XDG data directories and parses the fields the launcher needs:

- `Type=Application` only; skip `NoDisplay`, `Hidden` and `Terminal=true`;
- read `Name`, `Comment` and `Exec`;
- split `Exec` with a small quote/escape tokenizer and drop XDG field-code
  arguments (`%f`, `%U`, ...) because the palette has no file context;
- earlier XDG data directories win when desktop ids collide;
- scan at compositor startup and register entries as palette launchers.

No icon parsing, localization, desktop actions, D-Bus activation or live
directory watching in this slice.

## Consequences

- Standard applications appear in the palette with no per-app integration.
- Launching uses `Exec` directly, inheriting the compositor's environment and
  capabilities; sandboxing is a later policy layer.
- New installations appear after a restart until live refresh is added.
- Terminal-only entries are skipped; a future terminal app can be a normal GUI
  entry or the built-in `New terminal` command.
- The parser is small enough to test without a desktop environment.

## Alternatives

- **D-Bus activation** — richer, but requires a session bus and service files;
  the compositor must still know display names.
- **A custom app registry** — non-standard and needs packaging work per app.
- **A full freedesktop menu library** — more dependencies and policy than the
  palette needs today.
