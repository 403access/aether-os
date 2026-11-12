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
aether-compositor    facade + prelude + demo binary             (all of the above)
```

Dependency direction is one-way. There are no cycles and no optional GPU code.

| Crate | Use it when |
| --- | --- |
| `aether-geometry` | You only need rectangles and points |
| `aether-damage` | You track dirty regions for partial redraw |
| `aether-layout` | You place windows/tiles without a compositor |
| `aether-pacing` | You need a frame deadline scheduler |
| `aether-scene` | You model surfaces, z-order and hit testing |
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
```

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

Use `aether-compositor::prelude::*` in shell-side code and examples.

## Roadmap boundary

| Phase | Work |
| --- | --- |
| P0 (done) | Pure crates, tests, demo, facade |
| P2 | `aether-shell`: Smithay backend, renderer, input seat, DMA-BUF export |
| P2 | Text/atlas and scene-graph UI primitives |
| P3 | Shell layers (palette/chat) consume the same crates |

The backend crate may depend on this workspace; this workspace must never depend on
the backend. That keeps apps testable without a display.
