---
id: using-aether
title: Using Aether
sidebar_position: 6
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

## The commands to learn first

| Action | Simple (default) | Standard |
| --- | --- | --- |
| Command palette | `Ctrl+Alt+Space` | `Super+Space` |
| New terminal | `Ctrl+Alt+Return` | `Super+Return` |
| Explorer | `Ctrl+Alt+E` | `Super+E` |
| Chat | `Ctrl+Alt+C` | `Super+C` |
| Audio routes | `Ctrl+Alt+A` | `Super+A` |
| Agent sandboxes | `Ctrl+Alt+B` | `Super+B` |
| Themes | `Ctrl+Alt+T` | `Super+T` |
| Remote session | `Ctrl+Alt+R` | `Super+R` |
| Shortcut cheat sheet | `Ctrl+Alt+/` | `Super+/` |
| Focus next window | `Ctrl+Alt+Tab` | `Super+Tab` |
| Switch workspace | `Ctrl+Alt+1/2/3` | `Super+1/2/3` |
| Close focused window | `Ctrl+Alt+Q` | `Super+Q` |
| Close overlay | `Esc` | `Esc` |

`Ctrl+Alt+J` / `Ctrl+Alt+K` (Standard: `Super+J` / `Super+K`) move focus
next/previous. `Ctrl+Alt+Shift+Tab` walks the MRU stack backwards.

In Simple, the complete `Super` map remains available as a fallback, so existing
muscle memory keeps working.

## The menu bar and dock

The 38 px menu bar is permanent chrome, mirroring the Water OS concept:

![Menu bar and dock](/img/smithay-topbar.png)

| Area | Action |
| --- | --- |
| Brand mark | Keyboard cheat sheet (`Super+/`) |
| File / View / Go / Window / Help | Popover menus for windows, themes and shortcuts |
| Synced pill, CPU/RAM/STORAGE | Task manager |
| Wi-Fi, Bluetooth, battery | Tray (reserved) |
| Sound | Audio overlay (`Super+A`) |
| Clock | Calendar app |
| Moon | Cycle Water → Dark → Light |
| Grid | Launcher palette (`Super+Space`) |

The dock at the bottom edge mirrors the same applications: Explorer, Search,
Settings, Terminal, Tide Radio, About, Trash and the design-system gallery.
The Explorer is a floating window at 16/54 with its own title bar, so the dock
and the sea stay visible while you work.

## Calendar

Click the menu bar clock (or choose Window ▸ Calendar) to open the Calendar.
The month grid is always six rows, today is filled with the accent, selected
days carry an accent border and days with agenda entries show a dot.

| Key | Action |
| --- | --- |
| Arrow keys | Move the selection by a day or a week |
| Page Up / Page Down | Previous / next month |
| `t` | Jump to today |
| `Esc` | Close |

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
| `Ctrl+Alt+B` (`Super+B`) | Toggle the agent-sandbox monitor |
| `Ctrl+Alt+T` (`Super+T`) | Toggle the Themes app |
| `Ctrl+Alt+R` (`Super+R`) | Toggle the remote-session overlay |
| `Ctrl+Alt+/` (`Super+/`) | Toggle the shortcut cheat sheet |
| `Esc` | Close the open overlay, one layer at a time |

Opening one surface closes the others. The palette, explorer, chat, audio and
Themes layers are mutually exclusive; there is never a hidden modal stealing keys.

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

### Workspaces and outputs

| Chord | Action |
| --- | --- |
| `Ctrl+Alt+1/2/3` (`Super+1/2/3`) | Switch workspace on the focused output |
| `Ctrl+Alt+Shift+1/2/3` (`Super+Shift+1/2/3`) | Move the focused window to a workspace |
| `Super+Alt+1/2/3` | Switch the workspace on the output under the pointer |

