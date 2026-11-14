//! Backend-agnostic Wayland shell state, input policy and composition.
//!
//! The winit development adapter and the native DRM backend both create one
//! [`App`], register outputs, feed input events and draw the element list
//! returned by [`App::compose`].

mod agents;
mod app;
mod damage;
mod render;

pub use app::App;
pub use render::{AetherRenderElement, ComposedFrame, BACKGROUND};
