//! Remote-session overlay.

use aether_geometry::{Point, Rect};

use crate::design;
use crate::remote;

use super::Shell;

impl Shell {
    /// Open the remote-session overlay. The backend supplies a waiting layer
    /// from the session service, or the deterministic demo fixture.
    pub fn open_remote(&mut self, layer: remote::RemoteLayer) -> Option<()> {
        self.close_overlay();
        self.close_explorer();
        self.close_chat();
        self.close_audio();
        self.close_themes();
        self.remote = Some(layer);
        Some(())
    }

    pub fn close_remote(&mut self) {
        self.remote = None;
    }

    pub fn remote_open(&self) -> bool {
        self.remote.is_some()
    }

    pub fn remote_layer(&self) -> Option<&remote::RemoteLayer> {
        self.remote.as_ref()
    }

    pub fn remote_layer_mut(&mut self) -> Option<&mut remote::RemoteLayer> {
        self.remote.as_mut()
    }

    pub fn remote_rect(&self) -> Rect {
        let bounds = self
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport));
        let width = design::REMOTE_PANEL_WIDTH
            .min(bounds.width.saturating_sub(32))
            .max(1);
        let height = design::REMOTE_PANEL_HEIGHT
            .min(bounds.height.saturating_sub(32))
            .max(1);
        Rect::new(
            bounds.x + (bounds.width.saturating_sub(width) / 2) as i32,
            bounds.y + (bounds.height.saturating_sub(height) / 2) as i32,
            width,
            height,
        )
    }

    pub fn remote_action_rect(&self, index: usize) -> Option<Rect> {
        if index >= remote::RemoteAction::ALL.len() {
            return None;
        }
        let rect = self.remote_rect();
        Some(Rect::new(
            rect.x + 18,
            rect.y + design::REMOTE_ROWS_TOP + index as i32 * design::REMOTE_ROW_SPACING,
            rect.width.saturating_sub(36),
            design::REMOTE_ROW_HEIGHT,
        ))
    }

    pub fn remote_action_at(&self, point: Point) -> Option<usize> {
        (0..remote::RemoteAction::ALL.len()).find(|index| {
            self.remote_action_rect(*index)
                .is_some_and(|rect| rect.contains(point))
        })
    }

    pub fn remote_move_selection(&mut self, delta: isize) -> bool {
        self.remote
            .as_mut()
            .is_some_and(|layer| layer.move_selection(delta))
    }

    pub fn remote_select(&mut self, index: usize) -> bool {
        self.remote
            .as_mut()
            .is_some_and(|layer| layer.select(index))
    }

    pub fn remote_activate(&mut self) -> Option<remote::RemoteAction> {
        self.remote.as_mut().and_then(|layer| layer.activate())
    }

    pub fn remote_status(&self) -> Option<String> {
        self.remote.as_ref().and_then(|layer| layer.status_label())
    }
}
