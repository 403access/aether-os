//! Facade for the Aether compositor's reusable core.
//!
//! The stack is split into small crates so applications and the shell can take
//! only what they need:
//!
//! ```text
//! aether-geometry   points, sizes, rectangles (no dependencies)
//! aether-damage     damage regions (geometry)
//! aether-layout     tiling (geometry)
//! aether-pacing     frame deadlines (no dependencies)
//! aether-scene      surfaces + z-order + hit testing (geometry, damage)
//! aether-compositor this facade + demo binary
//! ```
//!
//! Prefer individual crates in apps that only need one concern. Use
//! [`prelude`] or this facade in shell integration and examples.
//!
//! The GPU/Wayland (Smithay) integration is P2 and will live in a separate
//! crate that depends on this one; none of these crates link a compositor
//! toolkit.

#![forbid(unsafe_code)]

pub use aether_damage::{Region, MAX_RECTS};
pub use aether_geometry::{Point, Rect, Size};
pub use aether_layout::{tile, LayoutConfig};
pub use aether_pacing::{FramePacer, PacingReport, NS_PER_SEC};
pub use aether_scene::{Layer, Scene, Surface, SurfaceId};

/// Version of the scene model, independent of crate releases.
pub const SCENE_MODEL_VERSION: u32 = 1;

/// Everything an application or shell needs for scene composition.
pub mod prelude {
    pub use crate::{
        tile, FramePacer, Layer, LayoutConfig, PacingReport, Point, Rect, Region, Scene, Size,
        Surface, SurfaceId, MAX_RECTS, NS_PER_SEC, SCENE_MODEL_VERSION,
    };
}
