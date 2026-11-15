//! Chat drawer: token streaming, scrolling and demo polling.

use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_chat(&mut self) {
        if self.shell.chat_open() {
            self.shell.close_chat();
        } else {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.shell.close_overlay();
            self.shell.close_explorer();
            self.shell.close_audio();
            let _ = self.shell.open_chat();
        }
        self.mark_full_damage();
    }

    pub(crate) fn handle_chat_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        if keysym == escape {
            self.shell.close_chat();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.chat_scroll_by(-1);
        } else if keysym == down {
            self.shell.chat_scroll_by(1);
        } else if keysym == page_up {
            self.shell.chat_page(-1);
        } else if keysym == page_down {
            self.shell.chat_page(1);
        }
    }

    /// Stream a token into the chat drawer (external producers/tests).
    pub fn push_chat_token(&mut self, token: &str) {
        if !self.shell.chat_open() {
            let _ = self.shell.open_chat();
        }
        self.shell.chat_push_token(token);
        self.mark_chat_damage();
    }

    pub(crate) fn poll_chat_demo(&mut self) {
        let tokens = self
            .chat_demo
            .as_ref()
            .map(|demo| demo.drain())
            .unwrap_or_default();
        if tokens.is_empty() {
            return;
        }
        for token in tokens {
            self.shell.chat_push_token(&token);
        }
        self.mark_chat_damage();
    }

    pub(crate) fn mark_chat_damage(&mut self) {
        if self.shell.chat_open() {
            let rect = self.shell.chat_rect();
            self.mark_damage_rect(rect);
        }
    }
}
