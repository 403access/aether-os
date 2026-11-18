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
| Menu bar | shell scene graph | pointer + shortcuts | session |
| Desktop dock | shell scene graph | pointer | session |
| Command palette | shell scene graph | modal keyboard | on demand |
| Chat drawer | shell scene graph | text + shortcuts | session or on demand |
| Audio overlay | shell scene graph | keyboard + pointer | on demand |
| Themes app | shell scene graph | keyboard + pointer | on demand |
| Task manager | shell scene graph | keyboard + pointer | on demand |
| Calendar | shell scene graph | keyboard + pointer | on demand |
| Agent sandboxes | shell scene graph | keyboard + pointer | on demand |

The palette merges three application sources: XDG `.desktop` entries, Aether's
own surface commands (Files, Chat, Audio, Agents, Themes, Keyboard shortcuts, Task
manager) and external launcher manifests.
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
scene-graph styles. The compositor also derives a runtime palette from the active
theme mode and Water tone, so chrome, selection fills and status colours follow
Light, Dark and Water consistently. See [design system](/design/design-system).

The **Themes** app (`Super+T`) edits the same settings as the reference Water
panel: mode, effects, cursor drips, click ripples, caustic shimmer, row motion,
flow speed, drip energy, background, tone and glass. Changes apply immediately and
persist to `$XDG_CONFIG_HOME/aether/themes.json`.

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
- No downloaded web fonts. Web surfaces use the system stack; the compositor
  loads bundled Inter through `AETHER_FONT` and keeps the 8×8 bitmap font as a
  first-paint fallback, so chrome never waits on font discovery.

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

Two profiles exist. **Simple** is the default and adds Ctrl+Alt aliases; the full
Super map remains available. **Standard** exposes Super only.

| Chord (Simple → Standard) | Action |
| --- | --- |
| `Ctrl+Alt+Space` → `Super+Space` | Palette |
| `Ctrl+Alt+Return` → `Super+Return` | New terminal |
| `Ctrl+Alt+E` → `Super+E` | Explorer |
| `Ctrl+Alt+C` → `Super+C` | Chat |
| `Ctrl+Alt+A` → `Super+A` | Audio overlay |
| `Ctrl+Alt+T` → `Super+T` | Themes app |
| `Ctrl+Alt+/` → `Super+/` | Shortcut cheat sheet |
| `Ctrl+Alt+Tab` → `Super+Tab` | Focus next (MRU) |
| `Ctrl+Alt+Shift+Tab` → `Super+Shift+Tab` | Focus previous (MRU) |
| `Ctrl+Alt+J/K` → `Super+J/K` | Focus next / previous |
| `Ctrl+Alt+Q` → `Super+Q` | Close window |
| `Ctrl+Alt+M` → `Super+M` | Maximize / restore |
| `Ctrl+Alt+F` → `Super+F` | Fullscreen / restore |
| `Ctrl+Alt+1..3` → `Super+1..3` | Switch workspace |
| `Ctrl+Alt+Shift+1..3` → `Super+Shift+1..3` | Move window to workspace |
| `Super+Alt+1..3` | Switch the output's workspace |
| `Alt+Left drag` | Move / float |
| `Alt+Right drag` | Resize floating |
| `Esc` | Close the open overlay |

The map is compiled into the shell from `aether-shell-wayland/src/keymap.rs`; the
same profile table drives the cheat sheet. Select with
`AETHER_KEY_PROFILE=standard|simple` (default `simple`). Runtime remapping is
**not implemented** yet: changing a profile currently requires a rebuild so the
cheat sheet and dispatch cannot drift apart.

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
- P4: audio device switcher and latency presets are implemented; live peak
  meters and per-app rows expand with P5.
- P7: the browser remote client reuses the same token vocabulary; the shell
  session overlay and the viewer HUD are implemented.
