//! Workspace creation, switching and indicators.

use aether_geometry::{Point, Rect};

use crate::output::{
    WORKSPACE_INDICATOR_HIT_HEIGHT, WORKSPACE_INDICATOR_SPACING, WORKSPACE_INDICATOR_WIDTH,
    WORKSPACE_INDICATOR_X, WORKSPACE_INDICATOR_Y,
};
use crate::{OutputId, WindowId};

use super::{Shell, Workspace};

impl Shell {
    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    pub fn active_workspace(&self) -> usize {
        self.active
    }

    pub fn workspace_names(&self) -> Vec<String> {
        self.workspaces.iter().map(|ws| ws.name.clone()).collect()
    }

    /// Picker hit area for a workspace on an output: the bar plus its label.
    pub fn workspace_indicator_rect_for(&self, output: OutputId, index: usize) -> Option<Rect> {
        if index >= self.workspaces.len() {
            return None;
        }
        let bounds = self.output_bounds(output);
        Some(Rect::new(
            bounds.x + WORKSPACE_INDICATOR_X + index as i32 * WORKSPACE_INDICATOR_SPACING,
            bounds.y + WORKSPACE_INDICATOR_Y,
            WORKSPACE_INDICATOR_WIDTH,
            WORKSPACE_INDICATOR_HIT_HEIGHT,
        ))
    }

    /// Picker hit area on the primary output.
    pub fn workspace_indicator_rect(&self, index: usize) -> Option<Rect> {
        let primary = self.primary_output().map(|output| output.id)?;
        self.workspace_indicator_rect_for(primary, index)
    }

    /// Workspace whose indicator on `output` contains `point`.
    pub fn workspace_at_in_output(&self, output: OutputId, point: Point) -> Option<usize> {
        (0..self.workspaces.len()).find(|&index| {
            self.workspace_indicator_rect_for(output, index)
                .is_some_and(|rect| rect.contains(point))
        })
    }

    /// Workspace whose indicator on any output contains `point`.
    pub fn workspace_at_global(&self, point: Point) -> Option<usize> {
        self.outputs
            .iter()
            .find_map(|output| self.workspace_at_in_output(output.id, point))
    }

    /// Workspace whose indicator on the primary output contains `point`
    /// (delegates to the global search).
    pub fn workspace_at(&self, point: Point) -> Option<usize> {
        self.workspace_at_global(point)
    }

    /// Switch an output's workspace, routing primary changes through the
    /// regular active-workspace path so focus/palette state stays coherent.
    pub fn switch_workspace_on(&mut self, output: OutputId, index: usize) -> bool {
        if index >= self.workspaces.len() {
            return false;
        }
        if self.primary_output().map(|candidate| candidate.id) == Some(output) {
            return self.switch_workspace(index);
        }
        self.set_output_workspace(output, index)
    }

    pub fn add_workspace(&mut self, name: impl Into<String>) -> usize {
        self.workspaces.push(Workspace {
            name: name.into(),
            windows: Vec::new(),
        });
        self.workspaces.len() - 1
    }

    pub fn switch_workspace(&mut self, index: usize) -> bool {
        if index >= self.workspaces.len() || index == self.active {
            return false;
        }

        self.active = index;
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.output_workspaces.insert(primary, index);
        self.focus = self.last_window_on_workspace(primary, index);
        self.relayout();
        true
    }

    pub fn move_window_to_workspace(&mut self, id: WindowId, target: usize) -> bool {
        if target >= self.workspaces.len() {
            return false;
        }
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let current = window.workspace;
        if current == target {
            return false;
        }
        let surface = window.surface;

        self.workspaces[current]
            .windows
            .retain(|candidate| *candidate != id);
        self.workspaces[target].windows.push(id);
        if let Some(window) = self.windows.get_mut(&id) {
            window.workspace = target;
        }

        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        if target != self.output_workspace(output) {
            if self.focus == Some(id) {
                self.focus = self.visible_windows_on(output).last().copied();
            }
        } else {
            self.focus = Some(id);
            self.scene.raise(surface);
        }
        self.relayout();
        true
    }
}
