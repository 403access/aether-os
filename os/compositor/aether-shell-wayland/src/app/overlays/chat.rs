//! Chat drawer: token streaming, scrolling and demo polling.

use aether_protocol::chat::ChatEvent;
use smithay::input::keyboard::{keysyms, xkb, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_chat(&mut self) {
        if self.shell.chat_open() {
            self.shell.close_chat();
        } else {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.close_sandboxes();
            self.shell.close_overlay();
            self.shell.close_explorer();
            self.shell.close_audio();
            self.shell.close_remote();
            let _ = self.shell.open_chat();
        }
        self.mark_full_damage();
    }

    pub(crate) fn handle_chat_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let keypad_enter: Keysym = keysyms::KEY_KP_Enter.into();
        let backspace: Keysym = keysyms::KEY_BackSpace.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        if keysym == escape {
            self.shell.close_chat();
            self.mark_full_damage();
        } else if keysym == enter || keysym == keypad_enter {
            if let Some(prompt) = self.shell.chat_take_input() {
                self.submit_chat_prompt(prompt);
            }
        } else if keysym == backspace {
            self.shell.chat_backspace();
        } else if keysym == up {
            self.shell.chat_scroll_by(-1);
        } else if keysym == down {
            self.shell.chat_scroll_by(1);
        } else if keysym == page_up {
            self.shell.chat_page(-1);
        } else if keysym == page_down {
            self.shell.chat_page(1);
        } else if let Some(character) = char::from_u32(xkb::keysym_to_utf32(keysym)) {
            if !character.is_control() {
                self.shell.chat_push_char(character);
            }
        }
    }

    /// Send a prompt to the chat backend and echo it into the transcript.
    pub(crate) fn submit_chat_prompt(&mut self, prompt: String) {
        if self.chat_bridge.connected() && self.chat_bridge.send_prompt(&prompt) {
            println!("aether-shell: chat prompt -> backend");
            self.shell.chat_push_user_line(&prompt);
        } else {
            println!("aether-shell: chat prompt without backend");
            self.shell.chat_push_user_line(&prompt);
            self.shell
                .chat_push_token("\n_chat backend is not connected — set AETHER_CHAT_SOCKET_\n");
            self.shell.chat_finish_stream();
        }
        self.mark_chat_damage();
    }

    pub(crate) fn poll_chat_bridge(&mut self) {
        let events = self.chat_bridge.poll();
        if events.is_empty() {
            return;
        }
        println!("aether-shell: chat events -> {}", events.len());
        if !self.shell.chat_open() {
            let _ = self.shell.open_chat();
        }
        for event in events {
            match event {
                ChatEvent::Token(text) => self.shell.chat_push_token(&text),
                ChatEvent::Tool {
                    name,
                    status,
                    detail,
                } => {
                    let suffix = if detail.is_empty() {
                        String::new()
                    } else {
                        format!(" · {detail}")
                    };
                    self.shell
                        .chat_push_token(&format!("\n`{name}` {status}{suffix}\n"));
                }
                ChatEvent::Done => self.shell.chat_finish_stream(),
                ChatEvent::Error(message) => {
                    self.shell
                        .chat_push_token(&format!("\n**error** {message}\n"));
                    self.shell.chat_finish_stream();
                }
            }
        }
        self.mark_chat_damage();
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
