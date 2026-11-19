//! Render element composition shared by every Aether backend.
//!
//! `App::compose` builds a single front-to-back element list covering client
//! surfaces, compositor chrome, palette icons/text, IME popups and cursors.
//! Winit draws the list into its GL framebuffer; the DRM backend hands the
//! same list to `DrmCompositor` for scan-out.
//!
//! Layout:
//!
//! - `element` — the concrete element wrapper and composed-frame output.
//! - `context` — per-frame state shared by the composition sections.
//! - `chrome` — cached icon, panel, rounded-fill and backdrop buffers.
//! - `compose` — frame orchestration, client surfaces, flush and cursor.
//! - `window_frame` — compositor-drawn title bar around client toplevels.
//! - `surfaces` — one module per compositor-owned surface.
//! - `helpers` — formatting, icon decoding and cursor hit-testing helpers.

mod chrome;
mod compose;
mod context;
mod element;
pub(crate) mod helpers;
pub(crate) mod surfaces;
pub(crate) mod ui;
mod window_frame;

pub(crate) use chrome::BackgroundStyle;
pub use element::{AetherRenderElement, ComposedFrame, BACKGROUND};
