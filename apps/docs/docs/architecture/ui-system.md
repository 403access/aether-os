---
id: ui-system
title: UI system
sidebar_position: 6
description: Layers, surface composition, the Water design language, text, focus and accessibility.
---

# UI system

Aether's UI is a small set of compositor-rendered layers plus native clients. It is
not a widget toolkit with a plugin ecosystem.

## Surfaces and layers

| Surface | Renderer | Input model | Lifetime |
| --- | --- | --- | --- |
| Status bar | shell scene graph | pointer + shortcuts | session |
| Command palette | shell scene graph | modal keyboard | on demand |
| Chat drawer | shell scene graph | text + shortcuts | session or on demand |
| Explorer | shell layer or native client | keyboard-first list | session |
| Browser | WebKitGTK buffer | web event loop | tab |
| Media | libmpv buffer | media keys + pointer | file/session |
| Toasts | shell scene graph | dismiss only | 3–8 s |

A surface is a rectangle with a buffer, a layer and an input policy. The shell
composites them in one pass; it does not spawn a toolkit per surface.

## The design language

The visual language is **Water**, vendored from the Data Cloud Compare design
system. It provides:

- Deep-sea surfaces with a single cyan accent.
- Semantic status colours, always paired with a label and a dot.
- 13 px dense interface type, 30 px rows, 62 px chrome.
- Fast interface motion (160–340 ms) and slow ambient motion (14–58 s).
- A component vocabulary: buttons, badges, panes, command bar, kbd, cards.

The complete token set lives in `packages/design-system/tokens.css`. The React
implementation lives in `packages/ui`; the compositor maps the same tokens to its
scene-graph styles. See [design system](/design/design-system).

## Composition rules

1. **One accent current.** Cyan marks interaction and live state. Status colours
   do not double as brand colours.
2. **No nested cards.** A pane contains rows; a row contains badges. Cards contain
   content, not other cards.
3. **Borders over shadows** for structure; shadows only for floating overlays.
4. **Opacity is a layer signal.** Glass surfaces use `backdrop-filter`; the shell
   approximates it with a pre-blurred texture on GPUs where blur is expensive.
5. **Density is fixed.** Users can change scale, not row density. A compact mode
   would be a second design language.

## Text

- Base UI text: 13 px, 1.6 line height.
- Monospace for paths, commands, metrics, tokens.
- Markdown subset in chat: fenced code, inline code, bold, italic, links.
- Streaming appends invalidate only the last line's damage.
- Code blocks use a monospace atlas with ligatures disabled for alignment.
- No web fonts. System font stack; the shell bundles a fallback for the compositor
  renderer so first paint does not wait on font discovery.

## Focus and modality

Focus is a stack:

```text
overlay focus (palette) → window focus → global shortcuts
```

- An overlay captures keys but not pointer events outside its rect.
- `Esc` pops one focus level, never two.
- A modal dialog cannot be opened by an agent; only the shell or the user can.
- Focus is always visible; the focus ring uses the accent token.
- A client that stops responding cannot hold focus indefinitely.

## Keyboard map (defaults)

| Chord | Action |
| --- | --- |
| `⌘K` | Palette |
| `⌘Tab` | Cycle surfaces |
| `⌘1..9` | Workspace |
| `⌘H / ⌘L` | Shrink / grow master |
| `⌘J / ⌘K` | Focus next / previous window |
| `⌘⇧Q` | Close surface |
| `⌘⇧L` | Lock session |
| `⌘⇧A` | Agent overlay |
| `⌘⇧R` | Recording indicator / stop |

The map is compiled into the shell. Remapping is a config file read at startup; a
bad map fails validation and falls back to defaults with a visible warning.

## Accessibility

Shipped in v1.0:

- Full keyboard operation.
- Visible focus.
- Contrast checked in both themes (the palette is dark-only for now).
- `prefers-reduced-motion` disables ambient and interface motion.
- Status never communicated by colour alone.

Known gaps (not claimed):

- Screen-reader support (AT-SPI) is not implemented.
- No magnifier.
- No on-screen keyboard.
- No high-contrast theme beyond the dark palette; light theme is a “shallow water”
  variant, not an accessibility mode.

These gaps are documented in the [FAQ](/reference/faq) and tracked as P8 work.

## Component inventory

| Component | Package | Tests |
| --- | --- | --- |
| CommandPalette | `packages/ui/src/Palette.tsx` | click, ARIA, empty state, keyboard |
| `usePalette` | `packages/ui/src/usePalette.ts` | async query, selection, abort |
| ChatTranscript | `packages/ui/src/Chat.tsx` | roles, markdown, streaming caret |
| FilePane | `packages/ui/src/Explorer.tsx` | selection, open, search, empty |
| SystemBar | `packages/ui/src/Chrome.tsx` | agents, CPU, memory, audio, latency |
| Badge / Kbd / Meter | `packages/ui/src/Chrome.tsx` | accessible values |
| Markdown subset | `packages/ui/src/markdown.ts` | fenced, inline, streaming |

Every component uses design-system classes and tokens; none hardcodes a colour.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Missing token | Build fails the CSS variable check; no silent fallback |
| Text atlas full | Glyphs evicted LRU; a frame may re-rasterise, never corrupt |
| Overlay throws | Task aborted; overlay closes; shell logs locally |
| Blur unsupported | Surface renders opaque with the same radius |
| Wrong scale factor | Surface is clipped to the output; pointer mapping stays consistent |

## What comes next

- P2: renderer integration, text shaping, cursor themes.
- P3: explorer as a first-class shell layer with 10k-row virtualisation.
- P4: audio device switcher using the same overlay primitives.
- P7: remote client reuses the component vocabulary in the browser.
