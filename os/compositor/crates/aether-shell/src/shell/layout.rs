//! Tiling and floating layout updates.

use aether_geometry::{Rect, Size};
use aether_layout::{tile, work_area};
use aether_scene::SurfaceId;

use crate::model::window::FLOATING_SNAP_DISTANCE;
use crate::WindowId;

use super::Shell;

impl Shell {
    pub(super) fn relayout(&mut self) {
        let outputs = self.outputs.clone();
        let mut visible_focus = false;

        for output in &outputs {
            let workspace = self.output_workspace(output.id);
            let ids = self.workspaces[workspace].windows.clone();

            let tiled: Vec<(WindowId, SurfaceId)> = ids
                .iter()
                .filter_map(|id| {
                    let window = self.windows.get(id)?;
                    (window.output == output.id
                        && !window.floating
                        && !window.maximized
                        && !window.fullscreen)
                        .then_some((*id, window.surface))
                })
                .collect();
            let rects = tile(output.rect, tiled.len(), self.layout);
            for ((_, surface), rect) in tiled.iter().zip(rects) {
                self.scene.move_to(*surface, rect);
            }

            let floating: Vec<(SurfaceId, Option<Rect>, Size, bool, bool)> = ids
                .iter()
                .filter_map(|id| {
                    let window = self.windows.get(id)?;
                    (window.output == output.id
                        && (window.floating || window.maximized || window.fullscreen))
                        .then_some((
                            window.surface,
                            window.floating_rect,
                            window.spec.preferred_size,
                            window.maximized,
                            window.fullscreen,
                        ))
                })
                .collect();
            for (index, (surface, explicit, size, maximized, fullscreen)) in
                floating.into_iter().enumerate()
            {
                let rect = if fullscreen {
                    output.rect
                } else if maximized {
                    work_area(output.rect, self.layout)
                } else if let Some(rect) = explicit {
                    self.clamp_rect_to(rect, output.rect)
                } else {
                    self.floating_rect(index, size, output.rect)
                };
                self.scene.move_to(surface, rect);
            }

            if let Some(focus) = self.focus {
                if self.windows.get(&focus).is_some_and(|window| {
                    window.output == output.id && window.workspace == workspace
                }) {
                    visible_focus = true;
                }
            }
        }

        // Per-output workspaces: a window is visible only on its own output,
        // and only while that output shows the window's workspace.
        for (id, window) in &self.windows {
            let visible = self.outputs.iter().any(|output| output.id == window.output)
                && window.workspace == self.output_workspace(window.output);
            self.scene.set_visible(window.surface, visible);
            if visible && self.focus == Some(*id) {
                visible_focus = true;
            }
        }

        if let Some(focus) = self.focus {
            if let Some(window) = self.windows.get(&focus) {
                if visible_focus {
                    self.scene.raise(window.surface);
                }
            }
        }
    }

    pub(super) fn floating_rect(&self, index: usize, size: Size, output_bounds: Rect) -> Rect {
        // Centre inside the same work area tiled windows use, so a large
        // floating window does not open underneath the top dock.
        let bounds = work_area(output_bounds, self.layout);
        if bounds.is_empty() {
            return Rect::new(bounds.x, bounds.y, 0, 0);
        }
        let width = size.width.min(bounds.width).max(1);
        let height = size.height.min(bounds.height).max(1);
        let offset = (index as i32) * 32;
        let x = bounds.x + ((bounds.width - width) / 2) as i32 + offset;
        let y = bounds.y + ((bounds.height - height) / 2) as i32 + offset;
        self.clamp_rect_to(Rect::new(x, y, width, height), output_bounds)
    }

    pub(super) fn clamp_rect_to(&self, rect: Rect, bounds: Rect) -> Rect {
        if bounds.is_empty() || rect.is_empty() {
            return rect;
        }
        let width = rect.width.min(bounds.width).max(1);
        let height = rect.height.min(bounds.height).max(1);
        let max_x = (bounds.x + bounds.width as i32 - width as i32).max(bounds.x);
        let max_y = (bounds.y + bounds.height as i32 - height as i32).max(bounds.y);
        Rect::new(
            rect.x.clamp(bounds.x, max_x),
            rect.y.clamp(bounds.y, max_y),
            width,
            height,
        )
    }

    pub(super) fn clamp_size(&self, size: Size) -> Size {
        let bounds = self.output_bounds(self.primary_output().map(|o| o.id).unwrap_or(1));
        Size::new(
            size.width.clamp(1, bounds.width.max(1)),
            size.height.clamp(1, bounds.height.max(1)),
        )
    }

    pub(super) fn snap_floating(&self, rect: Rect, bounds: Rect) -> Rect {
        let mut x = rect.x;
        let mut y = rect.y;
        if x - bounds.x <= FLOATING_SNAP_DISTANCE {
            x = bounds.x;
        } else if (bounds.right() - (x as i64 + rect.width as i64)) <= FLOATING_SNAP_DISTANCE as i64
        {
            x = (bounds.width as i32 - rect.width as i32).max(0);
        }
        if y - bounds.y <= FLOATING_SNAP_DISTANCE {
            y = bounds.y;
        } else if (bounds.bottom() - (y as i64 + rect.height as i64))
            <= FLOATING_SNAP_DISTANCE as i64
        {
            y = (bounds.height as i32 - rect.height as i32).max(0);
        }
        Rect::new(x, y, rect.width, rect.height)
    }
}
