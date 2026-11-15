//! Chat prompt and event stream shared by the shell drawer and `aether-chat`.
//!
//! The shell sends one [`prompt_line`] and receives one [`ChatEvent`] per line.
//! `aether-chat --stdio` also accepts a bare prompt line, so
//! [`parse_prompt_line`] keeps the JSON envelope optional for that mode.

use serde_json::{json, Value};

/// One streamed event from the chat backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatEvent {
    Token(String),
    Tool {
        name: String,
        status: String,
        detail: String,
    },
    Done,
    Error(String),
}

impl ChatEvent {
    /// Encode the event as the JSON object sent on the wire.
    pub fn to_value(&self) -> Value {
        match self {
            Self::Token(text) => json!({"type": "token", "text": text}),
            Self::Tool {
                name,
                status,
                detail,
            } => json!({
                "type": "tool",
                "name": name,
                "status": status,
                "detail": detail,
            }),
            Self::Done => json!({"type": "done"}),
            Self::Error(message) => json!({"type": "error", "message": message}),
        }
    }

    /// Encode without a trailing newline.
    pub fn to_line(&self) -> String {
        self.to_value().to_string()
    }

    /// Decode an event object. Unknown types and missing fields yield `None`.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value.get("type").and_then(Value::as_str)? {
            "token" => Some(Self::Token(
                value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            )),
            "tool" => Some(Self::Tool {
                name: value
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("tool")
                    .to_string(),
                status: value
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("start")
                    .to_string(),
                detail: value
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }),
            "done" => Some(Self::Done),
            "error" => Some(Self::Error(
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("chat backend error")
                    .to_string(),
            )),
            _ => None,
        }
    }

    /// Decode one transport line. Blank and malformed lines are ignored.
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let value = serde_json::from_str::<Value>(trimmed).ok()?;
        Self::from_value(&value)
    }
}

/// Encode a prompt as the JSON envelope the shell sends.
pub fn prompt_line(prompt: &str) -> String {
    json!({"type": "prompt", "prompt": prompt}).to_string()
}

/// Decode a prompt line.
///
/// A bare, non-JSON line is treated as the prompt itself so `aether-chat
/// --stdio` can be driven by any line-oriented tool. A JSON line must use the
/// `{"type":"prompt"}` envelope; other event types are ignored.
pub fn parse_prompt_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if value.get("type").and_then(Value::as_str) == Some("prompt") {
            return value
                .get("prompt")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_events_round_trip() {
        let events = [
            ChatEvent::Token("hello".into()),
            ChatEvent::Tool {
                name: "aether_status".into(),
                status: "ok".into(),
                detail: "3 windows".into(),
            },
            ChatEvent::Done,
            ChatEvent::Error("offline".into()),
        ];
        for event in events {
            let line = event.to_line();
            assert_eq!(ChatEvent::parse_line(&line), Some(event));
        }
    }

    #[test]
    fn unknown_and_blank_events_are_ignored() {
        assert_eq!(ChatEvent::parse_line(""), None);
        assert_eq!(ChatEvent::parse_line("not json"), None);
        assert_eq!(ChatEvent::parse_line(r#"{"type":"unknown"}"#), None);
    }

    #[test]
    fn prompts_accept_the_envelope_or_a_raw_line() {
        assert_eq!(
            parse_prompt_line(r#"{"type":"prompt","prompt":"hi"}"#),
            Some("hi".into())
        );
        assert_eq!(parse_prompt_line("  hi there  "), Some("hi there".into()));
        assert_eq!(parse_prompt_line(r#"{"type":"done"}"#), None);
        assert_eq!(parse_prompt_line("   "), None);
        assert_eq!(
            parse_prompt_line(&prompt_line("hi")),
            Some("hi".to_string())
        );
    }
}
