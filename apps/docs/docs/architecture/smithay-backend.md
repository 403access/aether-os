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

![Smithay winit window presenting tiled shell surfaces and the overlay](/img/smithay-window.png)

*The winit backend under Xvfb/llvmpipe: four tiled surfaces and the overlay,
rendered from the same `FramePlan` as the software preview. Solid colours are
stand-ins until client buffers are mapped.*

## What works

- Opens a native window with Smithay's winit/EGL backend.
- Consumes `FramePlan`; z-order (windows → overlay) matches the shell.
- Resize and close events; damage tracker is recreated on resize.
- Runs under Xvfb with Mesa software EGL for headless verification.
- `--frames N` exits after N frames for automated smoke tests.

## What is not implemented

- **Wayland clients.** There is no `wl_display`, xdg-shell, shm or seat yet, so
  the window shows compositor-owned surfaces, not client applications.
- **Client buffer rendering.** Solid colour elements stand in for surfaces.
- **Input routing.** Keyboard/pointer events are not mapped to shell actions.
- **udev/DRM backend.** The target path for the 7700K (no X server) is not written;
  winit is the development adapter.
- **Text and chrome.** No glyph atlas yet.

The P2 gate remains: a real client window, 120 Hz pacing and input on the target.

## Build and run

Requires Linux with Nix (the dev shell supplies EGL, X11, Xvfb and Mesa):

```bash
nix develop .#smithay

# in the shell:
cargo build --manifest-path os/compositor/aether-shell-smithay/Cargo.toml

# on a normal X11/Wayland session:
./os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay

# headless verification (Xvfb + llvmpipe), writes artifacts/smithay/window.png:
./os/compositor/aether-shell-smithay/smoke-xvfb.sh
```

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
