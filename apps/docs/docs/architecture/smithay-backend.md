---
id: smithay-backend
title: Smithay backend (Linux)
sidebar_position: 6
description: "The first real compositor backend: a Smithay winit window that presents FramePlan, plus what is still missing."
---

# Smithay backend (Linux)

`aether-shell-smithay` is the first native backend for the shell. It opens a real
Smithay/winit window, maps the shell's `FramePlan` to render elements and submits
frames at vsync.

It is **Linux-only** and deliberately a **standalone crate**: it is not in the
default Cargo workspace, so macOS development and CI never try to build Wayland
libraries. The shell and core crates remain toolkit-free.

```text
aether-shell (policy) ──> FramePlan ──> aether-shell-smithay (Linux)
                                   └── aether-preview (software, any OS)
```

![Two Wayland terminal clients tiled by the shell](/img/smithay-client.png)

*Two real `weston-terminal` xdg-shell clients with shared-memory buffers, tiled by
`aether-shell` and rendered by `aether-shell-smithay` under Xvfb.*

![The shell overlay above a client](/img/smithay-overlay.png)

*`Super+Space` toggles the modal command palette above clients. Header, search
row, workspace/window results and hints are real bitmap text rasterized by
`aether-text`; the labelled bars at top-left are workspace indicators (active is
bright). Titles come from the xdg-shell clients.*

![Filtering the palette by typing](/img/smithay-palette.png)

*Typing filters the palette. The selected row has a cyan edge and brighter
background; clicking it or pressing `Enter` runs it. `smoke-palette-xvfb.sh`
types `web`, captures this frame and clicks the row to switch workspace.*

![Filtering to a client window](/img/smithay-palette-window.png)

*The same palette lists client windows with their workspace. Activating the
result focuses the window and switches to its workspace; the smoke test types
`term`, presses `Enter` and asserts the return to workspace 1.*

![Clicking the second workspace indicator](/img/smithay-workspace-picker.png)

*Clicking a labelled indicator switches workspace; the active chip is cyan. The
capture is produced by `smoke-picker-xvfb.sh`, which drives the click with
`xdotool` and asserts the shell log.*

![Dragging the master/stack divider](/img/smithay-resize.png)

*Dragging the gap between the master and stack changes the tiling ratio;
`smoke-resize-xvfb.sh` drives the drag with `xdotool` and asserts the applied
ratio.*

![Alt-dragging a tiled client into a floating window](/img/smithay-window-move.png)

*`Alt+left-drag` promotes a tiled client to a floating window at its preferred
size, then follows the pointer. The other client re-tiles underneath; rendering
uses compositor z-order so the moved window stays on top.*

![Resizing the floating window from its corner](/img/smithay-window-resize.png)

*`Alt+right-drag` resizes a floating window from the nearest corner, with a
+160x120 minimum. Moving a window near an output edge snaps it flush. The smoke
+test shrinks the window from 800x600 to 620x500 and asserts the rect.*

![Dragging a window to the left edge snaps it to the left half](/img/smithay-window-snap.png)

*Releasing a moved window at an output edge snaps it: left/right become halves
and the top edge maximizes. Snapped halves stay floating, so they can be moved
or resized again. The smoke test asserts the 640x800 left-half rect.*

![Running the New terminal palette command](/img/smithay-new-terminal.png)

*The palette also exposes commands. "New terminal" launches
`$AETHER_TERMINAL` (default `weston-terminal`); the spawned client connects,
tiles and appears in the window provider.*

![Two clients after running New terminal](/img/smithay-new-terminal-open.png)

*`smoke-provider-xvfb.sh` types `new`, presses Enter, then asserts the action,
the spawned process and a second Wayland client connection.*

![Launching a discovered desktop application](/img/smithay-desktop-app.png)

*`aether-desktop` scans `XDG_DATA_HOME`/`XDG_DATA_DIRS` for `.desktop` entries and
registers them as palette launchers. The smoke points the data dirs at a fixture,
types `test`, and asserts the app launches and connects.*

![Searching the bounded file index in the palette](/img/smithay-file-provider.png)

*`aether-files` indexes filenames below `AETHER_FILE_ROOTS` (or `$HOME`),
skipping hidden entries and heavy build directories. The palette only shows file
results for non-empty queries; running one invokes `AETHER_OPENER` (default
`xdg-open`). The smoke uses a fixture tree and a deterministic opener script.*

![Copying in one terminal and pasting in another](/img/smithay-clipboard.png)

*Client-to-client clipboard works through `wl_data_device`: keyboard focus is
forwarded with `set_data_device_focus`, so the newly focused client receives the
selection offer. Primary selection uses the same focus forwarding via
`set_primary_focus`. The smoke copies with `wl-copy`/`wl-copy --primary` in one
terminal and pastes with `wl-paste`/`wl-paste --primary` in the other.*

![Partial damage from a terminal output commit](/img/smithay-partial-damage.png)

*A client commit damages only that window: the left tile redraws while the idle
right tile is untouched. Input-driven frames use full damage for safety.
`smoke-damage-xvfb.sh` schedules `sleep 1; echo ...` and asserts a partial frame
with the 681x768 master rect.*

