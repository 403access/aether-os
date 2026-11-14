# Aether compositor core

A small Rust workspace that models the parts of a compositor that are easy to get
wrong — geometry, damage, z-order, tiling and frame pacing — with **no GPU,
Wayland or windowing dependency**. It builds and tests on any machine.

This is the stable foundation for the shell and for applications that want to
compose surfaces. The Smithay/Vulkan backend is P2 and will be a separate crate
that depends on this workspace; it must not leak into these crates.

## Crates

```text
aether-geometry      Point, Size, Rect                         (no deps)
aether-damage        Region, merging, clipping                  (geometry)
aether-layout        master/stack tiling                        (geometry)
aether-pacing        FramePacer, deadlines, missed frames       (no deps)
aether-scene         Surface, Layer, Scene, hit testing          (geometry, damage)
aether-shell         windows, workspaces, focus, FramePlan      (all core crates)
aether-text          8x8 bitmap text -> RGBA buffer              (font8x8)
aether-desktop       XDG .desktop discovery + icons/Exec         (std)
aether-files         bounded filename index for the palette      (std)
aether-cursor        software cursor bitmaps                     (std)
aether-preview       software renderer + PNG preview            (shell, text)
aether-compositor    facade + prelude + demo binary             (core crates)

aether-shell-smithay  Linux-only Smithay/winit backend (standalone crate,
                      not in this workspace; consumes FramePlan)
```

Dependency direction is one-way. There are no cycles and no optional GPU code.

| Crate | Use it when |
| --- | --- |
| `aether-geometry` | You only need rectangles and points |
| `aether-damage` | You track dirty regions for partial redraw |
| `aether-layout` | You place windows/tiles without a compositor |
| `aether-pacing` | You need a frame deadline scheduler |
| `aether-scene` | You model surfaces, z-order and hit testing |
| `aether-shell` | You need windows, workspaces, focus or frame plans |
| `aether-text` | You need cropped bitmap text as RGBA pixels |
| `aether-desktop` | You need to discover/parse launchable XDG apps and PNG icons |
| `aether-files` | You need a small bounded filename index |
| `aether-cursor` | You need backend-neutral cursor bitmaps |
| `aether-preview` | You want to render a frame to PNG without a display |
| `aether-compositor` | You want the facade/prelude, or run the demo |

## Design rules

1. **No platform dependency in the core.** No Smithay, Vulkan, libinput, x11 or
   winit. The backend crate is separate.
2. **Pure functions where possible.** Layout and pacing take values and return
   values; time is injected.
3. **No `unsafe`.** Every crate sets `#![forbid(unsafe_code)]`.
4. **Small public APIs.** A crate should be understandable in one sitting.
5. **Damage is conservative.** Over-damage is safe; under-damage is a bug.
6. **Stable model version.** `SCENE_MODEL_VERSION` is independent of crate
   versions and bumps only for incompatible scene semantics.

## Commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p aether-compositor --example hello_scene
cargo run -p aether-compositor --bin aether-scene-demo
cargo run -p aether-preview --bin aether-shell-preview
```

The last command writes `artifacts/shell/frame.png` and `damage.png`.

The Nix package installs the demo as `aether-scene`:

```bash
nix build .#aether-scene
```

## Building an app on top

Depend on the smallest crates you need. A typical app surface:

```toml
[dependencies]
aether-geometry = { path = "../../os/compositor/crates/aether-geometry" }
aether-scene = { path = "../../os/compositor/crates/aether-scene" }
aether-pacing = { path = "../../os/compositor/crates/aether-pacing" }
```

```rust
use aether_geometry::{Rect, Size};
use aether_pacing::FramePacer;
use aether_scene::{Layer, Scene};

let mut scene = Scene::new(Size::new(1920, 1080));
let surface = scene.add(Layer::WINDOW, Rect::new(0, 0, 960, 1080));
scene.add_damage(surface, Rect::new(16, 16, 320, 40));
let _damage = scene.commit();

let mut pacer = FramePacer::new(120);
pacer.on_present(0);
```

Use `aether-shell` for window/workspace policy and `aether-compositor::prelude::*`
for geometry/damage/scene helpers. Apps should not depend on the future Smithay
backend.

## Roadmap boundary

| Phase | Work |
| --- | --- |
| P0 (done) | Pure crates, shell state, preview, tests, demo, facade |
| P2 (started) | `aether-shell-smithay`: clients tile/configure, palette with refreshing workspace/window/app/file/external providers, clipboard, wl_output/xdg-output, text-input, fullscreen/window management, software cursors, idle skipping and surface/window damage; IME engine, live agent IPC, xcursor themes, buffer damage/occlusion, multi-output/hotplug and udev/DRM next |
| P2 | Text/atlas and scene-graph UI primitives |
| P3 | Shell layers (palette/chat) consume the same crates |

The backend crate depends on this workspace; this workspace must never depend on
it. That keeps apps testable without a display.

Build the backend only on Linux:

```bash
nix develop .#smithay
cargo build --manifest-path os/compositor/aether-shell-smithay/Cargo.toml
./os/compositor/aether-shell-smithay/smoke-xvfb.sh
```
