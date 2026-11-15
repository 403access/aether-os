//! Window descriptors, snap zones and floating-size limits.

use aether_geometry::{Rect, Size};

use crate::{OutputId, WindowId};

/// Minimum width when resizing a floating window.
pub const MIN_FLOATING_WIDTH: u32 = 160;
/// Minimum height when resizing a floating window.
pub const MIN_FLOATING_HEIGHT: u32 = 120;
/// Distance from an output edge where a moving window snaps flush.
pub const FLOATING_SNAP_DISTANCE: i32 = 16;

#[derive(Clone, Debug)]
pub struct WindowSpec {
    pub title: String,
    pub app_id: String,
    pub preferred_size: Size,
    pub floating: bool,
}

impl WindowSpec {
    pub fn new(title: impl Into<String>, app_id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            app_id: app_id.into(),
            preferred_size: Size::new(900, 700),
            floating: false,
        }
    }
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self::new("window", "application")
    }
}

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub workspace: usize,
    pub output: OutputId,
    pub rect: Rect,
    pub floating: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub focused: bool,
}

/// Edge zones a moving window can snap into on release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapZone {
    LeftHalf,
    RightHalf,
    Maximize,
}
