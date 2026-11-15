//! Output descriptors and workspace-indicator geometry.

use aether_geometry::Rect;

use crate::OutputId;

/// A logical output in the shell's extended desktop.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputInfo {
    pub id: OutputId,
    pub name: String,
    pub rect: Rect,
    pub scale: f64,
    pub refresh_hz: u32,
    pub physical_size_mm: (u32, u32),
    pub primary: bool,
}

/// Top-left of the first workspace indicator.
pub const WORKSPACE_INDICATOR_X: i32 = 10;
/// Top of the workspace indicator strip.
pub const WORKSPACE_INDICATOR_Y: i32 = 10;
/// Width of one indicator bar.
pub const WORKSPACE_INDICATOR_WIDTH: u32 = 66;
/// Height of the coloured bar itself.
pub const WORKSPACE_INDICATOR_BAR_HEIGHT: u32 = 6;
/// Horizontal distance between indicator origins.
pub const WORKSPACE_INDICATOR_SPACING: i32 = 76;
/// Hit height including the label below the bar.
pub const WORKSPACE_INDICATOR_HIT_HEIGHT: u32 = 26;
