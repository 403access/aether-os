---
id: using-aether
title: Using Aether
sidebar_position: 5
description: "The daily-use manual: global hotkeys, palette modes, windows, workspaces, explorer, chat, audio and recovery."
---

# Using Aether

Aether is built for one thing: **getting to the next action without leaving the
keyboard**. There is one accent colour, one modal layer at a time, and every global
action is reachable with `Super` (the Command/Windows key).

Press **`Super+/`** at any time to open the same shortcut sheet inside the system:

![Shortcut cheat sheet in the compositor](/img/smithay-shortcuts.png)

## Keyboard profiles

Two profiles ship:

| Profile | Primary modifier | Notes |
| --- | --- | --- |
| **Simple (default)** | `Ctrl+Alt` | VNC/macOS-friendly aliases; `Super` shortcuts still work |
| **Standard** | `Super` | The original Super-only map |

Select at startup:

```bash
AETHER_KEY_PROFILE=standard aether-shell-drm
AETHER_KEY_PROFILE=standard ./os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay
```

The in-shell cheat sheet (`Super+/` or `Ctrl+Alt+/`) shows the active profile.

## The ten commands to learn first

| Action | Simple (default) | Standard |
| --- | --- | --- |
| Command palette | `Ctrl+Alt+Space` | `Super+Space` |
| New terminal | `Ctrl+Alt+Return` | `Super+Return` |
| Explorer | `Ctrl+Alt+E` | `Super+E` |
| Chat | `Ctrl+Alt+C` | `Super+C` |
| Audio routes | `Ctrl+Alt+A` | `Super+A` |
| Shortcut cheat sheet | `Ctrl+Alt+/` | `Super+/` |
| Focus next window | `Ctrl+Alt+Tab` | `Super+Tab` |
| Switch workspace | `Ctrl+Alt+1/2/3` | `Super+1/2/3` |
| Close focused window | `Ctrl+Alt+Q` | `Super+Q` |
| Close overlay | `Esc` | `Esc` |

`Ctrl+Alt+J` / `Ctrl+Alt+K` (Standard: `Super+J` / `Super+K`) move focus
next/previous. `Ctrl+Alt+Shift+Tab` walks the MRU stack backwards.

In Simple, the complete `Super` map remains available as a fallback, so existing
muscle memory keeps working.

## The top shortcut dock

The top-centre dock is the mouse-friendly mirror of the global shortcuts:

![Top shortcut dock](/img/smithay-topbar.png)

| Button | Opens |
| --- | --- |
| Palette | `Super+Space` |
| Terminal | `Super+Return` |
| Files | `Super+E` |
| Chat | `Super+C` |
| Audio | `Super+A` |
| Shortcuts | `Super+/` |

Click any button. The dock hides while a surface is open so it never competes
with the palette, explorer, chat or audio overlay.

## Global shortcuts

The tables below list the **Simple** aliases first with the Standard chord in
parentheses. Pointer actions are identical in both profiles.

### Surfaces

| Chord | Action |
| --- | --- |
| `Ctrl+Alt+Space` (`Super+Space`) | Toggle the command palette |
| `Ctrl+Alt+Return` (`Super+Return`) | Spawn `aether-terminal` |
| `Ctrl+Alt+E` (`Super+E`) | Toggle the full-output explorer |
| `Ctrl+Alt+C` (`Super+C`) | Toggle the streaming chat drawer |
| `Ctrl+Alt+A` (`Super+A`) | Toggle the audio overlay |
| `Ctrl+Alt+/` (`Super+/`) | Toggle the shortcut cheat sheet |
| `Esc` | Close the open overlay, one layer at a time |

Opening one surface closes the others. The palette, explorer, chat and audio
layers are mutually exclusive; there is never a hidden modal stealing keys.

### Windows

| Chord | Action |
| --- | --- |
| `Ctrl+Alt+Tab` / `Ctrl+Alt+Shift+Tab` (`Super+Tab` / `Super+Shift+Tab`) | Focus next / previous in MRU order |
| `Ctrl+Alt+J` / `Ctrl+Alt+K` (`Super+J` / `Super+K`) | Focus next / previous |
| `Ctrl+Alt+M` (`Super+M`) | Maximize / restore |
| `Ctrl+Alt+F` (`Super+F`) | Fullscreen / restore; hides shell chrome |
| `Ctrl+Alt+Q` (`Super+Q`) | Ask the focused client to close |
| `Alt+Left drag` | Move a window; promotes a tile to floating |
| `Alt+Right drag` | Resize a floating window from the nearest corner |
| Release at edge | Snap to left/right half; top edge maximizes |
| Click a workspace pill | Switch that output's workspace |

