//! Keyboard routing: global shortcuts, modal overlays and client forwarding.

use smithay::backend::input::{KeyState, Keycode};
use smithay::input::keyboard::{keysyms, FilterResult, KeyboardHandle, Keysym, ModifiersState};

use crate::keymap::{lookup_profile, Shortcut};

use super::{cursor_for, App};

impl App {
    fn handle_shortcut(&mut self, modifiers: &ModifiersState, keysym: Keysym) -> bool {
        let escape: Keysym = keysyms::KEY_Escape.into();
        if self.shortcuts_open && keysym == escape {
            self.close_shortcuts();
            return true;
        }
        if self.shell.overlay().is_some() && keysym == escape {
            self.toggle_overlay();
            return true;
        }
        let Some(shortcut) = lookup_profile(
            self.key_profile,
            modifiers.logo,
            modifiers.ctrl,
            modifiers.shift,
            modifiers.alt,
            keysym,
        ) else {
            return false;
        };
        match shortcut {
            Shortcut::TogglePalette => self.toggle_overlay(),
            Shortcut::ToggleExplorer => self.toggle_explorer(),
            Shortcut::ToggleChat => self.toggle_chat(),
            Shortcut::ToggleAudio => self.toggle_audio(),
            Shortcut::ToggleThemes => self.toggle_themes(),
            Shortcut::ToggleRemote => self.toggle_remote(),
            Shortcut::ToggleShortcuts => self.toggle_shortcuts(),
            Shortcut::NewTerminal => {
                self.spawn_terminal();
                println!("aether-shell: shortcut new terminal");
            }
            Shortcut::FocusNext | Shortcut::FocusPrevious => {
                let delta = if shortcut == Shortcut::FocusNext {
                    1
                } else {
                    -1
                };
                if let Some(id) = self.shell.focus_mru(delta) {
                    self.sync_focus();
                    self.configure_all();
                    self.mark_full_damage();
                    println!("aether-shell: mru focus {id}");
                }
            }
            Shortcut::SwitchWorkspace(workspace) => self.switch_workspace(workspace),
            Shortcut::MoveWindowToWorkspace(workspace) => {
                if let Some(id) = self.shell.focused_window() {
                    if self.shell.move_window_to_workspace(id, workspace) {
                        println!(
                            "aether-shell-smithay: moved window {id} -> workspace {workspace}"
                        );
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
            Shortcut::SwitchOutputWorkspace(workspace) => {
                let output = self.output_under_pointer();
                if self.shell.set_output_workspace(output, workspace) {
                    self.sync_focus();
                    self.configure_all();
                    self.mark_full_damage();
                    println!("aether-shell: output {output} workspace -> {workspace}");
                }
            }
            Shortcut::CloseWindow => {
                if let Some(id) = self.shell.focused_window() {
                    if let Some(toplevel) = self.windows.get(&id) {
                        toplevel.send_close();
                    }
                }
            }
            Shortcut::Maximize => {
                if let Some(id) = self.shell.focused_window() {
                    let maximized = !self.shell.is_maximized(id);
                    if self.shell.toggle_maximized(id) {
                        println!("aether-shell-smithay: maximized -> {maximized}");
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
            Shortcut::Fullscreen => {
                if let Some(id) = self.shell.focused_window() {
                    let fullscreen = !self.shell.is_fullscreen(id);
                    if self.shell.toggle_fullscreen(id) {
                        println!("aether-shell-smithay: fullscreen -> {fullscreen}");
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
        }
        true
    }

    /// Route a key event through xkb, shell shortcuts and the focused client.
    pub fn keyboard_key(
        &mut self,
        keyboard: &KeyboardHandle<Self>,
        key_code: Keycode,
        state: KeyState,
        time: u32,
    ) {
        let started = self.begin_input();
        let pressed = state == KeyState::Pressed;
        let old_cursor = self.cursor;
        let old_overlay = self.shell.overlay_rect();
        let mut shortcut = false;
        let mut palette_key = false;
        let mut explorer_key = false;
        let mut chat_key_pressed = false;
        let mut audio_key = false;
        let mut theme_key = false;
        let mut remote_key = false;
        keyboard.input::<(), _>(
            self,
            key_code,
            state,
            0.into(),
            time,
            |state, modifiers, keysym| {
                state.modifiers = *modifiers;
                let sym = keysym.modified_sym();
                if pressed && state.handle_shortcut(modifiers, sym) {
                    shortcut = true;
                    FilterResult::Intercept(())
                } else if state.shortcuts_open {
                    FilterResult::Intercept(())
                } else if state.task_manager_open() {
                    if pressed {
                        state.handle_task_key(sym);
                    }
                    FilterResult::Intercept(())
                } else if state.shell.overlay().is_some() {
                    if pressed {
                        state.handle_palette_key(sym);
                        state.refresh_index_results();
                        palette_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.explorer_open() {
                    if pressed {
                        state.handle_explorer_key(sym);
                        explorer_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.audio_open() {
                    if pressed {
                        state.handle_audio_key(sym);
                        audio_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.themes_open() {
                    if pressed {
                        state.handle_theme_key(sym);
                        theme_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.remote_open() {
                    if pressed {
                        state.handle_remote_key(sym);
                        remote_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.chat_open() {
                    if pressed {
                        state.handle_chat_key(sym);
                        chat_key_pressed = true;
                    }
                    FilterResult::Intercept(())
                } else {
                    FilterResult::Forward
                }
            },
        );
        self.modifiers = keyboard.modifier_state();
        let new_cursor = cursor_for(self);
        self.cursor = new_cursor;
        if new_cursor != old_cursor {
            self.mark_cursor_damage(self.pointer_pos);
        }
        if shortcut {
            // Shortcuts may move/close/raise windows or switch workspaces.
            self.mark_full_damage();
        } else if explorer_key {
            self.mark_explorer_damage();
        } else if chat_key_pressed {
            self.mark_chat_damage();
        } else if audio_key {
            self.mark_audio_damage();
        } else if theme_key {
            self.mark_theme_damage();
        } else if remote_key {
            self.mark_remote_damage();
        } else if self.shortcuts_open {
            self.mark_shortcuts_damage();
        } else if self.task_manager_open() {
            self.mark_task_damage();
        } else if palette_key {
            let new_overlay = self.shell.overlay_rect();
            if let Some(rect) = old_overlay {
                self.mark_damage_rect(rect);
            }
            if let Some(rect) = new_overlay {
                if Some(rect) != old_overlay {
                    self.mark_damage_rect(rect);
                }
            } else {
                // Enter/activation closed the palette and may have launched or
                // switched workspace.
                self.mark_full_damage();
            }
        }
        self.end_input(started);
    }
}
