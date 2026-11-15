//! Window insertion, closing, queries and damage.

use aether_geometry::{Point, Rect};
use aether_scene::Layer;

use crate::window::{WindowInfo, WindowSpec};
use crate::{OutputId, WindowId};

use super::{Shell, Window};

impl Shell {
    /// Update the title used for window lists and the palette.
    pub fn set_window_title(&mut self, id: WindowId, title: impl Into<String>) -> bool {
        match self.windows.get_mut(&id) {
            Some(window) => {
                window.spec.title = title.into();
                true
            }
            None => false,
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Add a window directly on an output's currently visible workspace.
    /// New clients never appear on a workspace that output is not showing.
    pub fn add_window_on_output(&mut self, spec: WindowSpec, output: OutputId) -> WindowId {
        if !self.outputs.iter().any(|candidate| candidate.id == output) {
            return self.add_window(spec);
        }
        let target = self.output_workspace(output);
        let id = self.add_window(spec);
        if let Some(window) = self.windows.get_mut(&id) {
            let previous = window.workspace;
            if previous != target {
                if let Some(workspace) = self.workspaces.get_mut(previous) {
                    workspace.windows.retain(|candidate| *candidate != id);
                }
                if let Some(workspace) = self.workspaces.get_mut(target) {
                    workspace.windows.push(id);
                }
                window.workspace = target;
            }
            window.output = output;
        }
        self.relayout();
        id
    }

    pub fn add_window(&mut self, spec: WindowSpec) -> WindowId {
        let id = self.next_id;
        self.next_id += 1;

        let size = self.clamp_size(spec.preferred_size);
        let floating = spec.floating;
        let rect = Rect::new(0, 0, size.width, size.height);
        let surface = self.scene.add(Layer::WINDOW, rect);
        let workspace = self.active;
        let output = self.primary_output().map(|output| output.id).unwrap_or(1);

        self.windows.insert(
            id,
            Window {
                spec,
                workspace,
                output,
                surface,
                floating,
                floating_rect: None,
                maximized: false,
                fullscreen: false,
                restore: None,
            },
        );
        self.workspaces[workspace].windows.push(id);
        self.focus = Some(id);
        self.relayout();
        id
    }

    pub fn close_window(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.remove(&id) else {
            return false;
        };
        self.scene.remove(window.surface);
        if let Some(workspace) = self.workspaces.get_mut(window.workspace) {
            workspace.windows.retain(|candidate| *candidate != id);
        }
        self.focus_history.retain(|candidate| *candidate != id);
        if self.focus == Some(id) {
            self.focus = self.workspaces[self.active].windows.last().copied();
            self.focus_cycle = None;
        }
        self.relayout();
        true
    }

    pub fn window_at(&self, point: Point) -> Option<WindowId> {
        let surface = self.scene.hit_test(point)?;
        if Some(surface) == self.overlay {
            return None;
        }
        self.windows
            .iter()
            .find_map(|(id, window)| (window.surface == surface).then_some(*id))
    }

    /// Window ids in compositor draw order, topmost first.
    pub fn window_stack(&self) -> Vec<WindowId> {
        self.scene
            .draw_order()
            .into_iter()
            .rev()
            .filter_map(|surface| {
                self.windows
                    .iter()
                    .find_map(|(id, window)| (window.surface == surface).then_some(*id))
            })
            .collect()
    }

    pub fn windows(&self) -> Vec<WindowInfo> {
        let mut infos: Vec<WindowInfo> = self
            .windows
            .iter()
            .map(|(id, window)| WindowInfo {
                id: *id,
                title: window.spec.title.clone(),
                app_id: window.spec.app_id.clone(),
                workspace: window.workspace,
                output: window.output,
                rect: self
                    .scene
                    .get(window.surface)
                    .map(|surface| surface.rect)
                    .unwrap_or_default(),
                floating: window.floating,
                maximized: window.maximized,
                fullscreen: window.fullscreen,
                focused: self.focus == Some(*id),
            })
            .collect();
        infos.sort_by_key(|window| window.id);
        infos
    }

    /// Simulate content damage for a window (text append, resize of app
    /// content). Clipped to the surface by the scene.
    pub fn damage_window(&mut self, id: WindowId, rect: Rect) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        self.scene.add_damage(window.surface, rect)
    }
}
