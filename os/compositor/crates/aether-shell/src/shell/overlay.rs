//! Modal palette overlay placement.

use aether_geometry::Rect;
use aether_scene::{Layer, SurfaceId};

use crate::OutputId;

use super::{PaletteState, Shell};

impl Shell {
    pub fn overlay_output(&self) -> Option<OutputId> {
        self.overlay_output
    }

    /// Open the overlay centred in `output` and reset the query.
    pub fn open_overlay_on(&mut self, output: OutputId) -> Option<SurfaceId> {
        self.close_remote();
        self.close_themes();
        self.palette = PaletteState::default();
        self.overlay_output = Some(output);
        let bounds = self.output_bounds(output);
        let width = 680.min(bounds.width.saturating_sub(64)).max(1);
        let height = 420.min(bounds.height.saturating_sub(64)).max(1);
        let x = bounds.x + ((bounds.width.saturating_sub(width)) / 2) as i32;
        let y = bounds.y + ((bounds.height.saturating_sub(height)) / 2) as i32;
        self.set_overlay(Some(Rect::new(x, y, width, height)))
    }

    /// Open the overlay on the primary output.
    pub fn open_overlay(&mut self) -> Option<SurfaceId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.open_overlay_on(primary)
    }

    /// Close the overlay and clear the palette query.
    pub fn close_overlay(&mut self) -> Option<SurfaceId> {
        self.palette = PaletteState::default();
        let removed = self.set_overlay(None);
        self.overlay_output = None;
        removed
    }

    /// Toggle the overlay on `output`. Returns whether it is now open.
    pub fn toggle_overlay_on(&mut self, output: OutputId) -> bool {
        if self.overlay.is_some() {
            self.close_overlay();
            false
        } else {
            self.open_overlay_on(output);
            true
        }
    }

    /// Toggle the overlay on the primary output.
    pub fn toggle_overlay(&mut self) -> bool {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.toggle_overlay_on(primary)
    }

    /// Create or update the single overlay surface (palette/chat). Passing
    /// `None` removes it.
    pub fn set_overlay(&mut self, rect: Option<Rect>) -> Option<SurfaceId> {
        let bounds = self.output_bounds(
            self.overlay_output
                .or_else(|| self.primary_output().map(|output| output.id))
                .unwrap_or(1),
        );
        match (rect, self.overlay) {
            (Some(rect), Some(surface)) => {
                self.scene
                    .move_to(surface, self.clamp_rect_to(rect, bounds));
                Some(surface)
            }
            (Some(rect), None) => {
                let surface = self
                    .scene
                    .add(Layer::OVERLAY, self.clamp_rect_to(rect, bounds));
                self.overlay = Some(surface);
                Some(surface)
            }
            (None, Some(surface)) => {
                self.scene.remove(surface);
                self.overlay = None;
                None
            }
            (None, None) => None,
        }
    }

    pub fn overlay(&self) -> Option<SurfaceId> {
        self.overlay
    }

    pub fn overlay_rect(&self) -> Option<Rect> {
        self.overlay
            .and_then(|surface| self.scene.get(surface).map(|surface| surface.rect))
    }
}