![The same window maximized](/img/smithay-window-maximized.png)

*`Super+M` maximizes the focused window to the full output; pressing it again
restores the previous floating/tiled state. `smoke-window-xvfb.sh` drives the
drag and both toggles with `xdotool` and asserts each transition.*

![The `--demo` FramePlan view](/img/smithay-window.png)

*`--demo` mode: tiled shell surfaces and the overlay, rendered from the same
`FramePlan` as the software preview.*

## What works

- Opens a native window with Smithay's winit/EGL backend.
- Accepts Wayland clients over a listening socket (`aether-0`): xdg-shell,
  shared-memory buffers, compositor and seat.
- Maps each toplevel into `aether-shell` as a window, so clients are **tiled by
  the shell layout** and configured to their tile size. Two terminals are shown
  side-by-side in the screenshot.
- Renders client surfaces with Smithay surface elements at their shell rects.
- Pointer motion focuses the window under the cursor and forwards motion/buttons
  and axis; keyboard input goes to the focused surface.
- The shell overlay is drawn above clients (`Super+Space`) and is **modal**:
  keystrokes edit the palette instead of leaking to the focused client.
- **Interactive palette with providers.** Results come from workspaces, client
  windows (titles stay in sync via xdg `title_changed`) and commands. The first
  command, `New terminal`, spawns `AETHER_TERMINAL` (default
  `weston-terminal`) into the compositor. Installed applications are discovered
  from XDG `.desktop` files at startup and appear as `LaunchApp` entries.
  A bounded filename index (`aether-files`) adds `OpenFile` results for non-empty
  queries. Typing filters them (case-insensitive prefix/substring),
  `Up/Down` wraps, `Enter` or a row click activates, `Backspace` edits and `Esc`
  closes. The list slides to keep the selection visible, and pointer hover
  selects a row.
- Workspace switching and window moves via shortcuts, with three labelled
  workspace indicator bars at the top-left:
  `Super+1/2/3`, `Super+Shift+1/2/3`, `Super+Q` to close.
- **Clickable workspace picker.** Clicking an indicator chip (bar or label)
  switches workspace. Chrome consumes pointer motion/buttons, so clicks never
  leak to a client underneath. The seat advertises `wl_pointer`.
- **Interactive layout resize.** Dragging the gap between the master and stack
  updates the split ratio (clamped 0.2–0.8) and reconfigures clients live.
- **Client clipboard.** `wl_data_device` copy/paste and `zwp_primary_selection`
  work between clients. The compositor forwards data-device and primary focus
  with keyboard focus so selection offers reach the right client. DnD and
  clipboard persistence after the source exits are not implemented.
- **Idle frame skipping and partial damage.** Input events request a full
  redraw; client commits damage the surface rectangles they report, falling
  back to the whole window when no usable damage arrives (buffer-coordinate
  damage currently falls back). When nothing is dirty, the compositor stops
  presenting and stops sending frame callbacks. `AETHER_FRAME_LOG=1` logs each
  frame with `damage=full` or `damage=partial [...]`.
- **Pointer cursor feedback.** The winit window's system cursor changes with
  the hover target: `ColResize` on the master/stack divider, `Pointer` on
  workspace chips and palette rows, `Text` on the palette input, `Move` over a
  window while Alt is held, and diagonal resize cursors on floating corners.
  The cursor icon is only reapplied when it changes.
- **Window move, resize and maximize.** `Alt+left-drag` promotes a tiled window
  to its preferred floating size and moves it; floating positions persist across
  relayouts and snap flush to output edges while moving. Releasing at the left
  or right edge snaps a half; releasing at the top maximizes.
  `Alt+right-drag` resizes from the nearest corner with a minimum size.
  `Super+M` maximizes to the full output and restores the previous state. Client surfaces render in shell z-order, so
  raised/floating/maximized windows stay above others.
- Compositor chrome renders real text through `aether-text`: workspace names,
  overlay header/search/results/footer and shortcut hints. Bitmaps are uploaded
  as Smithay `MemoryRenderBuffer` elements above client surfaces.
- Sends frame callbacks, handles client connect/disconnect/destroy and window close.
- `--demo` mode presents the shell `FramePlan` as solid surfaces (z-order and
  overlay) for visual verification without clients.
