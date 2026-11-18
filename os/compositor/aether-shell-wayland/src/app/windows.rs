//! Window configuration, focus mirroring and workspace policy.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use aether_shell::{OutputId, WindowId};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::utils::Size as SSize;

use super::App;

/// Height of the compositor-drawn title bar above a client surface.
/// Matches `.os-titlebar` in the Water OS demo.
pub(crate) const WINDOW_FRAME_HEIGHT: i32 = 40;
/// Traffic-light diameter and the gap between them (`.os-light`).
pub(crate) const WINDOW_LIGHT_SIZE: i32 = 11;
const WINDOW_LIGHT_GAP: i32 = 6;
/// Distance from the window edge to the first light (`.os-titlebar` padding).
const WINDOW_LIGHT_INSET: i32 = 11;

/// Which part of a client window frame a pointer landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameHit {
    Title,
    Close,
    Minimize,
    Maximize,
}

impl App {
    fn window_info(&self, id: WindowId) -> Option<aether_shell::WindowInfo> {
        self.shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
    }

    /// Client surface rect: the shell rect minus the server-side frame.
    pub(crate) fn window_content_rect(&self, id: WindowId) -> Option<AetherRect> {
        let window = self.window_info(id)?;
        let inset = if window.fullscreen {
            0
        } else {
            WINDOW_FRAME_HEIGHT
        };
        Some(AetherRect::new(
            window.rect.x,
            window.rect.y + inset,
            window.rect.width,
            window.rect.height.saturating_sub(inset as u32),
        ))
    }

    /// The three traffic lights of a framed window, left to right:
    /// close, minimise, maximise.
    pub(crate) fn window_light_rects(&self, id: WindowId) -> Option<[AetherRect; 3]> {
        let window = self.window_info(id)?;
        if window.fullscreen {
            return None;
        }
        let y = window.rect.y + (WINDOW_FRAME_HEIGHT - WINDOW_LIGHT_SIZE) / 2;
        let size = WINDOW_LIGHT_SIZE as u32;
        let step = WINDOW_LIGHT_SIZE + WINDOW_LIGHT_GAP;
        Some([
            AetherRect::new(window.rect.x + WINDOW_LIGHT_INSET, y, size, size),
            AetherRect::new(window.rect.x + WINDOW_LIGHT_INSET + step, y, size, size),
            AetherRect::new(window.rect.x + WINDOW_LIGHT_INSET + step * 2, y, size, size),
        ])
    }

    /// Whether a point is on a client window's title bar or one of its lights.
    pub(crate) fn window_frame_hit(&self, point: AetherPoint) -> Option<(WindowId, FrameHit)> {
        let id = self.shell.window_at(point)?;
        let window = self.window_info(id)?;
        if window.fullscreen {
            return None;
        }
        if let Some(lights) = self.window_light_rects(id) {
            for (rect, hit) in
                lights
                    .iter()
                    .zip([FrameHit::Close, FrameHit::Minimize, FrameHit::Maximize])
            {
                // A light is a small target: grow it by 3 px on every side so
                // it is reachable without changing the drawn size.
                let target =
                    AetherRect::new(rect.x - 3, rect.y - 3, rect.width + 6, rect.height + 6);
                if target.contains(point) {
                    return Some((id, hit));
                }
            }
        }
        (point.y < window.rect.y + WINDOW_FRAME_HEIGHT).then_some((id, FrameHit::Title))
    }

    pub(crate) fn switch_workspace(&mut self, workspace: usize) {
        if self.shell.switch_workspace(workspace) {
            println!("aether-shell: workspace -> {workspace}");
            self.sync_focus();
            self.configure_all();
            self.mark_full_damage();
        }
    }

    pub(crate) fn switch_workspace_on(&mut self, output: OutputId, workspace: usize) {
        if self.shell.switch_workspace_on(output, workspace) {
            println!("aether-shell: output {output} workspace -> {workspace}");
            self.sync_focus();
            self.configure_all();
            self.mark_full_damage();
        }
    }

    pub(crate) fn configure_window(&mut self, id: WindowId) {
        let Some(toplevel) = self.windows.get(&id).cloned() else {
            return;
        };
        let Some(rect) = self.window_content_rect(id) else {
            return;
        };
        let size = (rect.width.max(1) as i32, rect.height.max(1) as i32);
        let fullscreen = self.shell.is_fullscreen(id);
        let config = (size.0, size.1, fullscreen);
        if self.configured.get(&id) == Some(&config) {
            return;
        }
        self.configured.insert(id, config);
        toplevel.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            if fullscreen {
                state.states.set(xdg_toplevel::State::Fullscreen);
            } else {
                state.states.unset(xdg_toplevel::State::Fullscreen);
            }
            state.size = Some(SSize::from(size));
        });
        toplevel.send_configure();
    }

    pub fn configure_all(&mut self) {
        let ids: Vec<WindowId> = self.windows.keys().copied().collect();
        for id in ids {
            self.configure_window(id);
        }
    }

    pub(crate) fn sync_focus(&mut self) {
        let next = self.shell.focused_window().and_then(|id| {
            self.windows
                .get(&id)
                .map(|toplevel| toplevel.wl_surface().clone())
        });
        if next != self.focus {
            self.focus = next;
            // Frame emphasis follows focus, so both the old and new title bars
            // need a repaint even when no client buffer changes.
            self.mark_full_damage();
        }
    }

    pub(crate) fn window_rect(&self, id: WindowId) -> Option<AetherRect> {
        self.shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| window.rect)
    }
}
