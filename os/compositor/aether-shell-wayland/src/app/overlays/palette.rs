//! Launcher palette: providers, activation and query editing.

use aether_shell::PaletteAction;
use smithay::input::keyboard::{keysyms, xkb, Keysym};

use super::super::App;

impl App {
    pub fn toggle_overlay(&mut self) {
        if self.shell.overlay().is_none() {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.close_sandboxes();
            self.shell.close_audio();
            self.shell.close_remote();
            self.shell.close_explorer();
            self.shell.close_chat();
            self.refresh_providers();
        }
        let output = self.output_under_pointer();
        self.shell.toggle_overlay_on(output);
        self.mark_full_damage();
    }

    pub(crate) fn activate_palette(&mut self) {
        let Some(action) = self.shell.palette_activate() else {
            return;
        };
        println!("aether-shell-smithay: palette action {action:?}");
        self.mark_full_damage();
        match action {
            PaletteAction::SwitchWorkspace(index) => {
                let output = self.shell.overlay_output().unwrap_or(1);
                self.shell.close_overlay();
                self.switch_workspace_on(output, index);
            }
            PaletteAction::FocusWindow(id) => {
                self.shell.close_overlay();
                let target = self
                    .shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == id)
                    .map(|window| (window.output, window.workspace));
                if let Some((output, workspace)) = target {
                    if self.shell.output_workspace(output) != workspace {
                        let primary = self.shell.primary_output().map(|output| output.id);
                        if Some(output) == primary {
                            self.switch_workspace(workspace);
                        } else {
                            self.shell.set_output_workspace(output, workspace);
                        }
                    }
                    if self.shell.focus_window(id) {
                        self.sync_focus();
                    }
                    self.configure_all();
                }
            }
            PaletteAction::LaunchApp(index) => {
                self.shell.close_overlay();
                if let Some(launcher) = self.shell.launcher(index) {
                    self.spawn_launcher(launcher);
                }
            }
            PaletteAction::SpawnTerminal => {
                self.shell.close_overlay();
                self.spawn_terminal();
            }
            PaletteAction::OpenTaskManager => {
                self.shell.close_overlay();
                self.open_task_manager();
            }
            PaletteAction::OpenExplorer => {
                self.shell.close_overlay();
                self.toggle_explorer();
            }
            PaletteAction::OpenChat => {
                self.shell.close_overlay();
                self.toggle_chat();
            }
            PaletteAction::OpenAudio => {
                self.shell.close_overlay();
                self.toggle_audio();
            }
            PaletteAction::OpenThemes => {
                self.shell.close_overlay();
                self.toggle_themes();
            }
            PaletteAction::OpenShortcuts => {
                self.shell.close_overlay();
                self.toggle_shortcuts();
            }
            PaletteAction::OpenSandboxes => {
                self.shell.close_overlay();
                self.toggle_sandboxes();
            }
            PaletteAction::OpenFile(index) => {
                self.shell.close_overlay();
                if let Some(file) = self.shell.file(index) {
                    self.open_path(&file.path);
                }
            }
            PaletteAction::Close => {
                self.shell.close_overlay();
            }
        }
    }

    pub(crate) fn handle_palette_key(&mut self, keysym: Keysym) {
        let enter: Keysym = keysyms::KEY_Return.into();
        let keypad_enter: Keysym = keysyms::KEY_KP_Enter.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let backspace: Keysym = keysyms::KEY_BackSpace.into();

        if keysym == enter || keysym == keypad_enter {
            self.activate_palette();
            return;
        }
        if keysym == up {
            self.shell.palette_move_selection(-1);
            return;
        }
        if keysym == down {
            self.shell.palette_move_selection(1);
            return;
        }
        if keysym == backspace {
            if self.shell.palette_backspace() {
                println!(
                    "aether-shell-smithay: palette query -> {:?}",
                    self.shell.palette_query()
                );
            }
            return;
        }

        let Some(character) = char::from_u32(xkb::keysym_to_utf32(keysym)) else {
            return;
        };
        if !character.is_control() && self.shell.palette_push_char(character) {
            println!(
                "aether-shell-smithay: palette query -> {:?}",
                self.shell.palette_query()
            );
        }
    }

    pub(crate) fn mark_overlay_damage(&mut self) {
        if let Some(rect) = self.shell.overlay_rect() {
            self.mark_damage_rect(rect);
        }
    }
}
