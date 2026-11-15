//! Output topology, viewport and per-output workspace mapping.

use aether_geometry::{Point, Rect, Size};
use aether_layout::LayoutConfig;

use crate::output::OutputInfo;
use crate::{OutputId, WindowId};

use super::Shell;

impl Shell {
    pub fn viewport(&self) -> Size {
        self.viewport
    }

    pub fn layout(&self) -> LayoutConfig {
        self.layout
    }

    pub fn set_viewport(&mut self, viewport: Size) {
        if viewport == self.viewport {
            return;
        }
        self.viewport = viewport;
        if let Some(primary) = self.outputs.iter_mut().find(|output| output.primary) {
            primary.rect = Rect::from_size(viewport);
        } else if let Some(first) = self.outputs.first_mut() {
            first.rect = Rect::from_size(viewport);
            first.primary = true;
        }
        self.scene.resize_viewport(viewport);
        self.relayout();
    }

    /// Replace the output list. The viewport becomes the bounding box of all
    /// outputs, and a primary output is guaranteed.
    pub fn set_outputs(&mut self, mut outputs: Vec<OutputInfo>) -> bool {
        if outputs.is_empty() {
            return false;
        }
        if !outputs.iter().any(|output| output.primary) {
            outputs[0].primary = true;
        }
        let width = outputs
            .iter()
            .map(|output| output.rect.right().max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let height = outputs
            .iter()
            .map(|output| output.rect.bottom().max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let primary = outputs
            .iter()
            .find(|output| output.primary)
            .map(|output| output.id)
            .unwrap_or(outputs[0].id);
        self.output_workspaces
            .retain(|id, _| outputs.iter().any(|output| output.id == *id));
        for output in &outputs {
            self.output_workspaces.entry(output.id).or_insert(0);
        }
        if let Some(workspace) = self.output_workspaces.get_mut(&primary) {
            *workspace = self.active;
        }
        let valid: Vec<OutputId> = outputs.iter().map(|output| output.id).collect();
        for window in self.windows.values_mut() {
            if !valid.contains(&window.output) {
                window.output = primary;
            }
        }
        if self
            .overlay_output
            .is_some_and(|output| !valid.contains(&output))
        {
            self.overlay_output = Some(primary);
            if let Some(rect) = self.overlay_rect() {
                self.outputs = outputs.clone();
                self.set_overlay(Some(rect));
            }
        }
        self.outputs = outputs;
        self.viewport = Size::new(width, height);
        self.scene.resize_viewport(self.viewport);
        self.relayout();
        true
    }

    pub fn outputs(&self) -> &[OutputInfo] {
        &self.outputs
    }

    pub fn primary_output(&self) -> Option<&OutputInfo> {
        self.outputs
            .iter()
            .find(|output| output.primary)
            .or_else(|| self.outputs.first())
    }

    pub fn output_at(&self, point: Point) -> Option<OutputId> {
        self.outputs
            .iter()
            .find(|output| output.rect.contains(point))
            .map(|output| output.id)
    }

    /// Bounds of one output in logical coordinates.
    pub fn output_bounds(&self, output: OutputId) -> Rect {
        self.outputs
            .iter()
            .find(|candidate| candidate.id == output)
            .map(|candidate| candidate.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport))
    }

    /// Active workspace shown on an output.
    pub fn output_workspace(&self, output: OutputId) -> usize {
        self.output_workspaces
            .get(&output)
            .copied()
            .unwrap_or(0)
            .min(self.workspaces.len().saturating_sub(1))
    }

    /// Assign a workspace to an output. This is per-output: the primary
    /// output's assignment also drives `active_workspace`.
    pub fn set_output_workspace(&mut self, output: OutputId, workspace: usize) -> bool {
        if workspace >= self.workspaces.len()
            || !self.outputs.iter().any(|candidate| candidate.id == output)
        {
            return false;
        }
        if self.output_workspace(output) == workspace {
            return false;
        }
        self.output_workspaces.insert(output, workspace);
        if self
            .primary_output()
            .is_some_and(|primary| primary.id == output)
        {
            self.active = workspace;
        }
        self.focus = self.last_window_on_workspace(output, workspace);
        self.relayout();
        true
    }

    /// Move a window to another output, keeping its workspace assignment.
    pub fn set_window_output(&mut self, id: WindowId, output: OutputId) -> bool {
        if !self.outputs.iter().any(|candidate| candidate.id == output) {
            return false;
        }
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        if window.output == output {
            return false;
        }
        if let Some(window) = self.windows.get_mut(&id) {
            window.output = output;
        }
        self.relayout();
        true
    }

    /// Last window on an output's workspace, used to pick focus before the
    /// scene has been relaid out.
    pub(super) fn last_window_on_workspace(
        &self,
        output: OutputId,
        workspace: usize,
    ) -> Option<WindowId> {
        self.workspaces
            .get(workspace)?
            .windows
            .iter()
            .rev()
            .find(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| window.output == output)
            })
            .copied()
    }

    /// Visible window ids on an output, topmost first.
    pub fn visible_windows_on(&self, output: OutputId) -> Vec<WindowId> {
        let workspace = self.output_workspace(output);
        self.window_stack()
            .into_iter()
            .filter(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| window.output == output && window.workspace == workspace)
            })
            .collect()
    }
}
