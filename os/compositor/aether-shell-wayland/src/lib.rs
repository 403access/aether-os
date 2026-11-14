//! Backend-agnostic Wayland shell state, input policy and composition.
//!
//! The winit development adapter and the native DRM backend both create one
//! [`App`], register outputs, feed input events and draw the element list
//! returned by [`App::compose`].

mod agents;
mod app;
mod chat_demo;
mod damage;
mod index_worker;
mod render;
mod stats;
mod status;

pub use app::App;
pub use index_worker::{should_apply, IndexWorker};
pub use render::{AetherRenderElement, ComposedFrame, BACKGROUND};
pub use stats::{append as append_stat, damage_area};
pub use status::{StatusSnapshot, SAMPLE_INTERVAL};
