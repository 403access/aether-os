//! File explorer overlay and its index-service integration.

use smithay::input::keyboard::{keysyms, xkb, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_explorer(&mut self) {
        if self.shell.explorer_open() {
            self.shell.close_explorer();
        } else {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.shell.close_overlay();
            self.shell.close_chat();
            self.shell.close_audio();
            let _ = self.shell.open_explorer();
            if let Some(worker) = self.index_worker.as_mut() {
                let generation = worker.request_explorer();
                println!("aether-shell: explorer index generation {generation}");
            }
        }
        self.mark_full_damage();
    }

    pub(crate) fn handle_explorer_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let backspace: Keysym = keysyms::KEY_BackSpace.into();
        if keysym == escape {
            self.shell.close_explorer();
            self.mark_full_damage();
            return;
        }
        if keysym == enter {
            if let Some(file) = self.shell.explorer_selected_file() {
                self.open_path(&file.path);
            }
            self.shell.close_explorer();
            self.mark_full_damage();
            return;
        }
        if keysym == up {
            self.shell.explorer_move(-1);
            return;
        }
        if keysym == down {
            self.shell.explorer_move(1);
            return;
        }
        if keysym == page_up {
            self.shell.explorer_page(-1);
            return;
        }
        if keysym == page_down {
            self.shell.explorer_page(1);
            return;
        }
        if keysym == backspace {
            self.shell.explorer_backspace();
            return;
        }
        if let Some(character) = char::from_u32(xkb::keysym_to_utf32(keysym)) {
            if !character.is_control() {
                self.shell.explorer_push_char(character);
            }
        }
    }

    pub(crate) fn mark_explorer_damage(&mut self) {
        if self.shell.explorer_open() {
            let rect = self.shell.explorer_rect();
            self.mark_damage_rect(rect);
        }
    }
}
