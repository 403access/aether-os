//! Task Manager state: process-tree selection, tabs and scroll.

use std::collections::HashSet;

use aether_system::{Process, SystemSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskTab {
    Processes,
    Users,
    Resources,
}

impl TaskTab {
    pub const ALL: [Self; 3] = [Self::Processes, Self::Users, Self::Resources];

    pub fn label(self) -> &'static str {
        match self {
            Self::Processes => "Processes",
            Self::Users => "Users",
            Self::Resources => "Resources",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Processes => Self::Users,
            Self::Users => Self::Resources,
            Self::Resources => Self::Processes,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Processes => Self::Resources,
            Self::Users => Self::Processes,
            Self::Resources => Self::Users,
        }
    }
}

pub struct TaskManagerState {
    snapshot: SystemSnapshot,
    tab: TaskTab,
    selected: usize,
    scroll: usize,
    collapsed: HashSet<u32>,
}

impl TaskManagerState {
    pub fn new(snapshot: SystemSnapshot) -> Self {
        Self {
            snapshot,
            tab: TaskTab::Processes,
            selected: 0,
            scroll: 0,
            collapsed: HashSet::new(),
        }
    }

    pub fn snapshot(&self) -> &SystemSnapshot {
        &self.snapshot
    }

    pub fn tab(&self) -> TaskTab {
        self.tab
    }

    pub fn set_tab(&mut self, tab: TaskTab) {
        self.tab = tab;
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn rows_len(&self) -> usize {
        self.tree().len()
    }

    pub fn tree(&self) -> Vec<(usize, u32)> {
        self.snapshot.tree(&self.collapsed)
    }

    pub fn process_rows(&self) -> Vec<(usize, &Process)> {
        self.tree()
            .into_iter()
            .filter_map(|(depth, pid)| self.snapshot.process(pid).map(|process| (depth, process)))
            .collect()
    }

    pub fn selected_process(&self) -> Option<&Process> {
        self.process_rows()
            .get(self.selected)
            .map(|(_, process)| *process)
    }

    pub fn refresh(&mut self, snapshot: SystemSnapshot) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        self.snapshot = snapshot;
        if let Some(pid) = selected_pid {
            if let Some(index) = self
                .tree()
                .iter()
                .position(|(_, candidate)| *candidate == pid)
            {
                self.selected = index;
            }
        }
        self.clamp(0);
    }

    pub fn move_selection(&mut self, delta: isize, visible_rows: usize) {
        let len = self.rows_len();
        if len == 0 {
            self.selected = 0;
            self.scroll = 0;
            return;
        }
        self.selected = (self.selected as isize + delta).clamp(0, len as isize - 1) as usize;
        self.clamp(visible_rows);
    }

    pub fn select(&mut self, index: usize, visible_rows: usize) {
        if self.rows_len() == 0 {
            return;
        }
        self.selected = index.min(self.rows_len() - 1);
        self.clamp(visible_rows);
    }

    pub fn page(&mut self, direction: isize, visible_rows: usize) {
        self.move_selection(direction * visible_rows.max(1) as isize, visible_rows);
    }

    pub fn toggle_collapse(&mut self, visible_rows: usize) {
        let Some(process) = self.selected_process() else {
            return;
        };
        let pid = process.pid;
        if process.children.is_empty() {
            return;
        }
        if !self.collapsed.remove(&pid) {
            self.collapsed.insert(pid);
        }
        self.clamp(visible_rows);
    }

    pub fn collapse(&mut self, visible_rows: usize) {
        if let Some(process) = self.selected_process() {
            let pid = process.pid;
            if !process.children.is_empty() {
                self.collapsed.insert(pid);
            }
        }
        self.clamp(visible_rows);
    }

    pub fn expand(&mut self, visible_rows: usize) {
        if let Some(process) = self.selected_process() {
            let pid = process.pid;
            self.collapsed.remove(&pid);
        }
        self.clamp(visible_rows);
    }

    pub fn collapsed(&self) -> &HashSet<u32> {
        &self.collapsed
    }

    fn clamp(&mut self, visible_rows: usize) {
        let len = self.rows_len();
        if len == 0 {
            self.selected = 0;
            self.scroll = 0;
            return;
        }
        self.selected = self.selected.min(len - 1);
        if visible_rows == 0 {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible_rows {
            self.scroll = self.selected + 1 - visible_rows;
        }
        let max_scroll = len.saturating_sub(visible_rows);
        self.scroll = self.scroll.min(max_scroll);
    }
}
