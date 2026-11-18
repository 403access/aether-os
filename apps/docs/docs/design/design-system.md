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
3. **Density first.** 13 px base type, 30 px rows, a 38 px menu bar in the shell
   and a 62 px header on the web.
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

Aether uses `water` + `ocean` in the compositor and `dark` + `ocean` on the web
surfaces by default. The light theme exists and is tested, but is not the primary
target.

The compositor's **Themes** app (`Super+T`) edits the same state as the reference
Water settings panel:

| Group | Settings |
| --- | --- |
| Theme | System, Light, Dark, Water |
| Effects | Water effects, drips, click ripples, caustics, animated row highlights |
| Current | Flow speed (calm/flowing/rapid), drip energy (subtle/balanced/lively) |
| Look | Background (still/sea), Water tone (lagoon/ocean/deep), glass surfaces |

The app derives the runtime chrome palette from mode and tone, applies glass to
panel fills and borders, and rebuilds the backdrop from background style, caustics
and speed. Every change is written to
`$XDG_CONFIG_HOME/aether/themes.json` (`AETHER_THEME_CONFIG` overrides the path);
the JSON keys match the reference app's `clickRipples` / `rowMotion` shape.

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
| Type scale | `aether-text::font` — 8 / 9 / 10 / 11 / 13 / 16 / 20 px, semibold via double-blit |
| Menu bar | 38 px full-width bar: brand, active app, five menus, sync pill, CPU/RAM/STORAGE meters, tray, clock |
| Explorer | Floating window 16 px in from the work area: 40 px title bar, 52 px app bar, 56 px pane header, 30 px columns and rows, 30 px footer, 32 px status strip |
| Palette | 680 × 420, 58 px search header, six 40 px rows at 46 px pitch, keycap footer |
| Chat | 400 px drawer, header pill, assistant card with code strips, 38 px composer |
| Client windows | 40 px compositor title bar: three 11 px traffic lights, 23 px app mark, bold title with dim subtitle, accent rim on focus |
| Clock and Calendar | Pure civil-time formatter; 700 × 590 Calendar window with a six-row month grid and agenda |
| Terminal | `aether-terminal` (foot) with Water ANSI palette, 11 px mono, framed by the compositor title bar |
| Audio overlay | 660 × 560 glass panel, section labels, route rows, preset chips, input meter |
| Calendar | 700 × 590 window, six-row month grid, agenda column, event dots |
| Desktop dock | Centred 441 × 62 glass strip, eight 38 px tinted tiles, separator, running dots |

![Water explorer in the compositor](/img/smithay-explorer.png)

*Explorer window at 1280×800: the menu bar above, a 40 px title bar with traffic
lights, a 52 px app bar with search, the pane card with aligned columns and a
32 px status strip with icon badges. The dock stays visible below.*

![Calendar app](/img/smithay-calendar.png)

*Calendar: 700 × 590 window, six-row month grid, weekday headers, accent-filled
today, selected-day rim, event dots and an agenda column. Opened from the clock,
Window ▸ Calendar or the palette.*

![Water command palette](/img/smithay-palette.png)

*Palette: search header with inline `esc` keycap, six result rows with icon tiles,
rounded active row and a keycap footer.*

![Water chat drawer](/img/smithay-chat.png)

*Chat: live pill, assistant message card, code-block strips, scrollbar and the
composer with send affordance. Bold spans are rendered with a second coverage
pass, not a different font.*

![Water-themed terminal](/img/smithay-terminal.png)

![Audio overlay](/img/smithay-audio.png)

*Audio overlay: output/input sections, default pills, transport and codec labels,
input level meter, capability state and latency preset chips.*

*`aether-terminal` runs foot with the Water ANSI palette and DejaVu Sans Mono at
11 px. The compositor draws its 40 px title bar — traffic lights, app mark, bold
title with a dim subtitle and an accent rim on focus — so a client app reads as
a first-party surface.*

Font discovery order: `AETHER_FONT`, standard Inter paths, `/nix/store` scan,
then the bitmap fallback. NixOS sets `AETHER_FONT` in the session service and
installs Inter via `fonts.packages`; the Smithay dev shell exports it too.

The default terminal is `aether-terminal`, a wrapper around **foot** with
`os/compositor/aether-shell-smithay/foot/foot.ini`: Water background and ANSI
palette, `DejaVu Sans Mono` at 11 px and a cyan beam cursor. Set
`AETHER_TERMINAL` to override; `aether-session` exports the packaged wrapper.

The compositor never asks clients to draw their own frame: `xdg-decoration` is
not advertised, and every toplevel gets the server-side title bar described
above. The shell reserves a 38 px `top_gap` in the layout so tiled, floating
and maximized clients start below the menu bar, exactly like the first-party
surfaces.

`bun run parity` captures the Water OS demo and the compositor at 1280 × 800
and diffs them by region; see `artifacts/os-parity/report.md`.

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
| `os/compositor/aether-shell-wayland/src/render/chrome.rs` | Rounded panels, shadows, gradient, SVG icons |
| `os/compositor/aether-shell-wayland/src/render/window_frame.rs` | Server-side title bar for client windows |
| `os/compositor/crates/aether-text/src/font.rs` | Inter/fontdue rasterizer |
| `os/compositor/aether-shell-smithay/foot/foot.ini` | Water terminal palette |
| `os/compositor/aether-shell-smithay/aether-terminal.sh` | Dev terminal wrapper |
| `os/compositor/crates/aether-shell/src/design.rs` | Compositor token mirror (`scripts/design-parity.ts`) |

## Contribution rules

1. Additions go in `packages/ui` or the consuming app, not the vendored folder.
2. New colours require a token and a contrast check.
3. New motion requires a duration token and a reduced-motion fallback.
4. New status requires a label and a dot, not just a colour.
5. Screenshot parity with the reference explorer is part of review from P2 onward.
