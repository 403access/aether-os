//! Task Manager overlay: selection, tabs, scrolling and refresh cadence.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use smithay::input::keyboard::{keysyms, Keysym};

mod state;

pub(crate) use state::{TaskManagerState, TaskTab};

use super::super::App;

impl App {
    pub(crate) fn open_task_manager(&mut self) {
        self.close_system();
        self.shortcuts_open = false;
        self.shell.close_audio();
        self.shell.close_themes();
        self.shell.close_remote();
        self.shell.close_explorer();
        self.shell.close_chat();
        let snapshot = self.task_monitor.sample();
        println!(
            "aether-shell: task manager opened ({} processes, {} users)",
            snapshot.process_count,
            snapshot.users.len()
        );
        self.task_manager = Some(TaskManagerState::new(snapshot));
        self.task_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub fn task_manager_open(&self) -> bool {
        self.task_manager.is_some()
    }

    pub(crate) fn close_task_manager(&mut self) {
        if self.task_manager.take().is_some() {
            self.mark_full_damage();
        }
    }

    /// Task Manager as a first-party window: the demo's `.os-window` box.
    pub(crate) fn task_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = 1248.min(bounds.width.saturating_sub(32));
        let height = 730.min(bounds.height.saturating_sub(54 + 16));
        AetherRect::new(bounds.x + 16, bounds.y + 54, width, height)
    }

    pub(crate) fn task_pane_rect(&self) -> AetherRect {
        let bounds = self.task_rect();
        AetherRect::new(
            bounds.x + 16,
            bounds.y + 40 + 52,
            bounds.width.saturating_sub(32),
            bounds.height.saturating_sub(40 + 52 + 32 + 12),
        )
    }

    pub(crate) fn task_rows_top(&self) -> i32 {
        self.task_pane_rect().y + 56 + 30
    }

    pub(crate) fn task_visible_rows(&self) -> usize {
        let pane = self.task_pane_rect();
        (pane.height.saturating_sub(56 + 30 + 30) / 28).max(1) as usize
    }

    pub(crate) fn task_row_rect(&self, index: usize) -> AetherRect {
        let pane = self.task_pane_rect();
        AetherRect::new(
            pane.x + 8,
            self.task_rows_top() + index as i32 * 28,
            pane.width.saturating_sub(16),
            26,
        )
    }

    pub(crate) fn task_close_rect(&self) -> AetherRect {
        let rect = self.task_rect();
        AetherRect::new(rect.x + 11, rect.y + 14, 11, 11)
    }

    pub(crate) fn task_row_at(&self, point: AetherPoint) -> Option<usize> {
        let state = self.task_manager.as_ref()?;
        let rows = match state.tab() {
            TaskTab::Processes => state.rows_len(),
            TaskTab::Users => state.snapshot().users.len(),
            TaskTab::Resources => 0,
        };
        let start = state.scroll();
        (0..rows.min(self.task_visible_rows()))
            .find(|offset| {
                self.task_row_rect(*offset)
                    .contains(AetherPoint::new(point.x, point.y))
            })
            .map(|offset| start + offset)
    }

    pub fn poll_task_manager(&mut self) {
        if self.task_manager.is_none()
            || self.task_last.elapsed() < std::time::Duration::from_secs(1)
        {
            return;
        }
        let snapshot = self.task_monitor.sample();
        if let Some(state) = self.task_manager.as_mut() {
            state.refresh(snapshot);
        }
        self.task_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub fn task_refresh_in(&self) -> Option<std::time::Duration> {
        self.task_manager.as_ref()?;
        Some(
            std::time::Duration::from_secs(1)
                .saturating_sub(self.task_last.elapsed())
                .max(std::time::Duration::from_millis(50)),
        )
    }

    pub(crate) fn handle_task_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let tab: Keysym = keysyms::KEY_Tab.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let left: Keysym = keysyms::KEY_Left.into();
        let right: Keysym = keysyms::KEY_Right.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let refresh: Keysym = keysyms::KEY_r.into();
        let space: Keysym = keysyms::KEY_space.into();
        if keysym == escape {
            self.close_task_manager();
            self.close_sandboxes();
            return;
        }
        let visible = self.task_visible_rows();
        if keysym == tab {
            let shift = self.modifiers.shift;
            if let Some(state) = self.task_manager.as_mut() {
                state.set_tab(if shift {
                    state.tab().previous()
                } else {
                    state.tab().next()
                });
            }
            self.mark_task_damage();
        } else if keysym == up {
            if let Some(state) = self.task_manager.as_mut() {
                state.move_selection(-1, visible);
            }
            self.mark_task_damage();
        } else if keysym == down {
            if let Some(state) = self.task_manager.as_mut() {
                state.move_selection(1, visible);
            }
            self.mark_task_damage();
        } else if keysym == page_up {
            if let Some(state) = self.task_manager.as_mut() {
                state.page(-1, visible);
            }
            self.mark_task_damage();
        } else if keysym == page_down {
            if let Some(state) = self.task_manager.as_mut() {
                state.page(1, visible);
            }
            self.mark_task_damage();
        } else if keysym == space {
            if let Some(state) = self.task_manager.as_mut() {
                state.toggle_collapse(visible);
            }
            self.mark_task_damage();
        } else if keysym == left {
            if let Some(state) = self.task_manager.as_mut() {
                state.collapse(visible);
            }
            self.mark_task_damage();
        } else if keysym == right {
            if let Some(state) = self.task_manager.as_mut() {
                state.expand(visible);
            }
            self.mark_task_damage();
        } else if keysym == refresh {
            self.task_last = std::time::Instant::now() - std::time::Duration::from_secs(2);
            self.poll_task_manager();
            self.mark_task_damage();
        }
    }

    pub(crate) fn mark_task_damage(&mut self) {
        if self.task_manager.is_some() {
            let rect = self.task_rect();
            self.mark_damage_rect(rect);
        }
    }
}
