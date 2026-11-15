//! Central shortcut table. `App::handle_shortcut` dispatches only these.

use smithay::input::keyboard::{keysyms, Keysym};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    TogglePalette,
    ToggleExplorer,
    ToggleChat,
    ToggleAudio,
    ToggleThemes,
    ToggleRemote,
    ToggleShortcuts,
    NewTerminal,
    FocusNext,
    FocusPrevious,
    SwitchWorkspace(usize),
    MoveWindowToWorkspace(usize),
    SwitchOutputWorkspace(usize),
    CloseWindow,
    Maximize,
    Fullscreen,
}

/// Keyboard layout profile. Super remains available in both profiles; `Simple`
/// adds the Ctrl+Alt aliases that VNC clients deliver reliably.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyProfile {
    Standard,
    Simple,
}

impl KeyProfile {
    pub const DEFAULT: Self = Self::Simple;

    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Simple => "Simple (default)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Standard => "Super is Command",
            Self::Simple => "Ctrl+Alt for VNC · Super also works",
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("AETHER_KEY_PROFILE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "standard" | "super" => Self::Standard,
            _ => Self::DEFAULT,
        }
    }

    pub fn cheat_sheet_chord(self) -> &'static str {
        match self {
            Self::Standard => "Super+/",
            Self::Simple => "Ctrl+Alt+/",
        }
    }

    pub fn entries(self) -> &'static [ShortcutEntry] {
        match self {
            Self::Standard => STANDARD_SHORTCUTS,
            Self::Simple => SIMPLE_SHORTCUTS,
        }
    }
}

/// One row in the in-shell cheat sheet and the manual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutEntry {
    pub chord: &'static str,
    pub action: &'static str,
    pub group: &'static str,
}

