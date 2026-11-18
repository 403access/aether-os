//! Palette providers, actions, scoring and overlay constants.

use crate::WindowId;

/// Palette provider mode selected by a leading prefix character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    All,
    Command,
    Agent,
    File,
    Setting,
}

/// Parse `>run`, `@agent`, `/file` or `~setting`; otherwise `All`.
pub fn parse_palette_query(input: &str) -> (PaletteMode, String) {
    let trimmed = input.trim_start();
    let (mode, rest) = match trimmed.chars().next() {
        Some('>') => (PaletteMode::Command, &trimmed[1..]),
        Some('@') => (PaletteMode::Agent, &trimmed[1..]),
        Some('/') => (PaletteMode::File, &trimmed[1..]),
        Some('~') => (PaletteMode::Setting, &trimmed[1..]),
        _ => (PaletteMode::All, trimmed),
    };
    (mode, rest.trim().to_string())
}

/// Visible rows in the palette list.
pub const PALETTE_VISIBLE_ROWS: usize = 6;
/// Height of one result row.
pub const PALETTE_ROW_HEIGHT: u32 = 40;
/// Vertical distance between result rows.
pub const PALETTE_ROW_SPACING: u32 = 46;
/// Y offset of the first result row inside the overlay rect.
pub const PALETTE_CONTENT_TOP: i32 = 66;

/// A command palette result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    pub title: String,
    pub detail: String,
    pub action: PaletteAction,
}

/// What activating a palette entry does. The backend performs side effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteAction {
    SwitchWorkspace(usize),
    FocusWindow(WindowId),
    LaunchApp(usize),
    SpawnTerminal,
    OpenTaskManager,
    OpenCalendar,
    OpenExplorer,
    OpenChat,
    OpenAudio,
    OpenThemes,
    OpenShortcuts,
    OpenSandboxes,
    OpenFile(usize),
    Close,
}

/// An application the palette can launch. The backend spawns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteLauncher {
    pub name: String,
    pub detail: String,
    pub icon: Option<String>,
    pub program: String,
    pub args: Vec<String>,
}

/// A file the palette can open. The backend hands the path to an opener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteFile {
    pub name: String,
    pub detail: String,
    pub path: String,
}

pub(crate) fn palette_score(query: &str, title: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    let title = title.to_lowercase();
    if title == query {
        Some(0)
    } else if title.starts_with(query) {
        Some(1)
    } else if title.contains(query) {
        Some(2)
    } else if is_subsequence(query, &title) {
        Some(3)
    } else {
        None
    }
}

/// Case-insensitive subsequence match, matching the TypeScript palette's
/// fuzzy acceptance (not its numeric score).
pub fn is_subsequence(query: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    query
        .chars()
        .all(|needle| chars.any(|candidate| candidate == needle))
}
