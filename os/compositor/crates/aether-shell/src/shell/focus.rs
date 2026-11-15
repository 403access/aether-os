//! Focus transitions and the MRU stack.

use crate::WindowId;

use super::Shell;

impl Shell {
    pub fn focus_window(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let output_workspace = self.output_workspace(window.output);
        if window.workspace != output_workspace {
            return false;
        }
        self.focus = Some(id);
        self.focus_cycle = None;
        self.focus_history.retain(|candidate| *candidate != id);
        self.focus_history.insert(0, id);
        self.focus_history.truncate(32);
        let surface = window.surface;
        self.scene.raise(surface);
        true
    }

    /// MRU cycle used by Super+Tab. The history is not reordered during a
    /// cycle; any direct focus change resets it.
    pub fn focus_mru(&mut self, delta: isize) -> Option<WindowId> {
        self.focus_history
            .retain(|id| self.windows.contains_key(id));
        let visible: Vec<WindowId> = self
            .focus_history
            .iter()
            .copied()
            .filter(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| window.workspace == self.output_workspace(window.output))
            })
            .collect();
        if visible.is_empty() {
            return None;
        }
        let len = visible.len() as isize;
        let index = match self.focus_cycle {
            Some(index) => (index as isize + delta).rem_euclid(len),
            None if delta > 0 => 1 % len,
            None => (len - 1).rem_euclid(len),
        } as usize;
        self.focus_cycle = Some(index);
        self.focus = Some(visible[index]);
        self.scene.raise(self.windows.get(&visible[index])?.surface);
        Some(visible[index])
    }

    pub fn focus_next(&mut self) -> Option<WindowId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        let ids = self.visible_windows_on(primary);
        if ids.is_empty() {
            self.focus = None;
            return None;
        }
        let next = match self.focus.and_then(|f| ids.iter().position(|id| *id == f)) {
            Some(index) => (index + 1) % ids.len(),
            None => 0,
        };
        self.focus_window(ids[next]);
        Some(ids[next])
    }

    pub fn focus_previous(&mut self) -> Option<WindowId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        let ids = self.visible_windows_on(primary);
        if ids.is_empty() {
            self.focus = None;
            return None;
        }
        let previous = match self.focus.and_then(|f| ids.iter().position(|id| *id == f)) {
            Some(0) => ids.len() - 1,
            Some(index) => index - 1,
            None => ids.len() - 1,
        };
        self.focus_window(ids[previous]);
        Some(ids[previous])
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus
    }
}
