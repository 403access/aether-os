//! The virtualised explorer surface.

use aether_geometry::{Point, Rect};

use crate::design;
use crate::model::explorer::ExplorerState;
use crate::model::palette::{palette_score, PaletteFile};

use super::Shell;

impl Shell {
    /// Explorer window rect. Matches `.os-window--explorer`: 16 px from the
    /// top-left of the work area, full width below 1360 px, 610 px tall at
    /// 1280 x 800 so the dock stays visible.
    pub fn explorer_rect(&self) -> Rect {
        let bounds = self
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport));
        let width = if bounds.width <= 1360 {
            bounds.width.saturating_sub(32)
        } else {
            (bounds.width.saturating_sub(336)).min(1080)
        };
        let height = (bounds.height.saturating_sub(54 + 16)).min(730);
        Rect::new(bounds.x + 16, bounds.y + 54, width, height)
    }

    /// Explorer sidebar width (`.os-file__sidebar`).
    pub const SIDEBAR_WIDTH: i32 = 220;

    /// Scrollable rows area: below the title bar, toolbar, search row and
    /// column header; above the footer and status strip.
    pub fn explorer_pane_rect(&self) -> Rect {
        let bounds = self.explorer_rect();
        let main_x = bounds.x + Self::SIDEBAR_WIDTH;
        Rect::new(
            main_x + 8,
            bounds.y + 40 + 40 + 40 + 30,
            bounds.width.saturating_sub(Self::SIDEBAR_WIDTH as u32 + 16),
            bounds.height.saturating_sub(40 + 40 + 40 + 30 + 30 + 32),
        )
    }

    pub fn explorer_rows_top(&self) -> i32 {
        self.explorer_pane_rect().y
    }

    pub fn open_explorer(&mut self) -> Option<()> {
        self.close_remote();
        self.close_themes();
        if self.explorer.is_none() {
            self.explorer = Some(ExplorerState::default());
        }
        Some(())
    }

    pub fn close_explorer(&mut self) {
        self.explorer = None;
    }

    pub fn explorer_open(&self) -> bool {
        self.explorer.is_some()
    }

    pub fn explorer_query(&self) -> &str {
        self.explorer
            .as_ref()
            .map(|explorer| explorer.query.as_str())
            .unwrap_or("")
    }

    pub fn explorer_selected(&self) -> usize {
        self.explorer
            .as_ref()
            .map(|explorer| explorer.selected)
            .unwrap_or(0)
    }

    pub fn explorer_scroll(&self) -> usize {
        self.explorer
            .as_ref()
            .map(|explorer| explorer.scroll)
            .unwrap_or(0)
    }

    pub fn explorer_visible_rows(&self) -> usize {
        if self.explorer.is_none() {
            return 0;
        }
        let pane = self.explorer_pane_rect();
        (pane.height / design::EXPLORER_ROW_HEIGHT).max(1) as usize
    }

    pub fn explorer_match_count(&self) -> usize {
        let Some(explorer) = self.explorer.as_ref() else {
            return 0;
        };
        let _ = explorer;
        let mut explorer = self.explorer.clone();
        if let Some(state) = explorer.as_mut() {
            self.refresh_explorer(state);
            state.matches.len()
        } else {
            0
        }
    }

    fn refresh_explorer(&self, explorer: &mut ExplorerState) {
        // The row window follows the current pane height, not the default the
        // state was created with: the explorer window is resizable.
        explorer.visible = self.explorer_visible_rows();
        if explorer.dirty {
            let query = explorer.query.to_lowercase();
            explorer.matches = self
                .files
                .iter()
                .enumerate()
                .filter_map(|(index, file)| {
                    palette_score(&query, &file.name).map(|score| (score, index))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|(_, index)| index)
                .collect();
            explorer.dirty = false;
            if explorer.selected >= explorer.matches.len() {
                explorer.selected = explorer.matches.len().saturating_sub(1);
            }
        }
        let max_scroll = explorer.matches.len().saturating_sub(explorer.visible);
        explorer.scroll = explorer.scroll.min(max_scroll);
    }

    /// Only the visible window of file references, never the full list.
    pub fn visible_explorer_rows(&self) -> Vec<&PaletteFile> {
        let Some(explorer) = self.explorer.as_ref() else {
            return Vec::new();
        };
        let mut state = explorer.clone();
        self.refresh_explorer(&mut state);
        let visible = self.explorer_visible_rows();
        state
            .matches
            .iter()
            .skip(state.scroll)
            .take(visible)
            .filter_map(|index| self.files.get(*index))
            .collect()
    }

    /// Selected file index in `files`, resolving cached matches.
    pub fn explorer_selected_file(&self) -> Option<PaletteFile> {
        let explorer = self.explorer.as_ref()?;
        let mut state = explorer.clone();
        self.refresh_explorer(&mut state);
        let index = *state.matches.get(state.selected)?;
        self.files.get(index).cloned()
    }

    /// Select the visible row under `point`; returns the absolute index.
    pub fn explorer_select_at(&mut self, point: Point) -> Option<usize> {
        let pane = self.explorer_pane_rect();
        let content_top = self.explorer_rows_top();
        if point.x < pane.x + 8 || point.x >= pane.right() as i32 - 8 {
            return None;
        }
        let row_height = design::EXPLORER_ROW_HEIGHT as i32;
        if point.y < content_top
            || point.y >= content_top + self.explorer_visible_rows() as i32 * row_height
        {
            return None;
        }
        let row = ((point.y - content_top) / row_height) as usize;
        let mut state = self.explorer.clone()?;
        self.refresh_explorer(&mut state);
        let index = state.scroll + row;
        if index >= state.matches.len() {
            return None;
        }
        state.selected = index;
        self.explorer = Some(state);
        Some(index)
    }

    pub fn explorer_push_char(&mut self, character: char) -> bool {
        let Some(explorer) = self.explorer.as_mut() else {
            return false;
        };
        if character.is_control() || explorer.query.chars().count() >= 64 {
            return false;
        }
        explorer.query.push(character);
        explorer.selected = 0;
        explorer.scroll = 0;
        explorer.dirty = true;
        true
    }

    pub fn explorer_backspace(&mut self) -> bool {
        let Some(explorer) = self.explorer.as_mut() else {
            return false;
        };
        let changed = explorer.query.pop().is_some();
        if changed {
            explorer.selected = 0;
            explorer.scroll = 0;
            explorer.dirty = true;
        }
        changed
    }

    fn clamp_explorer(&self, explorer: &mut ExplorerState) {
        let visible = self.explorer_visible_rows();
        let max_scroll = explorer.matches.len().saturating_sub(visible);
        explorer.scroll = explorer.scroll.min(max_scroll);
        if explorer.selected < explorer.scroll {
            explorer.scroll = explorer.selected;
        } else if explorer.selected >= explorer.scroll + explorer.visible {
            explorer.scroll = explorer.selected + 1 - explorer.visible;
        }
    }

    pub fn explorer_move(&mut self, delta: isize) -> bool {
        let Some(mut state) = self.explorer.clone() else {
            return false;
        };
        self.refresh_explorer(&mut state);
        if state.matches.is_empty() {
            return false;
        }
        let len = state.matches.len() as isize;
        state.selected = (state.selected as isize + delta).rem_euclid(len) as usize;
        self.clamp_explorer(&mut state);
        self.explorer = Some(state);
        true
    }

    pub fn explorer_page(&mut self, direction: isize) -> bool {
        if self.explorer.is_none() {
            return false;
        }
        let page = self.explorer_visible_rows() as isize;
        self.explorer_move(direction * page)
    }

    pub fn explorer_scroll_by(&mut self, delta: isize) -> bool {
        let Some(mut state) = self.explorer.clone() else {
            return false;
        };
        self.refresh_explorer(&mut state);
        if state.matches.is_empty() {
            return false;
        }
        let visible = self.explorer_visible_rows();
        let max_scroll = state.matches.len().saturating_sub(visible) as isize;
        state.scroll = (state.scroll as isize + delta).clamp(0, max_scroll) as usize;
        state.selected = state.selected.max(state.scroll).min(
            (state.scroll + visible)
                .min(state.matches.len())
                .saturating_sub(1),
        );
        self.explorer = Some(state);
        true
    }
}
