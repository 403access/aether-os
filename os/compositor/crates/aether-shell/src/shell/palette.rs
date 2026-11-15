//! Command palette providers, selection and filtering.

use aether_geometry::{Point, Rect};

use crate::model::palette::{
    palette_score, parse_palette_query, PaletteAction, PaletteEntry, PaletteFile, PaletteLauncher,
    PaletteMode, PALETTE_CONTENT_TOP, PALETTE_ROW_HEIGHT, PALETTE_ROW_SPACING,
    PALETTE_VISIBLE_ROWS,
};

use super::Shell;

impl Shell {
    /// Replace the application launchers shown in the palette.
    pub fn set_launchers(&mut self, launchers: Vec<PaletteLauncher>) {
        self.launchers = launchers;
        self.palette.selected = 0;
    }

    pub fn launchers(&self) -> &[PaletteLauncher] {
        &self.launchers
    }

    /// Clone a launcher by its palette index.
    pub fn launcher(&self, index: usize) -> Option<PaletteLauncher> {
        self.launchers.get(index).cloned()
    }

    /// Replace the filename index shown for non-empty queries.
    pub fn set_files(&mut self, files: Vec<PaletteFile>) {
        self.files = files;
        self.files_prefiltered = false;
        self.palette.selected = 0;
        if let Some(explorer) = self.explorer.as_mut() {
            explorer.dirty = true;
        }
    }

    /// Replace the file list with results already matched by an external
    /// indexer. They are shown as-is for non-empty queries instead of being
    /// re-filtered by filename.
    pub fn set_indexed_files(&mut self, files: Vec<PaletteFile>) {
        self.files = files;
        self.files_prefiltered = true;
        self.palette.selected = 0;
        if let Some(explorer) = self.explorer.as_mut() {
            explorer.dirty = true;
        }
    }

    pub fn files(&self) -> &[PaletteFile] {
        &self.files
    }

    /// Clone a file by its palette index.
    pub fn file(&self, index: usize) -> Option<PaletteFile> {
        self.files.get(index).cloned()
    }

    pub fn palette_query(&self) -> &str {
        &self.palette.query
    }

    /// Filtered palette entries. A leading prefix narrows providers:
    /// `>` commands, `@` windows/agents, `/` files, `~` workspaces.
    pub fn palette_entries(&self) -> Vec<PaletteEntry> {
        let (mode, query) = parse_palette_query(&self.palette.query);
        let query = query.to_lowercase();
        let mut scored: Vec<(u8, usize, PaletteEntry)> = Vec::new();

        if matches!(
            mode,
            PaletteMode::All | PaletteMode::Command | PaletteMode::Setting
        ) {
            for (index, workspace) in self.workspaces.iter().enumerate() {
                let detail = format!("workspace {}", index + 1);
                if let Some(score) = palette_score(&query, &workspace.name) {
                    scored.push((
                        score,
                        index,
                        PaletteEntry {
                            title: workspace.name.clone(),
                            detail,
                            action: PaletteAction::SwitchWorkspace(index),
                        },
                    ));
                }
            }
        }

        let launcher_offset = self.workspaces.len();
        if matches!(mode, PaletteMode::All | PaletteMode::Command) {
            for (index, launcher) in self.launchers.iter().enumerate() {
                if let Some(score) = palette_score(&query, &launcher.name) {
                    scored.push((
                        score,
                        launcher_offset + index,
                        PaletteEntry {
                            title: launcher.name.clone(),
                            detail: launcher.detail.clone(),
                            action: PaletteAction::LaunchApp(index),
                        },
                    ));
                }
            }
        }

        let command_offset = launcher_offset + self.launchers.len();
        if matches!(mode, PaletteMode::All | PaletteMode::Command) {
            let new_terminal = PaletteEntry {
                title: "New terminal".to_string(),
                detail: "launch".to_string(),
                action: PaletteAction::SpawnTerminal,
            };
            if let Some(score) = palette_score(&query, &new_terminal.title) {
                scored.push((score, command_offset, new_terminal));
            }
            let apps = [
                ("Files", "Aether explorer", PaletteAction::OpenExplorer),
                ("Chat", "Aether chat", PaletteAction::OpenChat),
                ("Audio", "Aether audio routes", PaletteAction::OpenAudio),
                (
                    "Agents",
                    "Sandboxed agents · isolation",
                    PaletteAction::OpenSandboxes,
                ),
                (
                    "Themes",
                    "Theme · Water settings",
                    PaletteAction::OpenThemes,
                ),
                (
                    "Keyboard shortcuts",
                    "Aether shortcut reference",
                    PaletteAction::OpenShortcuts,
                ),
            ];
            for (offset, (title, detail, action)) in apps.into_iter().enumerate() {
                if let Some(score) = palette_score(&query, title) {
                    scored.push((
                        score,
                        command_offset + 1 + offset,
                        PaletteEntry {
                            title: title.to_string(),
                            detail: detail.to_string(),
                            action,
                        },
                    ));
                }
            }
        }

        if mode == PaletteMode::Setting {
            let themes = PaletteEntry {
                title: "Themes".to_string(),
                detail: "Theme · Water settings".to_string(),
                action: PaletteAction::OpenThemes,
            };
            if let Some(score) = palette_score(&query, &themes.title) {
                scored.push((score, command_offset, themes));
            }
        }

        let file_offset = command_offset + 1;
        if matches!(mode, PaletteMode::All | PaletteMode::File)
            && (!query.is_empty() || mode == PaletteMode::File)
        {
            let mut hits: Vec<(u8, usize, PaletteEntry)> = if self.files_prefiltered {
                self.files
                    .iter()
                    .enumerate()
                    .map(|(index, file)| {
                        (
                            0,
                            index,
                            PaletteEntry {
                                title: file.name.clone(),
                                detail: file.detail.clone(),
                                action: PaletteAction::OpenFile(index),
                            },
                        )
                    })
                    .collect()
            } else {
                self.files
                    .iter()
                    .enumerate()
                    .filter_map(|(index, file)| {
                        palette_score(&query, &file.name).map(|score| {
                            (
                                score,
                                index,
                                PaletteEntry {
                                    title: file.name.clone(),
                                    detail: file.detail.clone(),
                                    action: PaletteAction::OpenFile(index),
                                },
                            )
                        })
                    })
                    .collect()
            };
            hits.sort_by_key(|(score, index, _)| (*score, *index));
            for (rank, (score, _, entry)) in hits.into_iter().take(8).enumerate() {
                scored.push((score, file_offset + rank, entry));
            }
        }

        let window_offset = file_offset + 8;
        if matches!(mode, PaletteMode::All | PaletteMode::Agent) {
            for (offset, window) in self.windows().into_iter().enumerate() {
                let detail = format!("window {} - workspace {}", window.id, window.workspace + 1);
                if let Some(score) = palette_score(&query, &window.title) {
                    scored.push((
                        score,
                        window_offset + offset,
                        PaletteEntry {
                            title: window.title,
                            detail,
                            action: PaletteAction::FocusWindow(window.id),
                        },
                    ));
                }
            }
        }

        if matches!(mode, PaletteMode::All | PaletteMode::Command) {
            let task = PaletteEntry {
                title: "Task manager".to_string(),
                detail: "Processes · users · cpu · memory · network · disk".to_string(),
                action: PaletteAction::OpenTaskManager,
            };
            if let Some(score) = palette_score(&query, &task.title) {
                scored.push((score, window_offset + self.windows.len(), task));
            }
            let close = PaletteEntry {
                title: "Close palette".to_string(),
                detail: "Esc".to_string(),
                action: PaletteAction::Close,
            };
            if let Some(score) = palette_score(&query, &close.title) {
                scored.push((score, window_offset + self.windows.len() + 1, close));
            }
        }

        scored.sort_by_key(|(score, order, _)| (*score, *order));
        scored.into_iter().map(|(_, _, entry)| entry).collect()
    }

