//! Window configuration, focus mirroring and workspace policy.

use aether_geometry::Rect as AetherRect;
use aether_shell::{OutputId, WindowId};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::utils::Size as SSize;

use super::App;

impl App {
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
        let Some(rect) = self
            .shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| window.rect)
        else {
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
        self.focus = self.shell.focused_window().and_then(|id| {
            self.windows
                .get(&id)
                .map(|toplevel| toplevel.wl_surface().clone())
        });
    }

    pub(crate) fn window_rect(&self, id: WindowId) -> Option<AetherRect> {
        self.shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| window.rect)
    }
}
