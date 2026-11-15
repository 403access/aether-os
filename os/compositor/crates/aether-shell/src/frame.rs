//! Frame plans handed to compositor backends.

use aether_damage::Region;
use aether_geometry::{Rect, Size};
use aether_scene::{Layer, SurfaceId};

use crate::WindowId;

#[derive(Clone, Debug)]
pub struct FrameItem {
    pub surface: SurfaceId,
    pub window: Option<WindowId>,
    pub rect: Rect,
    pub layer: Layer,
    pub title: String,
    pub focused: bool,
}

/// What a backend should draw for one frame. `damage` is the region that
/// changed since the previous present.
#[derive(Clone, Debug)]
pub struct FramePlan {
    pub viewport: Size,
    pub damage: Region,
    pub items: Vec<FrameItem>,
    pub focus: Option<WindowId>,
    pub workspace: usize,
    pub presented_ns: u64,
}
