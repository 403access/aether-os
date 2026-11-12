//! Aether compositor scene core.
//!
//! This crate is deliberately free of Wayland, GPU and windowing dependencies.
//! It models the parts that are easy to get wrong — geometry, damage, z-order,
//! tiling and frame pacing — so they can be tested in CI without a display.
//!
//! The Smithay integration lives in `aether-shell` (P2) and consumes these
//! types; it does not duplicate the logic.

#![forbid(unsafe_code)]

mod geometry;
mod layout;
mod pacer;
mod region;
mod scene;

pub use geometry::{Point, Rect, Size};
pub use layout::{tile, LayoutConfig};
pub use pacer::{FramePacer, PacingReport, NS_PER_SEC};
pub use region::{Region, MAX_RECTS};
pub use scene::{Layer, Scene, Surface, SurfaceId};

/// Version of the scene model, independent of the crate version.
pub const SCENE_MODEL_VERSION: u32 = 1;
