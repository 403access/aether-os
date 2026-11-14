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
- **Interactive palette with providers.** Results currently come from
  workspaces and client windows (titles are kept in sync via xdg
  `title_changed`). Typing filters them (case-insensitive prefix/substring),
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

## What is not implemented

- **More palette providers.** Workspaces, windows and close exist; apps, files
  and agent sessions do not.
- **Text input methods.** Printable keys are mapped from keysyms; there is no
  `text-input`/IME protocol support yet.
- **Text cache/atlas.** Every frame rebuilds small RGBA text buffers; a cached
  atlas is needed before text-heavy apps.
- **Cursor theming** and hover cursor feedback.
- **Interactive window move**, maximize/fullscreen, popups and drag-and-drop are
  ignored. Tiled divider resizing works; floating move does not.
- **Per-client damage.** Full-window redraw remains.
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
```

The smoke scripts manage `XDG_RUNTIME_DIR`, Xvfb and the test client. The picker,
resize and palette smokes require `xdotool`, which the `.#smithay` dev shell
provides.

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