    /// Mode selected by the current palette query.
    pub fn palette_mode(&self) -> PaletteMode {
        parse_palette_query(&self.palette.query).0
    }

    /// 0-based index into [`Shell::palette_entries`].
    pub fn palette_selected(&self) -> usize {
        let len = self.palette_entries().len();
        if len == 0 {
            0
        } else {
            self.palette.selected.min(len - 1)
        }
    }

    /// Select an entry by index. Returns whether the index is valid.
    pub fn palette_select(&mut self, index: usize) -> bool {
        if index >= self.palette_entries().len() {
            return false;
        }
        self.palette.selected = index;
        true
    }

    /// First entry shown in the four-row result window. Sliding the window
    /// keeps the selection visible without an explicit scroll state.
    pub fn palette_visible_start(&self) -> usize {
        let len = self.palette_entries().len();
        if len <= PALETTE_VISIBLE_ROWS {
            0
        } else {
            self.palette_selected()
                .saturating_sub(PALETTE_VISIBLE_ROWS - 1)
                .min(len - PALETTE_VISIBLE_ROWS)
        }
    }

    /// Screen rect of a visible palette row (0-based within the window).
    pub fn palette_row_rect(&self, row: usize) -> Option<Rect> {
        if row >= PALETTE_VISIBLE_ROWS {
            return None;
        }
        let overlay = self.overlay_rect()?;
        Some(Rect::new(
            overlay.x + 10,
            overlay.y + PALETTE_CONTENT_TOP + row as i32 * PALETTE_ROW_SPACING as i32,
            overlay.width.saturating_sub(20),
            PALETTE_ROW_HEIGHT,
        ))
    }

    /// Entry index under a screen point, accounting for the sliding window.
    pub fn palette_row_at(&self, point: Point) -> Option<usize> {
        let overlay = self.overlay_rect()?;
        let content_left = overlay.x + 10;
        let content_right = overlay.x + overlay.width as i32 - 10;
        if point.x < content_left || point.x >= content_right {
            return None;
        }
        let relative = point.y - overlay.y - PALETTE_CONTENT_TOP;
        if relative < 0 {
            return None;
        }
        let row = relative as usize / PALETTE_ROW_SPACING as usize;
        if row >= PALETTE_VISIBLE_ROWS {
            return None;
        }
        let index = self.palette_visible_start() + row;
        (index < self.palette_entries().len()).then_some(index)
    }

    /// Append a printable character to the query. Returns whether it changed.
    pub fn palette_push_char(&mut self, character: char) -> bool {
        if character.is_control() || self.palette.query.chars().count() >= 48 {
            return false;
        }
        self.palette.query.push(character);
        self.palette.selected = 0;
        true
    }

    /// Remove the last query character. Returns whether it changed.
    pub fn palette_backspace(&mut self) -> bool {
        let changed = self.palette.query.pop().is_some();
        if changed {
            self.palette.selected = 0;
        }
        changed
    }

    /// Move the selection by `delta`, wrapping at the ends.
    pub fn palette_move_selection(&mut self, delta: i32) -> bool {
        let len = self.palette_entries().len();
        if len < 2 || delta == 0 {
            return false;
        }
        let current = self.palette_selected() as i32;
        self.palette.selected = (current + delta).rem_euclid(len as i32) as usize;
        true
    }

    /// Activate the selected entry. The caller performs the returned action.
    pub fn palette_activate(&self) -> Option<PaletteAction> {
        self.palette_entries()
            .get(self.palette_selected())
            .map(|entry| entry.action)
    }
}
