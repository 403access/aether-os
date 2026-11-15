//! Chat transcript model and markdown parsing.

/// One styled span inside a chat row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatSpan {
    pub text: String,
    pub kind: ChatSpanKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatSpanKind {
    Text,
    Bold,
    Code,
}

/// One rendered chat row. Markdown is parsed as tokens arrive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    pub spans: Vec<ChatSpan>,
    pub scale: u32,
    pub indent: i32,
    pub code: bool,
}

#[derive(Clone, Debug)]
pub struct ChatState {
    pub(crate) lines: Vec<ChatLine>,
    pub(crate) pending: String,
    pub(crate) scroll: usize,
    pub(crate) visible: usize,
    pub(crate) code_block: bool,
    pub(crate) width: usize,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            pending: String::new(),
            scroll: 0,
            visible: 20,
            code_block: false,
            width: 44,
        }
    }
}

pub(crate) fn chat_materialize(chat: &mut ChatState, line: &str) {
    let trimmed = line.trim_end();
    if trimmed.trim_start().starts_with("```") {
        chat.code_block = !chat.code_block;
        return;
    }
    let mut scale = 1;
    let mut indent = 0;
    let mut content = trimmed;
    if chat.code_block {
        let wrapped = chat_wrap(content, chat.width, 8);
        for text in wrapped {
            chat.lines.push(ChatLine {
                spans: vec![ChatSpan {
                    text,
                    kind: ChatSpanKind::Code,
                }],
                scale,
                indent: 8,
                code: true,
            });
        }
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("# ") {
        scale = 2;
        content = rest;
    } else if let Some(rest) = trimmed.strip_prefix("## ") {
        scale = 2;
        content = rest;
    } else if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        indent = 4;
        content = rest;
    }
    let mut spans = Vec::new();
    if indent > 0 {
        spans.push(ChatSpan {
            text: "• ".to_string(),
            kind: ChatSpanKind::Bold,
        });
    }
    spans.extend(chat_inline(content));
    if spans.is_empty() {
        chat.lines.push(ChatLine {
            spans: Vec::new(),
            scale,
            indent,
            code: false,
        });
        return;
    }
    let text: String = spans.iter().map(|span| span.text.as_str()).collect();
    if text.chars().count() <= chat.width {
        chat.lines.push(ChatLine {
            spans,
            scale,
            indent,
            code: false,
        });
        return;
    }
    // Word-aware wrap, preserving span styling across rows.
    let rows = wrap_chat_spans(spans, chat.width);
    for (index, row) in rows.into_iter().enumerate() {
        chat.lines.push(ChatLine {
            spans: row,
            scale,
            indent: indent + if index > 0 && indent > 0 { 2 } else { 0 },
            code: false,
        });
    }
    if chat.lines.len() > 4000 {
        let excess = chat.lines.len() - 3000;
        chat.lines.drain(..excess);
        chat.scroll = chat.scroll.saturating_sub(excess);
    }
}

fn wrap_chat_spans(spans: Vec<ChatSpan>, width: usize) -> Vec<Vec<ChatSpan>> {
    let mut rows = Vec::new();
    let mut row: Vec<ChatSpan> = Vec::new();
    let mut row_width = 0usize;
    let mut push_chunk =
        |row: &mut Vec<ChatSpan>, row_width: &mut usize, text: &str, kind: ChatSpanKind| {
            if text.is_empty() {
                return;
            }
            let length = text.chars().count();
            if *row_width > 0 && *row_width + length > width {
                rows.push(std::mem::take(row));
                *row_width = 0;
            }
            let text = if *row_width == 0 {
                text.trim_start().to_string()
            } else {
                text.to_string()
            };
            if text.is_empty() {
                return;
            }
            if let Some(last) = row.last_mut() {
                if last.kind == kind {
                    last.text.push_str(&text);
                    *row_width += text.chars().count();
                    return;
                }
            }
            *row_width += text.chars().count();
            row.push(ChatSpan { text, kind });
        };
    for span in spans {
        for chunk in span
            .text
            .split_inclusive(|character: char| character.is_whitespace())
        {
            push_chunk(&mut row, &mut row_width, chunk, span.kind);
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    rows
}

fn chat_wrap(text: &str, width: usize, indent: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        if !row.is_empty() && row.chars().count() + 1 + word.chars().count() > width {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(" ".repeat(indent.min(1)));
    }
    rows
}

fn chat_inline(text: &str) -> Vec<ChatSpan> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let push = |spans: &mut Vec<ChatSpan>, current: &mut String, kind: ChatSpanKind| {
        if !current.is_empty() {
            spans.push(ChatSpan {
                text: std::mem::take(current),
                kind,
            });
        }
    };
    while let Some(character) = chars.next() {
        if character == '*' && chars.peek() == Some(&'*') {
            chars.next();
            push(&mut spans, &mut current, ChatSpanKind::Text);
            let mut bold = String::new();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'*') {
                    chars.next();
                    break;
                }
                bold.push(next);
            }
            if !bold.is_empty() {
                spans.push(ChatSpan {
                    text: bold,
                    kind: ChatSpanKind::Bold,
                });
            }
        } else if character == '`' {
            push(&mut spans, &mut current, ChatSpanKind::Text);
            let mut code = String::new();
            for next in chars.by_ref() {
                if next == '`' {
                    break;
                }
                code.push(next);
            }
            if !code.is_empty() {
                spans.push(ChatSpan {
                    text: code,
                    kind: ChatSpanKind::Code,
                });
            }
        } else {
            current.push(character);
        }
    }
    push(&mut spans, &mut current, ChatSpanKind::Text);
    spans
}
