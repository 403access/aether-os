//! Themes app: theme mode, Water effects and display settings.
//!
//! The panel model lives in `aether_shell::theme`; this module owns the
//! desktop interaction and persists the active settings to
//! `$XDG_CONFIG_HOME/aether/themes.json` after every change.

use aether_shell::theme::{ThemeAction, ThemeField};
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_themes(&mut self) {
        if self.shell.themes_open() {
            self.shell.close_themes();
            self.mark_full_damage();
            return;
        }
        self.shortcuts_open = false;
        self.close_task_manager();
        self.shell.open_themes();
        let settings = *self.shell.theme_settings();
        println!(
            "aether-shell: themes opened ({} · {} · {})",
            settings.mode.label(),
            settings.tone.label(),
            settings.status_text()
        );
        self.mark_full_damage();
    }

    pub(crate) fn handle_theme_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let space: Keysym = keysyms::KEY_space.into();
        let keypad_enter: Keysym = keysyms::KEY_KP_Enter.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let left: Keysym = keysyms::KEY_Left.into();
        let right: Keysym = keysyms::KEY_Right.into();
        let reset: Keysym = keysyms::KEY_r.into();
        let water: Keysym = keysyms::KEY_w.into();

        let selected = self
            .shell
            .theme_layer()
            .map(|layer| layer.selected())
            .unwrap_or(0);
        let is_switch = ThemeField::ALL
            .get(selected)
            .is_some_and(|field| field.is_switch());

        if keysym == escape {
            self.shell.close_themes();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.theme_move_selection(-1);
            self.mark_theme_damage();
        } else if keysym == down {
            self.shell.theme_move_selection(1);
            self.mark_theme_damage();
        } else if keysym == left || keysym == right {
            if !is_switch {
                let delta = if keysym == left { -1 } else { 1 };
                let action = self.shell.theme_adjust(delta);
                self.apply_theme_action(action);
            }
            self.mark_theme_damage();
        } else if keysym == enter || keysym == keypad_enter || keysym == space {
            let action = self.shell.theme_activate();
            self.apply_theme_action(action);
        } else if keysym == reset {
            let action = self.shell.theme_reset();
            self.apply_theme_action(Some(action));
        } else if keysym == water {
            let action = self.shell.theme_use_water();
            self.apply_theme_action(action);
        }
    }

    pub(crate) fn apply_theme_action(&mut self, action: Option<ThemeAction>) {
        let Some(action) = action else {
            return;
        };
        self.persist_theme();
        let settings = *self.shell.theme_settings();
        println!(
            "aether-shell: theme {action:?} -> {} · {} · {}{}",
            settings.mode.label(),
            settings.tone.label(),
            settings.background.label(),
            if settings.glass { "" } else { " · opaque" }
        );
        // Mode, tone and glass change the chrome vocabulary globally.
        self.mark_full_damage();
    }

    pub(crate) fn persist_theme(&self) {
        let settings = *self.shell.theme_settings();
        match settings.save() {
            Ok(path) => println!("aether-shell: theme saved to {}", path.display()),
            Err(error) => eprintln!("aether-shell: theme save failed: {error}"),
        }
    }

    pub(crate) fn mark_theme_damage(&mut self) {
        if self.shell.themes_open() {
            let rect = self.shell.theme_rect();
            self.mark_damage_rect(rect);
        }
    }
}