- Runs under Xvfb with Mesa software EGL for headless verification.
- `--frames N` exits after N iterations for automated smoke tests.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Super+Space` | Toggle the shell overlay above clients |
| `Esc` | Close the overlay |
| *Typing* | Filter palette entries while the overlay is open |
| `Up/Down` | Move the palette selection (wraps) |
| `Enter` | Run the selected palette entry |
| `Backspace` | Delete a query character |
| *Click row* | Activate a palette result |
| `Super+1/2/3` | Switch workspace |
| `Super+Shift+1/2/3` | Move the focused client to a workspace |
| `Super+Q` | Close the focused client |
| `Super+M` | Toggle maximize for the focused window |
| `Alt+left-drag` | Move a window (promotes tiles to floating) |
| `Alt+right-drag` | Resize a floating window from the nearest corner |
| *Release at edge* | Snap to left/right half, or maximize at the top |

## What is not implemented

- **Agent-session provider and full-text files.** Desktop apps, the terminal
  command and a bounded filename index work; the system content indexer and an
  agent-session provider are not wired in yet.
- **Desktop-entry refresh, icons, actions and D-Bus activation.** The app list is
  scanned once at startup and only `Exec` is used.
- **Text input methods.** Printable keys are mapped from keysyms; there is no
  `text-input`/IME protocol support yet.
- **Text cache/atlas.** Every frame rebuilds small RGBA text buffers; a cached
  atlas is needed before text-heavy apps.
- **Cursor themes on the DRM backend.** The winit adapter uses system cursor
  icons; a DRM target needs client cursor images and an xcursor theme.
- **Fullscreen and multi-output edge snapping** are not implemented. Single
  output halves/maximize, floating resize and edge snapping work. Popups and
  drag-and-drop are ignored.
- **Buffer-coordinate damage and occlusion.** Surface-coordinate damage is used;
  buffer-coordinate damage falls back to the window, and opaque-region
  occlusion/per-subsurface tracking are not wired in yet.
- **Popups, drag-and-drop and grabs** are accepted/ignored at the protocol level.
- **udev/DRM backend.** The target path for the 7700K (no X server) is not written;
  winit is the development adapter.
- **Damage from client buffers** is tracked as a full-window redraw for now.

The P2 gate remains: a real client window, 120 Hz pacing and input on the target.

## Build and run

Requires Linux with Nix (the dev shell supplies EGL, X11, Xvfb and Mesa):

```bash
nix develop .#smithay

# in the shell:
cargo build --manifest-path os/compositor/aether-shell-smithay/Cargo.toml

# client-capable mode on a normal X11/Wayland session (prints the socket name):
./os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay

# launch a client against it in another shell:
WAYLAND_DISPLAY=aether-0 weston-terminal

# shell FramePlan demo (solid surfaces, no clients):
./os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay --demo

# show the overlay immediately (also Super+Space at runtime):
./os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay --overlay

# headless verification:
./os/compositor/aether-shell-smithay/smoke-xvfb.sh          # --demo
./os/compositor/aether-shell-smithay/smoke-client-xvfb.sh   # weston-terminal
./os/compositor/aether-shell-smithay/smoke-picker-xvfb.sh   # xdotool workspace click
./os/compositor/aether-shell-smithay/smoke-resize-xvfb.sh   # xdotool divider drag
./os/compositor/aether-shell-smithay/smoke-palette-xvfb.sh  # xdotool typed query
./os/compositor/aether-shell-smithay/smoke-window-xvfb.sh   # xdotool move + maximize
./os/compositor/aether-shell-smithay/smoke-cursor-xvfb.sh   # hover cursor mapping
./os/compositor/aether-shell-smithay/smoke-provider-xvfb.sh # spawn a client from the palette
./os/compositor/aether-shell-smithay/smoke-desktop-xvfb.sh  # launch a discovered .desktop app
./os/compositor/aether-shell-smithay/smoke-files-xvfb.sh    # search/open a fixture file
./os/compositor/aether-shell-smithay/smoke-clipboard-xvfb.sh # clipboard + primary between terminals
./os/compositor/aether-shell-smithay/smoke-idle-xvfb.sh      # idle frame skipping
./os/compositor/aether-shell-smithay/smoke-damage-xvfb.sh    # partial window damage
```

The smoke scripts manage `XDG_RUNTIME_DIR`, Xvfb and the test client. The picker,
resize, palette, window, cursor and provider smokes require `xdotool`, which the
`.#smithay` dev shell provides (`wl-copy`/`wl-paste` for the clipboard smoke and
the idle-frame smoke for `AETHER_FRAME_LOG`).
Set `AETHER_TERMINAL` to change the built-in
terminal launcher. Desktop apps come from `XDG_DATA_HOME`/`XDG_DATA_DIRS`
(`~/.local/share` and `/usr/local/share:/usr/share` by default). The file index
uses `AETHER_FILE_ROOTS` (default `$HOME`), `AETHER_FILE_LIMIT` (default 2000),
`AETHER_FILE_DEPTH` (default 5) and `AETHER_OPENER` (default `xdg-open`).

The environment sets:

```text
LIBGL_ALWAYS_SOFTWARE=1
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
__EGL_VENDOR_LIBRARY_FILENAMES=<mesa>/share/glvnd/egl_vendor.d/50_mesa.json
```

These are required under Xvfb; a normal desktop uses the hardware driver.

## Why a separate crate

Smithay pulls in Wayland, EGL, X11 and libinput. Adding it to the workspace would:

- break `cargo test --workspace` on macOS,
- make every app that uses `aether-shell` depend on a display stack,
- blur the line between policy (testable anywhere) and backend.

The standalone crate keeps `FramePlan` as the only contract. The winit adapter
now exercises the shell, client, input, overlay and text paths; the next backend
is udev/DRM for the target.
