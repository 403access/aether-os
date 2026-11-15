//! Floating, maximize, fullscreen and snapping geometry.

use aether_geometry::{Point, Rect};

use crate::model::window::{SnapZone, MIN_FLOATING_HEIGHT, MIN_FLOATING_WIDTH};
use crate::{OutputId, WindowId};

use super::{Shell, WindowRestore};

impl Shell {
    pub fn set_floating(&mut self, id: WindowId, floating: bool) -> bool {
        let Some((surface, current_floating, preferred)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.floating, window.spec.preferred_size))
        else {
            return false;
        };
        if current_floating == floating {
            return true;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let output_bounds = self.output_bounds(output);
        let floating_rect = if floating {
            if current_floating {
                self.scene.get(surface).map(|surface| surface.rect)
            } else {
                Some(self.floating_rect(0, preferred, output_bounds))
            }
        } else {
            None
        };
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = floating;
            window.maximized = false;
            window.restore = None;
            window.floating_rect = floating_rect;
        }
        self.relayout();
        true
    }

    /// Start an interactive move. Promotes a tiled window to its preferred
    /// floating size, centred on the pointer, and returns the new rect.
    pub fn begin_floating_move(&mut self, id: WindowId, pointer: Point) -> Option<Rect> {
        let window = self.windows.get(&id)?;
        if window.maximized {
            return None;
        }
        let surface = window.surface;
        let floating = window.floating;
        let preferred = window.spec.preferred_size;
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let rect = if floating {
            self.scene.get(surface)?.rect
        } else {
            let bounds = self
                .output_bounds(output)
                .inset(self.layout.outer_gap as i32);
            let width = preferred.width.min(bounds.width).max(1);
            let height = preferred.height.min(bounds.height).max(1);
            self.clamp_rect_to(
                Rect::new(
                    pointer.x - width as i32 / 2,
                    pointer.y - height as i32 / 2,
                    width,
                    height,
                ),
                self.output_bounds(output),
            )
        };
        let window = self.windows.get_mut(&id)?;
        window.floating = true;
        window.maximized = false;
        window.restore = None;
        window.floating_rect = Some(rect);
        self.relayout();
        Some(rect)
    }

    pub fn toggle_floating(&mut self, id: WindowId) -> bool {
        let floating = self.windows.get(&id).map(|w| w.floating);
        match floating {
            Some(current) => self.set_floating(id, !current),
            None => false,
        }
    }

    pub fn is_maximized(&self, id: WindowId) -> bool {
        self.windows.get(&id).is_some_and(|window| window.maximized)
    }

    pub fn is_fullscreen(&self, id: WindowId) -> bool {
        self.windows
            .get(&id)
            .is_some_and(|window| window.fullscreen)
    }

    /// Fullscreen window visible on an output, if any.
    pub fn fullscreen_window_on(&self, output: OutputId) -> Option<WindowId> {
        self.visible_windows_on(output)
            .into_iter()
            .find(|id| self.is_fullscreen(*id))
    }

    /// Fullscreen window on the primary output, if any.
    pub fn fullscreen_window(&self) -> Option<WindowId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.fullscreen_window_on(primary)
    }

    /// Toggle a window between its previous state and full-output fullscreen.
    pub fn toggle_fullscreen(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let (surface, fullscreen, floating, floating_rect) = (
            window.surface,
            window.fullscreen,
            window.floating,
            window.floating_rect,
        );

        if fullscreen {
            if let Some(window) = self.windows.get_mut(&id) {
                window.fullscreen = false;
                if let Some(restore) = window.restore.take() {
                    window.floating = restore.floating;
                    window.floating_rect = restore.floating_rect;
                    window.maximized = false;
                }
            }
        } else {
            let current = self.scene.get(surface).map(|surface| surface.rect);
            if let Some(window) = self.windows.get_mut(&id) {
                window.restore = Some(WindowRestore {
                    floating,
                    floating_rect,
                });
                window.fullscreen = true;
                window.maximized = false;
                window.floating = false;
                window.floating_rect = current;
            }
        }
        self.relayout();
        true
    }

    /// Toggle a window between its floating/tiled rect and a full viewport.
    pub fn toggle_maximized(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let (surface, maximized, floating, floating_rect) = (
            window.surface,
            window.maximized,
            window.floating,
            window.floating_rect,
        );

        if maximized {
            if let Some(window) = self.windows.get_mut(&id) {
                window.maximized = false;
                if let Some(restore) = window.restore.take() {
                    window.floating = restore.floating;
                    window.floating_rect = restore.floating_rect;
                } else {
                    window.floating = false;
                    window.floating_rect = None;
                }
            }
        } else {
            let current = self.scene.get(surface).map(|surface| surface.rect);
            if let Some(window) = self.windows.get_mut(&id) {
                window.restore = Some(WindowRestore {
                    floating,
                    floating_rect,
                });
                window.maximized = true;
                window.floating = false;
                window.floating_rect = current;
            }
        }
        self.relayout();
        true
    }

    /// Move a window on screen, promoting it to floating if necessary.
    pub fn move_floating(&mut self, id: WindowId, position: Point) -> bool {
        let Some((surface, blocked)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.maximized || window.fullscreen))
        else {
            return false;
        };
        if blocked {
            return false;
        }
        let Some(current) = self.scene.get(surface).map(|surface| surface.rect) else {
            return false;
        };
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let bounds = self.output_bounds(output);
        let moved = self.clamp_rect_to(
            Rect::new(position.x, position.y, current.width, current.height),
            bounds,
        );
        let moved = self.snap_floating(moved, bounds);
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.floating_rect = Some(moved);
        }
        self.relayout();
        true
    }

    /// Start an interactive resize. Promotes a tiled window to floating while
    /// keeping its current rect, and returns that rect.
    pub fn begin_floating_resize(&mut self, id: WindowId) -> Option<Rect> {
        let window = self.windows.get(&id)?;
        if window.maximized || window.fullscreen {
            return None;
        }
        let surface = window.surface;
        let rect = self.scene.get(surface)?.rect;
        let window = self.windows.get_mut(&id)?;
        window.floating = true;
        window.maximized = false;
        window.restore = None;
        window.floating_rect = Some(rect);
        self.relayout();
        Some(rect)
    }

    /// Snap a window to a half or maximize it. Halves stay floating so they can
    /// be moved or resized afterwards.
    pub fn snap_window(&mut self, id: WindowId, zone: SnapZone) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        if window.maximized {
            return false;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let viewport = self.output_bounds(output);
        if zone == SnapZone::Maximize {
            let was_maximized = self.is_maximized(id);
            return if was_maximized {
                true
            } else {
                self.toggle_maximized(id)
            };
        }
        let half = viewport.width / 2;
        let rect = match zone {
            SnapZone::LeftHalf => Rect::new(viewport.x, viewport.y, half, viewport.height),
            SnapZone::RightHalf => Rect::new(
                viewport.x + half as i32,
                viewport.y,
                viewport.width - half,
                viewport.height,
            ),
            SnapZone::Maximize => unreachable!(),
        };
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.maximized = false;
            window.restore = None;
            window.floating_rect = Some(rect);
        }
        self.relayout();
        true
    }

    /// Resize a floating window, enforcing the minimum size and output bounds.
    pub fn resize_floating(&mut self, id: WindowId, rect: Rect) -> bool {
        let Some((surface, blocked)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.maximized || window.fullscreen))
        else {
            return false;
        };
        if blocked {
            return false;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let bounds = self.output_bounds(output);
        let width = rect
            .width
            .max(MIN_FLOATING_WIDTH.min(bounds.width.max(1)))
            .min(bounds.width.max(1));
        let height = rect
            .height
            .max(MIN_FLOATING_HEIGHT.min(bounds.height.max(1)))
            .min(bounds.height.max(1));
        let resized = self.clamp_rect_to(Rect::new(rect.x, rect.y, width, height), bounds);
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.floating_rect = Some(resized);
        }
        if self.scene.get(surface).is_some() {
            self.relayout();
        }
        true
    }
}
