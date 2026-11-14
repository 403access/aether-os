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
6. **Bundled type, no web fetch.** The system stack covers the web surfaces; the
   native compositor ships Inter for deterministic chrome metrics and falls back
   to its 8×8 bitmap font when no font file is available.

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

## Native compositor chrome

The OS shell is not a web view. `aether-shell-wayland` reproduces the Water
vocabulary with real pixels: Inter (via `fontdue`), rounded glass panels,
premultiplied alpha, soft shadows, SVG icons and a deep-sea gradient.

| Water primitive | Compositor implementation |
| --- | --- |
| `--ds-bg-grad` | `chrome::background` radial linear gradient |
| `.ds-card--glass` | `chrome::panel` — 14 px radius, 1 px accent border, stacked shadow |
| Panel header bands | `chrome::rounded_top` |
| `.ds-badge`, selection fill | `chrome::rounded` |
| Icon set | `chrome::icon_svg` + `resvg` (search, folder, file, terminal, chat, cpu, memory, audio, agents, palette, close, arrow) |
| Type scale | `aether-text::font` — 11 / 13 / 16 / 20 px, semibold via double-blit |
| Explorer | 62 px app bar, 56 px pane header, 30 px columns and rows, 30 px footer, 36 px status bar |
| Palette | 680 × 420, 58 px search header, six 40 px rows at 46 px pitch, keycap footer |
| Chat | 400 px drawer, header pill, assistant card with code strips, 38 px composer |
| Terminal | `aether-terminal` (foot) with Water ANSI palette, 11 px mono, 28 px top pad, 1 px CSD border |

![Water explorer in the compositor](/img/smithay-explorer.png)

*Explorer layer at 1280×800: app bar, Files/Palette/Chat tabs, search affordance,
pane card, aligned columns and a status bar with icon badges.*

![Water command palette](/img/smithay-palette.png)

*Palette: search header with inline `esc` keycap, six result rows with icon tiles,
rounded active row and a keycap footer.*

![Water chat drawer](/img/smithay-chat.png)

*Chat: live pill, assistant message card, code-block strips, scrollbar and the
composer with send affordance. Bold spans are rendered with a second coverage
pass, not a different font.*

![Water-themed terminal](/img/smithay-terminal.png)

*`aether-terminal` runs foot with the Water ANSI palette, DejaVu Sans Mono at
11 px and a 1 px border. It has no client-side titlebar under the compositor, so the
shell workspace pills stay legible above the padded terminal surface.*

Font discovery order: `AETHER_FONT`, standard Inter paths, `/nix/store` scan,
then the bitmap fallback. NixOS sets `AETHER_FONT` in the session service and
installs Inter via `fonts.packages`; the Smithay dev shell exports it too.

The default terminal is `aether-terminal`, a wrapper around **foot** with
`os/compositor/aether-shell-smithay/foot/foot.ini`: Water background and ANSI
palette, `DejaVu Sans Mono` at 11 px, a cyan beam cursor and a 1 px CSD border.
The 28 px top pad keeps the shell's workspace pills from colliding with terminal
output, since foot has no titlebar under the compositor. Set `AETHER_TERMINAL`
to override; `aether-session` exports the packaged wrapper.

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
| `os/compositor/aether-shell-wayland/src/chrome.rs` | Rounded panels, shadows, gradient, SVG icons |
| `os/compositor/crates/aether-text/src/font.rs` | Inter/fontdue rasterizer |
| `os/compositor/aether-shell-smithay/foot/foot.ini` | Water terminal palette and CSD |
| `os/compositor/aether-shell-smithay/aether-terminal.sh` | Dev terminal wrapper |
| `os/compositor/crates/aether-shell/src/design.rs` | Compositor token mirror (`scripts/design-parity.ts`) |

## Contribution rules

1. Additions go in `packages/ui` or the consuming app, not the vendored folder.
2. New colours require a token and a contrast check.
3. New motion requires a duration token and a reduced-motion fallback.
4. New status requires a label and a dot, not just a colour.
5. Screenshot parity with the reference explorer is part of review from P2 onward.
