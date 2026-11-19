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
pub mod design_manifest;
mod render;

#[cfg(feature = "scene-dump")]
mod scene;

/// Record a structural scene node; compiles to `0` without `scene-dump`.
#[cfg(feature = "scene-dump")]
#[macro_export]
macro_rules! scene_node {
    ($ui:expr, $parent:expr, $role:literal, $rect:expr) => {{
        let _ = &$ui;
        $crate::scene::push($parent, $role, None, $rect, None, 0.0)
    }};
}

/// Record a styled box (background + radius).
#[cfg(feature = "scene-dump")]
#[macro_export]
macro_rules! scene_box {
    ($ui:expr, $parent:expr, $role:literal, $rect:expr, $bg:expr, $radius:expr) => {{
        let _ = &$ui;
        $crate::scene::push($parent, $role, None, $rect, Some($bg), $radius)
    }};
}

/// Record a text node with its renderer text scale.
#[cfg(feature = "scene-dump")]
#[macro_export]
macro_rules! scene_text {
    ($ui:expr, $parent:expr, $role:literal, $text:expr, $rect:expr, $scale:expr, $color:expr) => {{
        let _ = &$ui;
        $crate::scene::push_text($parent, $role, $text, $rect, $scale, $color)
    }};
}

#[cfg(not(feature = "scene-dump"))]
#[macro_export]
macro_rules! scene_node {
    ($ui:expr, $parent:expr, $role:literal, $rect:expr) => {{
        let _: Option<usize> = $parent;
        let _ = (&$ui, &$rect);
        0
    }};
}

#[cfg(not(feature = "scene-dump"))]
#[macro_export]
macro_rules! scene_box {
    ($ui:expr, $parent:expr, $role:literal, $rect:expr, $bg:expr, $radius:expr) => {{
        let _: Option<usize> = $parent;
        let _ = (&$ui, &$rect, &$bg, &$radius);
        0
    }};
}

#[cfg(not(feature = "scene-dump"))]
#[macro_export]
macro_rules! scene_text {
    ($ui:expr, $parent:expr, $role:literal, $text:expr, $rect:expr, $scale:expr, $color:expr) => {{
        let _: Option<usize> = $parent;
        let _ = (&$ui, &$text, &$rect, &$scale, &$color);
        0
    }};
}
mod stats;

pub use app::index_worker::{should_apply, IndexWorker};
pub use app::status::{StatusSnapshot, SAMPLE_INTERVAL};
pub use app::App;
pub use render::{AetherRenderElement, ComposedFrame, BACKGROUND};
pub use stats::{append as append_stat, damage_area};
