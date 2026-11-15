//! Backend-agnostic Wayland shell state, input policy and composition.
//!
//! The winit development adapter and the native DRM backend both create one
//! [`App`], register outputs, feed input events and draw the element list
//! returned by [`App::compose`].
//!
//! Layout:
//!
//! - `app` — Wayland state, protocol handlers, input, overlays, workers and
//!   app-level status/damage bookkeeping.
//! - `render` — element wrapping, frame orchestration, chrome buffers and one
//!   module per compositor-owned surface.
//! - `stats` — newline-delimited frame statistics for the backends.

mod app;
mod render;
mod stats;

pub use app::index_worker::{should_apply, IndexWorker};
pub use app::status::{StatusSnapshot, SAMPLE_INTERVAL};
pub use app::App;
pub use render::{AetherRenderElement, ComposedFrame, BACKGROUND};
pub use stats::{append as append_stat, damage_area};
