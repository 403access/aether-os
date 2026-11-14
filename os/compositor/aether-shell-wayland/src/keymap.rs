//! Central shortcut table. `App::handle_shortcut` dispatches only these.

use smithay::input::keyboard::{keysyms, Keysym};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    TogglePalette,
    ToggleExplorer,
    ToggleChat,
    ToggleAudio,
    ToggleShortcuts,
    NewTerminal,
    AdjustMaster(i8),
    FocusNext,
    FocusPrevious,
    SwitchWorkspace(usize),
    MoveWindowToWorkspace(usize),
    SwitchOutputWorkspace(usize),
    CloseWindow,
    Maximize,
    Fullscreen,
}

/// One row in the in-shell cheat sheet and the manual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutEntry {
    pub chord: &'static str,
    pub action: &'static str,
    pub group: &'static str,
}

/// Canonical fast-path list. Keep this in sync with `lookup` and the manual.
pub const SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        chord: "Super+Space",
        action: "Command palette",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+Return",
        action: "New terminal",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+E",
        action: "Explorer",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+C",
        action: "Chat",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+A",
        action: "Audio routes",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+/",
        action: "Shortcut cheat sheet",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+Tab",
        action: "Focus next (MRU)",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+Shift+Tab",
        action: "Focus previous (MRU)",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+J",
        action: "Focus next",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+K",
        action: "Focus previous",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+M",
        action: "Maximize / restore",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+F",
        action: "Fullscreen / restore",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+Q",
        action: "Close window",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Super+H",
        action: "Narrow master",
        group: "Layout",
    },
    ShortcutEntry {
        chord: "Super+L",
        action: "Widen master",
        group: "Layout",
    },
    ShortcutEntry {
        chord: "Alt+Left drag",
        action: "Move / float window",
        group: "Layout",
    },
    ShortcutEntry {
        chord: "Alt+Right drag",
        action: "Resize floating window",
        group: "Layout",
    },
    ShortcutEntry {
        chord: "Super+1/2/3",
        action: "Switch workspace",
        group: "Workspaces",
    },
    ShortcutEntry {
        chord: "Super+Shift+1/2/3",
        action: "Move window to workspace",
        group: "Workspaces",
    },
    ShortcutEntry {
        chord: "Super+Alt+1/2/3",
        action: "Switch output workspace",
        group: "Workspaces",
    },
    ShortcutEntry {
        chord: "↑ / ↓",
        action: "Move selection",
        group: "Overlays",
    },
    ShortcutEntry {
        chord: "Enter",
        action: "Activate selection",
        group: "Overlays",
    },
    ShortcutEntry {
        chord: "PgUp / PgDn",
        action: "Page lists / chat",
        group: "Overlays",
    },
    ShortcutEntry {
        chord: "P",
        action: "Cycle audio latency preset",
        group: "Overlays",
    },
    ShortcutEntry {
        chord: "Esc",
        action: "Close overlay",
        group: "Overlays",
    },
];

