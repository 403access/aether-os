//! Agent sandboxes overlay: registry rows, selection and stop actions.
//!
//! Records are written by `aether-sandbox run`. The panel samples the process
//! table once per second to tell a live sandbox from a stale record left by a
//! killed launcher.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use aether_sandbox::registry::{self, SandboxRecord};
use aether_system::SystemSnapshot;
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SandboxStatus {
    Running,
    Stale,
}

impl SandboxStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stale => "stale",
        }
    }
}

pub(crate) struct SandboxPanelState {
    records: Vec<SandboxRecord>,
    snapshot: SystemSnapshot,
    selected: usize,
    scroll: usize,
    message: Option<String>,
}

impl SandboxPanelState {
    pub(crate) fn new(records: Vec<SandboxRecord>, snapshot: SystemSnapshot) -> Self {
        Self {
            records,
            snapshot,
            selected: 0,
            scroll: 0,
            message: None,
        }
    }

    pub(crate) fn records(&self) -> &[SandboxRecord] {
        &self.records
    }

    pub(crate) fn snapshot(&self) -> &SystemSnapshot {
        &self.snapshot
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn rows_len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn selected_record(&self) -> Option<&SandboxRecord> {
        self.records.get(self.selected)
    }

    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub(crate) fn running_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| self.status(record) == SandboxStatus::Running)
            .count()
    }

    pub(crate) fn status(&self, record: &SandboxRecord) -> SandboxStatus {
        if self
            .snapshot
            .process(record.pid)
            .is_some_and(|process| process.state != "Z")
        {
            SandboxStatus::Running
        } else {
            SandboxStatus::Stale
        }
    }

    pub(crate) fn refresh(&mut self, records: Vec<SandboxRecord>, snapshot: SystemSnapshot) {
        let selected_id = self.selected_record().map(|record| record.id.clone());
        self.records = records;
        self.snapshot = snapshot;
        if let Some(id) = selected_id {
            if let Some(index) = self.records.iter().position(|record| record.id == id) {
                self.selected = index;
            }
        }
        self.clamp(0);
    }

    pub(crate) fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub(crate) fn move_selection(&mut self, delta: isize, visible_rows: usize) {
        if self.records.is_empty() {
            self.selected = 0;
            self.scroll = 0;
            return;
        }
        self.selected =
            (self.selected as isize + delta).clamp(0, self.records.len() as isize - 1) as usize;
        self.clamp(visible_rows);
    }

    pub(crate) fn select(&mut self, index: usize, visible_rows: usize) {
        if self.records.is_empty() {
            return;
        }
        self.selected = index.min(self.records.len() - 1);
        self.clamp(visible_rows);
    }

    pub(crate) fn page(&mut self, direction: isize, visible_rows: usize) {
        self.move_selection(direction * visible_rows.max(1) as isize, visible_rows);
    }

    fn clamp(&mut self, visible_rows: usize) {
        if self.records.is_empty() {
            self.selected = 0;
            self.scroll = 0;
            return;
        }
        self.selected = self.selected.min(self.records.len() - 1);
        if visible_rows == 0 {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible_rows {
            self.scroll = self.selected + 1 - visible_rows;
        }
        self.scroll = self
            .scroll
            .min(self.records.len().saturating_sub(visible_rows));
    }
}

