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
        Rect::new(
            bounds.right() as i32 - width as i32 - 16,
            bounds.y + 20,
            width,
            bounds.height.saturating_sub(40),
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
        chat.pending.push_str(token);
        while let Some(newline) = chat.pending.find('\n') {
            let line: String = chat.pending.drain(..=newline).collect();
            let line = line.trim_end_matches(['\n', '\r']);
            chat_materialize(chat, line);
        }
        let max_scroll = chat.lines.len().saturating_sub(chat.visible);
        if chat.scroll + chat.visible >= chat.lines.len().saturating_sub(chat.visible) {
            chat.scroll = max_scroll;
        }
    }

    pub fn chat_finish_stream(&mut self) {
        let Some(chat) = self.chat.as_mut() else {
            return;
        };
        if !chat.pending.is_empty() {
            let line = std::mem::take(&mut chat.pending);
            chat_materialize(chat, &line);
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
}