/// Resolve a logo-modified key. Escape is handled by the overlay before this.
pub fn lookup(logo: bool, shift: bool, alt: bool, keysym: Keysym) -> Option<Shortcut> {
    if !logo {
        return None;
    }
    let one: Keysym = keysyms::KEY_1.into();
    let two: Keysym = keysyms::KEY_2.into();
    let three: Keysym = keysyms::KEY_3.into();
    let index = if keysym == one {
        Some(0)
    } else if keysym == two {
        Some(1)
    } else if keysym == three {
        Some(2)
    } else {
        None
    };
    if let Some(index) = index {
        return Some(if alt {
            Shortcut::SwitchOutputWorkspace(index)
        } else if shift {
            Shortcut::MoveWindowToWorkspace(index)
        } else {
            Shortcut::SwitchWorkspace(index)
        });
    }
    let tab: Keysym = keysyms::KEY_Tab.into();
    if keysym == tab {
        return Some(if shift {
            Shortcut::FocusPrevious
        } else {
            Shortcut::FocusNext
        });
    }
    let space: Keysym = keysyms::KEY_space.into();
    let enter: Keysym = keysyms::KEY_Return.into();
    let slash: Keysym = keysyms::KEY_slash.into();
    let explorer: Keysym = keysyms::KEY_e.into();
    let next_window: Keysym = keysyms::KEY_j.into();
    let previous_window: Keysym = keysyms::KEY_k.into();
    let narrow_master: Keysym = keysyms::KEY_h.into();
    let wide_master: Keysym = keysyms::KEY_l.into();
    let chat: Keysym = keysyms::KEY_c.into();
    let audio: Keysym = keysyms::KEY_a.into();
    let close: Keysym = keysyms::KEY_q.into();
    let maximize: Keysym = keysyms::KEY_m.into();
    let fullscreen: Keysym = keysyms::KEY_f.into();
    Some(match keysym {
        _ if keysym == space => Shortcut::TogglePalette,
        _ if keysym == enter => Shortcut::NewTerminal,
        _ if keysym == slash => Shortcut::ToggleShortcuts,
        _ if keysym == explorer => Shortcut::ToggleExplorer,
        _ if keysym == next_window => Shortcut::FocusNext,
        _ if keysym == previous_window => Shortcut::FocusPrevious,
        _ if keysym == narrow_master => Shortcut::AdjustMaster(-1),
        _ if keysym == wide_master => Shortcut::AdjustMaster(1),
        _ if keysym == chat => Shortcut::ToggleChat,
        _ if keysym == audio => Shortcut::ToggleAudio,
        _ if keysym == close => Shortcut::CloseWindow,
        _ if keysym == maximize => Shortcut::Maximize,
        _ if keysym == fullscreen => Shortcut::Fullscreen,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: u32) -> Keysym {
        Keysym::new(name)
    }

    #[test]
    fn workspace_modifiers_are_distinct() {
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_2)),
            Some(Shortcut::SwitchWorkspace(1))
        );
        assert_eq!(
            lookup(true, true, false, key(keysyms::KEY_2)),
            Some(Shortcut::MoveWindowToWorkspace(1))
        );
        assert_eq!(
            lookup(true, false, true, key(keysyms::KEY_2)),
            Some(Shortcut::SwitchOutputWorkspace(1))
        );
    }

    #[test]
    fn tab_cycles_mru_and_logo_is_required() {
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_Tab)),
            Some(Shortcut::FocusNext)
        );
        assert_eq!(
            lookup(true, true, false, key(keysyms::KEY_Tab)),
            Some(Shortcut::FocusPrevious)
        );
        assert_eq!(lookup(false, false, false, key(keysyms::KEY_Tab)), None);
    }

    #[test]
    fn shortcut_table_is_unique_and_complete() {
        assert!(SHORTCUTS.len() >= 20);
        let mut chords: Vec<&str> = SHORTCUTS.iter().map(|entry| entry.chord).collect();
        chords.sort_unstable();
        chords.dedup();
        assert_eq!(chords.len(), SHORTCUTS.len());
        assert!(SHORTCUTS.iter().any(|entry| entry.chord == "Super+Return"));
        assert!(SHORTCUTS.iter().any(|entry| entry.chord == "Super+/"));
    }

    #[test]
    fn shell_surfaces_and_window_actions() {
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_space)),
            Some(Shortcut::TogglePalette)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_e)),
            Some(Shortcut::ToggleExplorer)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_c)),
            Some(Shortcut::ToggleChat)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_a)),
            Some(Shortcut::ToggleAudio)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_Return)),
            Some(Shortcut::NewTerminal)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_slash)),
            Some(Shortcut::ToggleShortcuts)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_j)),
            Some(Shortcut::FocusNext)
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_h)),
            Some(Shortcut::AdjustMaster(-1))
        );
        assert_eq!(
            lookup(true, false, false, key(keysyms::KEY_q)),
            Some(Shortcut::CloseWindow)
        );
    }
}
