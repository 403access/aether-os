//! Streaming chat drawer state.

use aether_geometry::Rect;

use crate::model::chat::{chat_materialize, ChatLine, ChatState};

use super::Shell;

impl Shell {
    pub fn chat_rect(&self) -> Rect {
        let bounds = self
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport));
        let width = 400.min(bounds.width.saturating_sub(32)).max(1);
        // Start below the 38 px menu bar and stop above the dock.
        Rect::new(
            bounds.right() as i32 - width as i32 - 16,
            bounds.y + 54,
            width,
            bounds.height.saturating_sub(54 + 86),
        )
    }

    pub fn open_chat(&mut self) -> Option<()> {
        self.close_remote();
        self.close_themes();
        if self.chat.is_none() {
            self.chat = Some(ChatState::default());
        }
        Some(())
    }

    pub fn close_chat(&mut self) {
        self.chat = None;
    }

    pub fn chat_open(&self) -> bool {
        self.chat.is_some()
    }

    pub fn chat_line_count(&self) -> usize {
        self.chat.as_ref().map(|chat| chat.lines.len()).unwrap_or(0)
    }

    pub fn chat_scroll(&self) -> usize {
        self.chat.as_ref().map(|chat| chat.scroll).unwrap_or(0)
    }

    pub fn chat_visible_rows(&self) -> usize {
        self.chat.as_ref().map(|chat| chat.visible).unwrap_or(0)
    }

    /// Lines that enter the render plan (bounded by the drawer height).
    pub fn visible_chat_lines(&self) -> &[ChatLine] {
        let Some(chat) = self.chat.as_ref() else {
            return &[];
        };
        let end = (chat.scroll + chat.visible).min(chat.lines.len());
        if chat.scroll >= end {
            return &[];
        }
        &chat.lines[chat.scroll..end]
    }

    /// Feed a token (possibly containing newlines). Parsing is incremental.
    pub fn chat_push_token(&mut self, token: &str) {
        let Some(chat) = self.chat.as_mut() else {
            return;
        };
        let was_at_bottom = chat.scroll + chat.visible >= chat.lines.len();
        chat.pending.push_str(token);
        while let Some(newline) = chat.pending.find('\n') {
            let line: String = chat.pending.drain(..=newline).collect();
            let line = line.trim_end_matches(['\n', '\r']);
            chat_materialize(chat, line);
        }
        let max_scroll = chat.lines.len().saturating_sub(chat.visible);
        if was_at_bottom {
            chat.scroll = max_scroll;
        }
    }

    pub fn chat_finish_stream(&mut self) {
        let Some(chat) = self.chat.as_mut() else {
            return;
        };
        let was_at_bottom = chat.scroll + chat.visible >= chat.lines.len();
        if !chat.pending.is_empty() {
            let line = std::mem::take(&mut chat.pending);
            chat_materialize(chat, &line);
            if was_at_bottom {
                chat.scroll = chat.lines.len().saturating_sub(chat.visible);
            }
        }
    }

    pub fn chat_scroll_by(&mut self, delta: isize) {
        let Some(chat) = self.chat.as_mut() else {
            return;
        };
        let max_scroll = chat.lines.len().saturating_sub(chat.visible) as isize;
        chat.scroll = (chat.scroll as isize + delta).clamp(0, max_scroll) as usize;
    }

    pub fn chat_page(&mut self, direction: isize) {
        let page = self
            .chat
            .as_ref()
            .map(|chat| chat.visible as isize)
            .unwrap_or(1);
        self.chat_scroll_by(direction * page);
    }

    /// Current composer text. Empty when the drawer is closed.
    pub fn chat_input(&self) -> &str {
        self.chat
            .as_ref()
            .map(|chat| chat.input.as_str())
            .unwrap_or("")
    }

    /// Append a printable character to the composer.
    pub fn chat_push_char(&mut self, character: char) -> bool {
        let Some(chat) = self.chat.as_mut() else {
            return false;
        };
        if chat.input.chars().count() >= 512 {
            return false;
        }
        chat.input.push(character);
        true
    }

    /// Remove the last character from the composer.
    pub fn chat_backspace(&mut self) -> bool {
        let Some(chat) = self.chat.as_mut() else {
            return false;
        };
        chat.input.pop().is_some()
    }

    /// Take the composer text if it is non-empty, leaving the composer empty.
    pub fn chat_take_input(&mut self) -> Option<String> {
        let chat = self.chat.as_mut()?;
        let prompt = chat.input.trim().to_string();
        chat.input.clear();
        if prompt.is_empty() {
            None
        } else {
            Some(prompt)
        }
    }

    /// Append a user-authored line to the transcript.
    pub fn chat_push_user_line(&mut self, prompt: &str) {
        if !self.chat_open() {
            let _ = self.open_chat();
        }
        self.chat_push_token(&format!("> {prompt}\n"));
        self.chat_finish_stream();
    }
}

#[cfg(test)]
mod tests {
    use aether_geometry::Size;
    use aether_layout::LayoutConfig;

    use super::*;

    fn shell() -> Shell {
        let mut shell = Shell::new(Size::new(1280, 800), LayoutConfig::default(), 120);
        let _ = shell.open_chat();
        shell
    }

    #[test]
    fn composer_takes_trimmed_input_and_clears() {
        let mut shell = shell();
        for character in "  list windows  ".chars() {
            shell.chat_push_char(character);
        }
        assert_eq!(shell.chat_input(), "  list windows  ");
        assert_eq!(shell.chat_take_input().as_deref(), Some("list windows"));
        assert_eq!(shell.chat_input(), "");
        assert_eq!(shell.chat_take_input(), None);
    }

    #[test]
    fn composer_backspace_removes_one_character() {
        let mut shell = shell();
        shell.chat_push_char('a');
        shell.chat_push_char('b');
        assert!(shell.chat_backspace());
        assert_eq!(shell.chat_input(), "a");
        assert!(shell.chat_backspace());
        assert_eq!(shell.chat_input(), "");
        assert!(!shell.chat_backspace());
    }

    #[test]
    fn streaming_pins_the_view_to_the_bottom() {
        let mut shell = shell();
        for index in 0..200 {
            shell.chat_push_token(&format!("line {index}\n"));
        }
        assert!(shell.chat_scroll() > 0);
        assert_eq!(
            shell.chat_scroll() + shell.chat_visible_rows(),
            shell.chat_line_count()
        );
    }

    #[test]
    fn user_lines_mark_the_prompt() {
        let mut shell = shell();
        shell.chat_push_user_line("status?");
        let text: String = shell
            .visible_chat_lines()
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.text.clone()))
            .collect();
        assert!(text.contains("status?"));
    }
}
