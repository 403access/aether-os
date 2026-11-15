//! Backend-neutral shell state.
//!
//! `aether-shell` owns windows, workspaces, focus, tiling and the overlay
//! surface. It produces [`FramePlan`]s for a backend to present. It does not
//! know about Wayland, DRM, GPU APIs or input devices, so it is fully testable
//! on any machine and can be driven by the Smithay backend on the target or by
//! the software preview renderer during development.
//!
//! ```text
//! inputs (keys, IPC) ──> Shell (this crate) ──> FramePlan ──> backend
//!                          │
//!                          └── aether-scene / aether-layout / aether-pacing
//! ```
//!
//! The compositor backend maps Wayland buffers into the scene; the shell only
//! decides which surfaces exist, where they are, and what changed.
//!
//! Layout:
//!
//! - `model` — backend-independent domain types and algorithms:
//!   - `output` — output descriptors and indicator geometry.
//!   - `window` — window specs, info, snap zones and limits.
//!   - `palette` — providers, actions, scoring and overlay constants.
//!   - `chat` — transcript model and markdown parsing.
//!   - `explorer` — virtualised explorer state.
//!   - `frame` — frame plans handed to backends.
//! - `theme` — Themes app settings and chrome tokens.
//! - `shell` — [`Shell`] plus focused implementation modules.

pub mod audio;
pub mod design;
pub mod remote;
pub mod theme;

mod model;
mod shell;

pub type WindowId = u64;
pub type OutputId = u64;

pub use model::chat::{ChatLine, ChatSpan, ChatSpanKind, ChatState};
pub use model::explorer::ExplorerState;
pub use model::frame::{FrameItem, FramePlan};
pub use model::output::{
    OutputInfo, WORKSPACE_INDICATOR_BAR_HEIGHT, WORKSPACE_INDICATOR_HIT_HEIGHT,
    WORKSPACE_INDICATOR_SPACING, WORKSPACE_INDICATOR_WIDTH, WORKSPACE_INDICATOR_X,
    WORKSPACE_INDICATOR_Y,
};
pub use model::palette::{
    is_subsequence, parse_palette_query, PaletteAction, PaletteEntry, PaletteFile, PaletteLauncher,
    PaletteMode, PALETTE_CONTENT_TOP, PALETTE_ROW_HEIGHT, PALETTE_ROW_SPACING,
    PALETTE_VISIBLE_ROWS,
};
pub use model::window::{
    SnapZone, WindowInfo, WindowSpec, FLOATING_SNAP_DISTANCE, MIN_FLOATING_HEIGHT,
    MIN_FLOATING_WIDTH,
};
pub use shell::Shell;
