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
- Pointer motion focuses the window under the cursor and forwards motion/buttons;
  keyboard input goes to the focused surface.
- Sends frame callbacks, handles client connect/disconnect/destroy and window close.
- `--demo` mode presents the shell `FramePlan` as solid surfaces (z-order and
  overlay) for visual verification without clients.
- Runs under Xvfb with Mesa software EGL for headless verification.
- `--frames N` exits after N iterations for automated smoke tests.

## What is not implemented

- **Workspaces have no UI/shortcuts yet.** Clients join the active workspace and
  tile, but there is no keybinding/overlay to switch workspaces.
- **No shell overlay in client mode.** The palette/chat overlay is only drawn in
  `--demo`; integrating it above clients is next.
- **Cursor theming and axis** are not handled; pointer motion/buttons are basic.
- **Interactive move/resize, popups and drag-and-drop** are ignored.
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

# headless verification:
./os/compositor/aether-shell-smithay/smoke-xvfb.sh          # --demo
./os/compositor/aether-shell-smithay/smoke-client-xvfb.sh   # weston-terminal
```

The smoke scripts manage `XDG_RUNTIME_DIR`, Xvfb and the test client.

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

The standalone crate keeps `FramePlan` as the only contract. The next step is to
replace solid elements with compositor surface render elements and add the
xdg-shell/shm/seat handlers, then a udev/DRM backend for the target.