/// Canonical fast-path list. Keep this in sync with `lookup` and the manual.
pub const STANDARD_SHORTCUTS: &[ShortcutEntry] = &[
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
        chord: "Super+T",
        action: "Themes",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Super+R",
        action: "Remote session",
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

pub const SIMPLE_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        chord: "Ctrl+Alt+Space",
        action: "Command palette",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+Return",
        action: "New terminal",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+E",
        action: "Explorer",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+C",
        action: "Chat",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+A",
        action: "Audio routes",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+T",
        action: "Themes",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+R",
        action: "Remote session",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+/",
        action: "Shortcut cheat sheet",
        group: "Surfaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+Tab",
        action: "Focus next (MRU)",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+Shift+Tab",
        action: "Focus previous (MRU)",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+J",
        action: "Focus next",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+K",
        action: "Focus previous",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+M",
        action: "Maximize / restore",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+F",
        action: "Fullscreen / restore",
        group: "Windows",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+Q",
        action: "Close window",
        group: "Windows",
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
        chord: "Ctrl+Alt+1/2/3",
        action: "Switch workspace",
        group: "Workspaces",
    },
    ShortcutEntry {
        chord: "Ctrl+Alt+Shift+1/2/3",
        action: "Move window to workspace",
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

/// Resolve a key against a profile. Simple checks Ctrl+Alt first, then falls
/// through to the Super map so muscle memory keeps working everywhere.
pub fn lookup_profile(
    profile: KeyProfile,
    logo: bool,
    ctrl: bool,
    shift: bool,
    alt: bool,
    keysym: Keysym,
) -> Option<Shortcut> {
    if profile == KeyProfile::Simple && ctrl && alt {
        let space: Keysym = keysyms::KEY_space.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let slash: Keysym = keysyms::KEY_slash.into();
        let tab: Keysym = keysyms::KEY_Tab.into();
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
            return Some(if shift {
                Shortcut::MoveWindowToWorkspace(index)
            } else {
                Shortcut::SwitchWorkspace(index)
            });
        }
        let shortcut = if keysym == space {
            Some(Shortcut::TogglePalette)
        } else if keysym == enter {
            Some(Shortcut::NewTerminal)
        } else if keysym == slash {
            Some(Shortcut::ToggleShortcuts)
        } else if keysym == tab {
            Some(if shift {
                Shortcut::FocusPrevious
            } else {
                Shortcut::FocusNext
            })
        } else if keysym == keysyms::KEY_e.into() {
            Some(Shortcut::ToggleExplorer)
        } else if keysym == keysyms::KEY_c.into() {
            Some(Shortcut::ToggleChat)
        } else if keysym == keysyms::KEY_a.into() {
            Some(Shortcut::ToggleAudio)
        } else if keysym == keysyms::KEY_t.into() {
            Some(Shortcut::ToggleThemes)
        } else if keysym == keysyms::KEY_r.into() {
            Some(Shortcut::ToggleRemote)
        } else if keysym == keysyms::KEY_j.into() {
            Some(Shortcut::FocusNext)
        } else if keysym == keysyms::KEY_k.into() {
            Some(Shortcut::FocusPrevious)
        } else if keysym == keysyms::KEY_m.into() {
            Some(Shortcut::Maximize)
        } else if keysym == keysyms::KEY_f.into() {
            Some(Shortcut::Fullscreen)
        } else if keysym == keysyms::KEY_q.into() {
            Some(Shortcut::CloseWindow)
        } else {
            None
        };
        if shortcut.is_some() {
            return shortcut;
        }
    }
    lookup(logo, shift, alt, keysym)
}

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
    let chat: Keysym = keysyms::KEY_c.into();
    let audio: Keysym = keysyms::KEY_a.into();
    let themes: Keysym = keysyms::KEY_t.into();
    let remote: Keysym = keysyms::KEY_r.into();
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
        _ if keysym == chat => Shortcut::ToggleChat,
        _ if keysym == audio => Shortcut::ToggleAudio,
        _ if keysym == themes => Shortcut::ToggleThemes,
        _ if keysym == remote => Shortcut::ToggleRemote,
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
        assert!(STANDARD_SHORTCUTS.len() >= 20);
        let mut chords: Vec<&str> = STANDARD_SHORTCUTS.iter().map(|entry| entry.chord).collect();
        chords.sort_unstable();
        chords.dedup();
        assert_eq!(chords.len(), STANDARD_SHORTCUTS.len());
        assert!(STANDARD_SHORTCUTS
            .iter()
            .any(|entry| entry.chord == "Super+Return"));
        assert!(STANDARD_SHORTCUTS
            .iter()
            .any(|entry| entry.chord == "Super+/"));
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
            lookup(true, false, false, key(keysyms::KEY_t)),
            Some(Shortcut::ToggleThemes)
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
            lookup(true, false, false, key(keysyms::KEY_q)),
            Some(Shortcut::CloseWindow)
        );
    }

    #[test]
    fn simple_profile_uses_ctrl_alt_aliases() {
        let palette = lookup_profile(
            KeyProfile::Simple,
            false,
            true,
            false,
            true,
            key(keysyms::KEY_space),
        );
        assert_eq!(palette, Some(Shortcut::TogglePalette));
        assert_eq!(
            lookup_profile(
                KeyProfile::Standard,
                false,
                true,
                false,
                true,
                key(keysyms::KEY_space)
            ),
            None
        );
        assert_eq!(
            lookup_profile(
                KeyProfile::Simple,
                false,
                true,
                false,
                true,
                key(keysyms::KEY_Return)
            ),
            Some(Shortcut::NewTerminal)
        );
        assert_eq!(
            lookup_profile(
                KeyProfile::Simple,
                false,
                true,
                false,
                true,
                key(keysyms::KEY_e)
            ),
            Some(Shortcut::ToggleExplorer)
        );
    }

    #[test]
    fn simple_profile_keeps_super_as_fallback() {
        assert_eq!(
            lookup_profile(
                KeyProfile::Simple,
                true,
                false,
                false,
                false,
                key(keysyms::KEY_space)
            ),
            Some(Shortcut::TogglePalette)
        );
    }
}