### Workspaces and outputs

| Chord | Action |
| --- | --- |
| `Ctrl+Alt+1/2/3` (`Super+1/2/3`) | Switch workspace on the focused output |
| `Ctrl+Alt+Shift+1/2/3` (`Super+Shift+1/2/3`) | Move the focused window to a workspace |
| `Super+Alt+1/2/3` | Switch the workspace on the output under the pointer |

Each output has its own active workspace. The pills at the top-left show the
output under the pointer; the active pill is cyan. On a multi-output desktop,
`Super+Alt+1/2/3` targets the output containing the pointer.

### Layout

| Chord | Action |
| --- | --- |
| `Alt+Left drag` | Move/float |
| `Alt+Right drag` | Resize |

Tiling is equal-width by design: one window fills the work area, each new window
adds a column, and closing a window gives the space back to the rest. There is no
master/stack ratio or divider drag.

## Command palette

`Super+Space` opens the palette above the current workspace.

| Input | Meaning |
| --- | --- |
| `>command` | Commands and shell actions |
| `@agent` | Windows and agent sessions |
| `/file` | Indexed files |
| `~setting` | Settings |
| anything else | All providers |

- Type to filter; matching is prefix/subsequence and case-insensitive.
- `Up`/`Down` wraps; the list slides to keep selection visible.
- `Enter` or a row click runs the result.
- `Backspace` deletes a query character.
- `Esc` closes the palette.

The palette is also the fastest way to reach a window: type part of its title,
press `Enter`, and Aether focuses it and switches to its workspace. Files open
through the configured opener; apps launch from XDG desktop entries.

## Explorer

`Super+E` opens the full-output explorer.

| Input | Action |
| --- | --- |
| Type | Filter the file list |
| `Up` / `Down` | Move selection |
| `PgUp` / `PgDn` | Page |
| `Enter` | Open the selected file and close |
| `Backspace` | Edit the filter (delete a character) |
| `Esc` | Close |

The explorer only composes visible rows. The footer shows the visible count; the
status bar keeps the selected path and live system facts in view.

## Chat

`Super+C` opens the streaming chat drawer.

| Input | Action |
| --- | --- |
| `PgUp` / `PgDn` | Scroll the transcript |
| `Esc` | Close |

The drawer displays streamed markdown today; the composer is present but outbound
messages are P5 host wiring, so this manual does not claim they are sendable yet.

Markdown is parsed incrementally. Headings, lists, bold, inline code and fenced
code blocks are supported; only visible rows enter the render plan.

## Audio

`Super+A` opens routes and latency presets.

| Input | Action |
| --- | --- |
| `Up` / `Down` | Move through outputs/inputs |
| `Enter` | Make the selected route the default |
| `P` | Cycle latency preset: media → voice → monitor |
| `Esc` | Close |

Defaults carry a labelled `default` pill. Transport, profile and codec are text,
never colour alone. The `mic locked` label is the agent capability state; a
process needs an explicit `audio.capture` capability before capture.

The CLI exposes the same policy for scripts:

```bash
aether-audio list
aether-audio switch 43
aether-audio preset voice
aether-audio apps
```

## Terminal

`Super+Return` launches `aether-terminal` immediately—no palette step.

The terminal is foot with the Water palette, DejaVu Sans Mono at 11 px and a dark
1 px border. If `AETHER_TERMINAL` is set, that command is used instead. The
palette's **New terminal** action does the same thing.

## Recovery and layer rules

- `Esc` closes exactly one overlay. It never closes two layers at once.
- `Super+Space`, `Super+E`, `Super+C` and `Super+A` replace each other; the last
  one wins.
- Clicking outside an overlay closes it.
- A client that hangs can be closed with `Super+Q`; the shell stays responsive.
- If an overlay seems stuck, press `Esc` twice: first the overlay, then nothing.
- The cheat sheet is read-only; `Super+/` or `Esc` closes it.

## Speed rules

The UX follows the compositor's performance rules:

1. **No mode hunting.** Global actions are single chords; the cheat sheet is one
   more chord away.
2. **No hidden focus.** The active layer and active output are visible.
3. **One accent.** Cyan always means “interactive/live”, never decoration.
4. **No animation on the critical path.** Overlays appear and accept keys
   immediately.
5. **Fast paths are also CLI paths.** Audio, index and launcher state can be
   inspected without opening a window.

## Where to go next

- [Boot the development VM](/getting-started/virtual-machine) for the VNC desktop.
- [Design system](/design/design-system) for the visual language.
- [Smithay backend](/architecture/smithay-backend) for the implementation map.
- [Roadmap](/roadmap) for what is still a budget rather than a shipped result.
