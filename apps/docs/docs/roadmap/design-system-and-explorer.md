---
id: design-system-and-explorer
title: Design system and explorer source
sidebar_position: 3
description: The design system and explorer GUI come from the Data Cloud Compare project; this page records the source, the live reference and the adoption plan.
---

# Design system and explorer source

**Decision:** Aether adopts the **Water design system** and the **two-pane explorer
GUI** from the `jottacloud-comparison` project rather than inventing a new visual
language.

## Source of truth

| Item | Value |
| --- | --- |
| Design system | `/Users/olivermolnar/Developer/jottacloud-comparison/design-system` |
| Explorer app source | `/Users/olivermolnar/Developer/jottacloud-comparison/app` |
| Live reference instance | `http://localhost:8787/` |
| Vendored copy | `packages/design-system` |
| React rebuild | `packages/ui` (`FilePane`, `Badge`, `SystemBar`) |
| Visual baseline | Reference explorer running at port 8787 |

The reference app is titled **“Tank ⇄ Jottacloud”**: a dense, two-pane filesystem
comparator with a ⌘K command bar, semantic status pills, glass chrome and three
themes (light, dark, water). The Water theme is the canonical Aether theme.

## Why reuse it

1. **It solves the same interaction problem.** Dense file rows, keyboard-first
   search, status per row, a command palette over everything.
2. **It is already a system, not a mockup.** Tokens (`--ds-*`), component primitives
   (`ds-`), water effects, accessibility rules and a showcase exist.
3. **It keeps four surfaces coherent.** The OS shell, the browser remote session,
   the docs and the marketing site can share one vocabulary.
4. **It reduces design risk.** The visual language has been exercised against real
   file listings, not placeholder cards.

## What was imported in P0

- `tokens.css` — colour, type, spacing, radius, elevation, motion, water tones.
- `components.css` — buttons, badges, cards, panes, command bar, kbd, tables.
- `water.js` — dependency-free caustics, drips and ripples with reduced-motion
  support.
- `index.html` + showcase — served at `/design-system/` in the assembled site.
- `README.md` — rules and divergence policy, adapted with provenance.

The React rebuild currently covers:

| Reference element | Aether component | Tests |
| --- | --- | --- |
| Two-pane file list | `FilePane` | selection, open, search, empty |
| Status pills | `Badge` | tone + label + dot |
| Bottom status bar | `SystemBar` | agents, CPU, memory, audio, latency |
| Command bar (⌘K) | `CommandPalette` + `usePalette` | ARIA, keyboard, click, empty |
| Search/fuzzy behaviour | `@aether/palette` engine | ranking, modes, stale queries |
| Water background | `WaterSurface` | manual + reduced-motion guard |

## Adoption plan

| Phase | Work | Acceptance |
| --- | --- | --- |
| P0 (done) | Vendor tokens/components/effects; rebuild explorer in React; ship marketing + docs | Tokens used by every surface; `FilePane` DOM tests pass |
| P2 | Map tokens to compositor scene-graph styles; match type metrics | Side-by-side screenshot parity with the reference at 1440×900 |
| P3 | Explorer as a shell layer with 10k-row virtualisation | Scroll at 120 Hz; row height/status classes unchanged |
| P3 | Palette layer parity | Same ranking and keyboard model as the web demo |
| P4 | Audio overlay uses the same badge/segmented primitives | No new one-off components |
| P6 | Browser and media chrome reuse the component vocabulary | Visual review against the reference |
| P8 | Accessibility pass on the shared components | Focus, contrast, reduced motion verified |

## Parity checklist

Before any surface is declared “aligned”, verify:

- [ ] Row height 30 px, base type 13 px, chrome 62 px.
- [ ] One accent current; status colours are semantic only.
- [ ] Status always includes a text label and a dot.
- [ ] Motion: interface 160–340 ms; ambient 14–58 s; reduced-motion disables both.
- [ ] Glass surfaces use the token blur; opaque fallback exists.
- [ ] Focus ring visible on every interactive element.
- [ ] Keyboard path exists for every pointer action.
- [ ] No hardcoded colours outside `tokens.css`.

## Divergence policy

The vendored folder is treated as an upstream import. Aether-specific additions go
in `packages/ui/src/ui.css` or `apps/web/src/site.css`, not in `packages/design-system`.
If a change must happen upstream, apply it in both repositories and record it in
[the design system page](/design/design-system).

## Risks

| Risk | Mitigation |
| --- | --- |
| Reference repo drifts from the vendored copy | Record the source commit when re-importing; parity screenshots |
| Docusaurus CSS conflicts with `ds-` classes | Namespace prefix; docs only import tokens |
| Water effects cost frames on the target GPU | Effects are disableable; the shell uses static gradients by default |
| Two React versions across docs and app | One hoisted React; docs pinned in its workspace |

## Definition of done for the import

1. `packages/design-system/README.md` states provenance and divergence policy. ✅
2. The marketing site imports the tokens and components. ✅
3. The explorer is rebuilt as React components and tested. ✅
4. The docs use the tokens and match the type scale. ✅
5. The showcase is published at `/design-system/`. ✅
6. The shell parity work is scheduled in P2/P3. ⏳