Each output has its own active workspace. The menu bar names the active
application; workspaces are keyboard- and palette-driven (`Super+1/2/3`, or the
palette's workspace rows). On a multi-output desktop, `Super+Alt+1/2/3` targets
the output containing the pointer.

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
press `Enter`, and Aether focuses it and switches to its workspace. Installed
applications come from XDG `.desktop` entries; Aether's own surfaces are indexed
as applications too — **Files**, **Chat**, **Audio**, **Agents**, **Themes**,
**Keyboard shortcuts** and **Task manager**. A **Aether Terminal** desktop entry
is installed on the target and live images.

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
status strip keeps the selected path and live system facts in view.

## Chat

`Super+C` opens the streaming chat drawer.

| Input | Action |
| --- | --- |
| `PgUp` / `PgDn` | Scroll the transcript |
| `Esc` | Close |

The drawer displays streamed markdown and tool-call traces. `aether-chat` owns the
model loop; set `AETHER_CHAT_ENDPOINT` in `~/.config/aether/chat.env` to a local
OpenAI-compatible endpoint. Without one, the first prompt reports the missing
configuration instead of failing silently.

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

## Agent sandboxes

`Super+B` opens the agent-sandbox monitor. It reads the runtime registry written
by `aether-sandbox run`, so a sandboxed process appears without a supervisor.

| Input | Action |
| --- | --- |
| `Up` / `Down` | Move selection |
| `PgUp` / `PgDn` | Page |
| `K` | Send `SIGTERM` to the selected sandbox |
| `Shift+K` | Send `SIGKILL` |
| `L` or `Enter` | Open the sandbox log (`--log PATH`) |
| `R` | Refresh immediately |
| `C` | Prune stale records |
| `Esc` | Close |

Each row shows the label, id, trust, tier, PID, CPU and RSS. The detail pane shows
the enforcement (`landlock-seccomp`), UID, limits, capabilities, read/write paths,
warnings and log path. A record whose PID is gone stays visible with a `stale`
label until the CLI cleans it up or you press `C`.

Start a sandbox from a terminal:

```bash
aether-sandbox run --id build --label "Build agent" \
  --read ~/work --write ~/work/scratch --log ~/work/build.log \
  -- /usr/bin/python3 ~/work/task.py
```

## Themes

`Super+T` opens the Themes app. It exposes the same settings as the reference
explorer's Water panel and saves each change to disk.

| Input | Action |
| --- | --- |
| `Up` / `Down` | Move through settings |
| `Left` / `Right` | Change the selected mode, tone, background or speed |
| `Enter` / `Space` | Toggle a switch or advance a selection |
| `W` | Use the Water theme |
| `R` | Reset water settings |
| `Esc` | Close |

| Group | Settings |
| --- | --- |
| Theme | System, Light, Dark, Water |
| Effects | Water effects, cursor drips, click ripples, caustic shimmer, animated row highlights |
| Current | Flow speed, drip energy |
| Look | Background, Water tone, glass surfaces |

`System` follows the session default, which is Water. Tone changes the accent
current and caustic colour used by the chrome. Mode
changes the workspace backdrop and the derived chrome palette; glass controls
panel fill and borders; caustics, background style and flow speed shape the
backdrop. The preview band mirrors cursor drips, click ripples, caustics,
background style, tone and drip energy. Animated row highlights remain a stored
Water preference for surfaces that animate hover state; static chrome does not.

Settings are saved on this device at `$XDG_CONFIG_HOME/aether/themes.json`
(falling back to `~/.config/aether/themes.json`). Set `AETHER_THEME_CONFIG` to
override the path.

## Remote display

`Super+R` opens the remote-session panel. It shows the one-shot pairing code,
the session id, the transport tier, codec, latency, bitrate and the always-visible
recording indicator.

| Input | Action |
| --- | --- |
| `Up` / `Down` | Move through the audited actions |
| `Enter` | Activate the selected action |
| `I` | Toggle remote input |
| `C` | Toggle clipboard (off by default) |
| `S` | Stop the session |
| `Esc` | Close the panel |

Start the local service for a session:

```bash
systemctl --user start aether-remote
aether-remote pair                 # mint a code and inspect the welcome
curl -s http://127.0.0.1:7443/health
```

The service binds loopback only. Input is scancodes and relative motion; there is
no command channel. Clipboard starts off and every toggle is audited.

![Remote-session overlay](/img/smithay-remote.png)

### Browser live viewer

The marketing site ships a real viewer page at `/viewer.html`. It posts the hello
to `aether-remote serve`, answers the WebRTC offer, renders the H.264 track and
returns scancodes over the `input` data channel.

```bash
# Terminal 1: host. `--code` is optional; without it the printed code is random.
cargo build --manifest-path os/compositor/Cargo.toml -p aether-remote
./os/compositor/target/debug/aether-remote serve \
  --http 127.0.0.1:7443 --udp 127.0.0.1:0 --code K7QM2H4P

# Terminal 2: the page.
bun run dev:web
```

Open `http://localhost:4321/viewer.html?code=K7QM2H4P`, click **Connect**, then
tick **Send input** (pointer lock is requested on the video). The HUD shows the
session id, tier, codec, resolution, fps, bitrate, RTT, decoded frames and the
data-channel state; the recording indicator stays visible while the track is
live. **Stop** sends the bye and closes the peer connection.

Notes:

- Run the page and the host on the same machine. The candidate is loopback-only.
- Use system Chrome (or another browser with H.264); Playwright's bundled
  Chromium usually lacks the proprietary decoder.
- The pairing code is one-shot. Restart the host, or omit `--code` and copy the
  printed code, for another pairing.
- The host still streams its synthetic 640×360 pattern; binding the compositor's
  DMA-BUFs is the remaining P7 task. The page reads the host's `/health` document
  and shows a **synthetic test source** badge over the video, so the test pattern
  is never mistaken for the desktop. A host that captures the compositor would
  report `source: compositor` and the badge would read **live desktop**.

The `#remote` panel on the marketing page is a demo of the same client state
machine with synthetic frames; it does not connect to a host.

## Task manager

Open from the palette: `Ctrl+Alt+Space` (Simple) or `Super+Space`, type **task**,
press Enter.

![Task manager](/img/task-manager.png)

| Input | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Processes / Users / Resources |
| `Up` / `Down` | Select a process |
| `PgUp` / `PgDn` | Page |
| `Space` or `Left` / `Right` | Collapse / expand the process tree |
| `R` | Force a refresh |
| `Esc` | Close |

The process tab shows the parent/child tree, owner, CPU, memory, state and
command. Users aggregates by UID. Resources shows CPU cores, memory/swap,
network rates, disk rates, load and uptime. The surface refreshes from `/proc`
once per second while open.

## Terminal

`Super+Return` launches `aether-terminal` immediately—no palette step.

The terminal is foot with the Water palette, DejaVu Sans Mono at 11 px and a dark
1 px border. If `AETHER_TERMINAL` is set, that command is used instead. The
palette's **New terminal** action does the same thing.

## Recovery and layer rules

- `Esc` closes exactly one overlay. It never closes two layers at once.
- `Super+Space`, `Super+E`, `Super+C`, `Super+A`, `Super+T` and `Super+R`
  replace each other; the last one wins.
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
