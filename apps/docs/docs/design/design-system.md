---
id: design-system
title: Design system — Water
sidebar_position: 1
description: The Water design language, its tokens and rules, and how it is implemented across Aether surfaces.
---

# Design system — Water

Water is the visual language of Aether. It was adopted from the Data Cloud Compare
project (see [the source record](/roadmap/design-system-and-explorer)) because it
already solves the interface problem Aether has: dense lists, one accent, semantic
status, glass chrome and slow ambient motion.

## Principles

1. **Deep sea.** Dark, low-contrast surfaces with a single cyan current.
2. **One accent.** `--ds-accent` marks interaction and live state. Status colours are
   separate and semantic.
3. **Density first.** 13 px base type, 30 px rows, 62 px chrome.
4. **Motion explains.** Interface motion 160–340 ms; ambient 14–58 s; both
   disableable.
5. **Colour never alone.** Every status pill is colour + label + dot.
6. **No web fonts.** System stack, monospace for technical data.

## Tokens

All tokens are CSS custom properties prefixed `--ds-`, defined in
`packages/design-system/tokens.css`.

| Group | Examples |
| --- | --- |
| Colour | `--ds-bg`, `--ds-surface`, `--ds-text`, `--ds-line`, `--ds-accent`, `--ds-glow` |
| Status | `--ds-green`, `--ds-amber`, `--ds-red`, `--ds-blue`, `--ds-violet`, each with `-bg` |
| Type | `--ds-font`, `--ds-font-mono`, `--ds-text-2xs` … `--ds-text-4xl` |
| Space | `--ds-space-1` (4 px) … `--ds-space-13` (96 px) |
| Radius | `--ds-radius-xs` (5 px) … `--ds-radius-2xl` (26 px), `--ds-radius-pill` |
| Elevation | `--ds-shadow-soft`, `--ds-shadow`, `--ds-shadow-pop` |
| Motion | `--ds-motion-fast/base/layout`, `--ds-ease-out` |
| Layout | `--ds-content` (1180 px), `--ds-header-h` (62 px), `--ds-focus-ring` |
| Water | `--ds-bg-grad`, `--ds-water-*` |

Changing a token is a product change, not a styling tweak. It requires a screenshot
review across the marketing site, the docs, the UI kit and the reference explorer.

## Themes and tones

```html
<html
  data-theme="dark"        <!-- dark | light | water -->
  data-water-tone="ocean"  <!-- lagoon | ocean | deep -->
  data-water-speed="flowing" <!-- calm | flowing | rapid -->
  data-water-effects="on"  <!-- on | off -->
>
```

| Tone | Accent | Use |
| --- | --- | --- |
| Lagoon | `#55F0E0` | Bright marketing moments |
| Ocean | `#38D6FF` | Default product theme |
| Deep | `#48B7FF` | Quiet, archival, late night |

Aether uses `dark` + `ocean` everywhere by default. The light theme exists and is
tested, but is not the primary target.

## Components

The React layer lives in `packages/ui`. Components use only tokens and `ds-` classes.

| Component | Purpose |
| --- | --- |
| `CommandPalette` | Modal search surface with ARIA combobox/listbox semantics |
| `usePalette` | React binding for the headless engine (cancellation, keyboard) |
| `ChatTranscript` | Streaming messages with a safe markdown subset |
| `FilePane` | Dense two-pane explorer pane |
| `SystemBar` | Session, agents, CPU, memory, audio, frame time |
| `Badge` | Semantic status pill |
| `Meter` | Accessible budget/progress bar |
| `Kbd` | Keycap |

Primitives from `components.css` (buttons, cards, inputs, segmented controls, panes,
command bar) are available to docs and the marketing site without React.

## Usage

```ts
// app entry
import "@aether/design-system/tokens.css";
import "@aether/design-system/components.css";
import "@aether/ui/ui.css";
```

```tsx
import { Badge, CommandPalette, usePalette } from "@aether/ui";
```

Docusaurus imports only `tokens.css` to avoid component-class collisions with
Infima, then maps Infima variables to the tokens in `apps/docs/src/css/custom.css`.

## Motion

| Layer | Duration | Examples |
| --- | --- | --- |
| Fast | 160–210 ms | Hover, selection, badges |
| Base | 240–340 ms | Overlays, panels |
| Layout | 340–520 ms | Pane resize, workspace change |
| Ambient | 14–58 s | Waves, caustics, drips |

`prefers-reduced-motion: reduce` disables all of it. The water canvas is not even
instantiated in that case.

## Accessibility rules

- Focus ring: `0 0 0 3px var(--ds-accent-soft)`; never removed.
- Contrast checked against `--ds-surface` in dark and light.
- Interactive elements are real buttons/inputs; no div click handlers in the kit.
- ARIA: palette is a combobox controlling a listbox; options carry
  `aria-selected`; chat is a polite log.
- Decorative layers are `aria-hidden` and `pointer-events: none`.

Known gaps: AT-SPI/screen-reader support is not implemented; see
[UI system](/architecture/ui-system#accessibility).

## Implementation map

| File | Role |
| --- | --- |
| `packages/design-system/tokens.css` | Token source of truth |
| `packages/design-system/components.css` | Framework-free primitives |
| `packages/design-system/water.js` | Canvas caustics/drips/ripples |
| `packages/design-system/index.html` | Showcase at `/design-system/` |
| `packages/ui/src/ui.css` | Aether-specific components |
| `apps/web/src/site.css` | Marketing layout |
| `apps/docs/src/css/custom.css` | Docs theme mapping |

## Contribution rules

1. Additions go in `packages/ui` or the consuming app, not the vendored folder.
2. New colours require a token and a contrast check.
3. New motion requires a duration token and a reduced-motion fallback.
4. New status requires a label and a dot, not just a colour.
5. Screenshot parity with the reference explorer is part of review from P2 onward.
