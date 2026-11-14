//! Central shortcut table. `App::handle_shortcut` dispatches only these.

use smithay::input::keyboard::{keysyms, Keysym};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    TogglePalette,
    ToggleExplorer,
    ToggleChat,
    ToggleAudio,
    FocusNext,
    FocusPrevious,
    SwitchWorkspace(usize),
    MoveWindowToWorkspace(usize),
    SwitchOutputWorkspace(usize),
    CloseWindow,
    Maximize,
    Fullscreen,
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
    let explorer: Keysym = keysyms::KEY_e.into();
    let chat: Keysym = keysyms::KEY_c.into();
    let audio: Keysym = keysyms::KEY_a.into();
    let close: Keysym = keysyms::KEY_q.into();
    let maximize: Keysym = keysyms::KEY_m.into();
    let fullscreen: Keysym = keysyms::KEY_f.into();
    Some(match keysym {
        _ if keysym == space => Shortcut::TogglePalette,
        _ if keysym == explorer => Shortcut::ToggleExplorer,
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
            lookup(true, false, false, key(keysyms::KEY_q)),
            Some(Shortcut::CloseWindow)
        );
    }
}