impl App {
    pub(crate) fn open_sandboxes(&mut self) {
        self.shortcuts_open = false;
        self.shell.close_audio();
        self.shell.close_themes();
        self.shell.close_remote();
        self.shell.close_explorer();
        self.shell.close_chat();
        self.close_task_manager();
        let records = registry::list();
        let snapshot = SystemSnapshot::sample();
        println!(
            "aether-shell: agent sandboxes opened ({} records, {} running)",
            records.len(),
            records
                .iter()
                .filter(|record| snapshot.process(record.pid).is_some())
                .count()
        );
        self.sandboxes = Some(SandboxPanelState::new(records, snapshot));
        self.sandbox_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub(crate) fn toggle_sandboxes(&mut self) {
        self.close_system();
        if self.sandbox_panel_open() {
            self.close_sandboxes();
        } else {
            self.open_sandboxes();
        }
    }

    pub(crate) fn sandbox_panel_open(&self) -> bool {
        self.sandboxes.is_some()
    }

    pub(crate) fn close_sandboxes(&mut self) {
        if self.sandboxes.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn poll_sandboxes(&mut self) {
        if self.sandboxes.is_none()
            || self.sandbox_last.elapsed() < std::time::Duration::from_secs(1)
        {
            return;
        }
        let records = registry::list();
        let snapshot = SystemSnapshot::sample();
        if let Some(state) = self.sandboxes.as_mut() {
            state.refresh(records, snapshot);
        }
        self.sandbox_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub(crate) fn sandbox_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        AetherRect::new(
            bounds.x,
            bounds.y + 38,
            bounds.width,
            bounds.height.saturating_sub(38 + 74),
        )
    }

    pub(crate) fn sandbox_pane_rect(&self) -> AetherRect {
        let bounds = self.sandbox_rect();
        AetherRect::new(
            bounds.x + 16,
            bounds.y + 74,
            bounds.width.saturating_sub(32),
            bounds.height.saturating_sub(74 + 52),
        )
    }

    pub(crate) fn sandbox_list_rect(&self) -> AetherRect {
        let pane = self.sandbox_pane_rect();
        let width = ((pane.width as i32 * 52) / 100).max(240) as u32;
        AetherRect::new(pane.x, pane.y, width, pane.height)
    }

    pub(crate) fn sandbox_detail_rect(&self) -> AetherRect {
        let pane = self.sandbox_pane_rect();
        let list = self.sandbox_list_rect();
        AetherRect::new(
            list.right() as i32 + 12,
            pane.y,
            pane.width.saturating_sub(list.width + 12),
            pane.height,
        )
    }

    pub(crate) fn sandbox_rows_top(&self) -> i32 {
        self.sandbox_list_rect().y + 38 + 30
    }

    pub(crate) fn sandbox_visible_rows(&self) -> usize {
        let list = self.sandbox_list_rect();
        (list.height.saturating_sub(38 + 30 + 10) / 54).max(1) as usize
    }

    pub(crate) fn sandbox_row_rect(&self, index: usize) -> AetherRect {
        let list = self.sandbox_list_rect();
        AetherRect::new(
            list.x + 8,
            self.sandbox_rows_top() + index as i32 * 54,
            list.width.saturating_sub(16),
            50,
        )
    }

    pub(crate) fn sandbox_close_rect(&self) -> AetherRect {
        let rect = self.sandbox_rect();
        AetherRect::new(rect.right() as i32 - 42, rect.y + 20, 24, 24)
    }

    pub(crate) fn sandbox_row_at(&self, point: AetherPoint) -> Option<usize> {
        let state = self.sandboxes.as_ref()?;
        let start = state.scroll();
        (0..state.rows_len().min(self.sandbox_visible_rows()))
            .find(|offset| self.sandbox_row_rect(*offset).contains(point))
            .map(|offset| start + offset)
    }

    pub(crate) fn handle_sandbox_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let refresh: Keysym = keysyms::KEY_r.into();
        let stop: Keysym = keysyms::KEY_k.into();
        let log: Keysym = keysyms::KEY_l.into();
        let clean: Keysym = keysyms::KEY_c.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        if keysym == escape {
            self.close_sandboxes();
            return;
        }
        let visible = self.sandbox_visible_rows();
        if keysym == up {
            if let Some(state) = self.sandboxes.as_mut() {
                state.move_selection(-1, visible);
            }
            self.mark_sandbox_damage();
        } else if keysym == down {
            if let Some(state) = self.sandboxes.as_mut() {
                state.move_selection(1, visible);
            }
            self.mark_sandbox_damage();
        } else if keysym == page_up {
            if let Some(state) = self.sandboxes.as_mut() {
                state.page(-1, visible);
            }
            self.mark_sandbox_damage();
        } else if keysym == page_down {
            if let Some(state) = self.sandboxes.as_mut() {
                state.page(1, visible);
            }
            self.mark_sandbox_damage();
        } else if keysym == refresh {
            self.sandbox_last = std::time::Instant::now() - std::time::Duration::from_secs(2);
            self.poll_sandboxes();
        } else if keysym == stop {
            let force = self.modifiers.shift;
            self.stop_selected_sandbox(force);
        } else if keysym == log || keysym == enter {
            self.open_selected_sandbox_log();
        } else if keysym == clean {
            self.clean_stale_sandboxes();
        }
    }

    fn stop_selected_sandbox(&mut self, force: bool) {
        let Some(record) = self
            .sandboxes
            .as_ref()
            .and_then(|state| state.selected_record())
            .cloned()
        else {
            return;
        };
        let result = registry::request_stop(&record, force);
        let message = match result {
            Ok(()) => format!(
                "{} sent to {} ({})",
                if force { "SIGKILL" } else { "SIGTERM" },
                record.label,
                record.pid
            ),
            Err(error) => format!("stop failed for {}: {error}", record.pid),
        };
        println!("aether-shell: {message}");
        if let Some(state) = self.sandboxes.as_mut() {
            state.set_message(message);
        }
        self.sandbox_last = std::time::Instant::now() - std::time::Duration::from_secs(2);
        self.poll_sandboxes();
        self.mark_sandbox_damage();
    }

    fn open_selected_sandbox_log(&mut self) {
        let Some(record) = self
            .sandboxes
            .as_ref()
            .and_then(|state| state.selected_record())
            .cloned()
        else {
            return;
        };
        let Some(path) = record.log_path else {
            if let Some(state) = self.sandboxes.as_mut() {
                state.set_message("no log file; rerun with --log PATH".to_string());
            }
            self.mark_sandbox_damage();
            return;
        };
        if std::path::Path::new(&path).exists() {
            self.open_path(&path);
        } else if let Some(state) = self.sandboxes.as_mut() {
            state.set_message(format!("log not found: {path}"));
        }
        self.mark_sandbox_damage();
    }

    fn clean_stale_sandboxes(&mut self) {
        let records = self
            .sandboxes
            .as_ref()
            .map(|state| state.records().to_vec())
            .unwrap_or_default();
        let snapshot = SystemSnapshot::sample();
        let mut removed = 0usize;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        for record in records {
            if now.saturating_sub(record.started_unix) > 5
                && snapshot.process(record.pid).is_none()
                && registry::remove(&record.id).is_ok()
            {
                removed += 1;
            }
        }
        let records = registry::list();
        if let Some(state) = self.sandboxes.as_mut() {
            state.refresh(records, snapshot);
            state.set_message(format!("pruned {removed} stale record(s)"));
        }
        self.sandbox_last = std::time::Instant::now();
        self.mark_sandbox_damage();
    }

    pub(crate) fn mark_sandbox_damage(&mut self) {
        if self.sandboxes.is_some() {
            let rect = self.sandbox_rect();
            self.mark_damage_rect(rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_sandbox::registry::SandboxRecord;

    fn record(id: &str, pid: u32) -> SandboxRecord {
        SandboxRecord {
            id: id.into(),
            label: id.into(),
            pid,
            trust: "untrusted".into(),
            tier: "process-landlock".into(),
            enforcement: "landlock-seccomp".into(),
            uid: Some(4001),
            memory_mib: 512,
            cpu_weight: 25,
            timeout_secs: Some(60),
            allow_net: false,
            capabilities: Vec::new(),
            read_paths: vec!["/work".into()],
            write_paths: vec!["/work/scratch".into()],
            program: "/bin/sh".into(),
            args: Vec::new(),
            started_unix: 1,
            log_path: None,
            control_socket: None,
            warnings: Vec::new(),
        }
    }

    fn state() -> SandboxPanelState {
        SandboxPanelState::new(
            vec![record("one", 10), record("two", 20)],
            SystemSnapshot::default(),
        )
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut state = state();
        state.move_selection(1, 5);
        assert_eq!(state.selected(), 1);
        state.move_selection(10, 5);
        assert_eq!(state.selected(), 1);
        state.page(-1, 5);
        assert_eq!(state.selected(), 0);
    }

    #[test]
    fn refresh_keeps_the_selected_id() {
        let mut state = state();
        state.move_selection(1, 5);
        state.refresh(
            vec![record("zero", 5), record("one", 10), record("two", 20)],
            SystemSnapshot::default(),
        );
        assert_eq!(state.selected_record().unwrap().id, "two");
    }

    #[test]
    fn missing_pid_is_stale() {
        let state = state();
        assert_eq!(state.status(&record("one", 10)), SandboxStatus::Stale);
        assert_eq!(state.running_count(), 0);
    }

    #[test]
    fn live_pid_is_running() {
        let mut snapshot = SystemSnapshot::default();
        let process = aether_system::Process {
            pid: 10,
            state: "S".into(),
            ..Default::default()
        };
        snapshot.processes.push(process);
        let state = SandboxPanelState::new(vec![record("one", 10)], snapshot);
        assert_eq!(state.status(&record("one", 10)), SandboxStatus::Running);
        assert_eq!(state.running_count(), 1);
    }
}
