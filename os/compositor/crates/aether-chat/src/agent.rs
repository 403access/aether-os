//! The prompt loop: model turn, tool calls, tool results, repeat.

use serde_json::{json, Value};

use crate::mcp::McpClient;
use crate::model::{Model, ToolCall};

pub use aether_protocol::chat::ChatEvent as Event;

pub struct Agent {
    model: Box<dyn Model>,
    mcp: McpClient,
    messages: Vec<Value>,
    max_rounds: usize,
}

impl Agent {
    pub fn new(model: Box<dyn Model>, mcp: McpClient, max_rounds: usize) -> Self {
        Self {
            model,
            mcp,
            messages: vec![json!({
                "role": "system",
                "content": "You are Aether's system assistant. You can inspect and control \
                    this machine through the provided tools. Prefer read tools before actions. \
                    Window close is destructive and needs confirm=true. Keep answers short and \
                    concrete; state what you did when you used a tool.",
            })],
            max_rounds: max_rounds.max(1),
        }
    }

    pub fn prompt(&mut self, prompt: &str, emit: &mut dyn FnMut(Event)) -> Result<(), String> {
        self.messages
            .push(json!({"role": "user", "content": prompt}));
        let tools = self.mcp.tool_defs();
        for _ in 0..self.max_rounds {
            let turn = match self.model.complete(&self.messages, &tools) {
                Ok(turn) => turn,
                Err(error) => {
                    emit(Event::Error(error.clone()));
                    return Err(error);
                }
            };
            if !turn.content.is_empty() {
                for chunk in chunks(&turn.content) {
                    emit(Event::Token(chunk));
                }
            }
            if turn.tool_calls.is_empty() {
                self.messages
                    .push(json!({"role": "assistant", "content": turn.content}));
                emit(Event::Done);
                return Ok(());
            }
            self.messages.push(json!({
                "role": "assistant",
                "content": turn.content,
                "tool_calls": turn.tool_calls.iter().map(tool_call_json).collect::<Vec<_>>(),
            }));
            for call in turn.tool_calls {
                emit(Event::Tool {
                    name: call.name.clone(),
                    status: "start".into(),
                    detail: String::new(),
                });
                match self.mcp.call(&call.name, &call.arguments) {
                    Ok(value) => {
                        let text = serde_json::to_string(&value).unwrap_or_default();
                        emit(Event::Tool {
                            name: call.name.clone(),
                            status: "ok".into(),
                            detail: tool_detail(&call.name, &value),
                        });
                        self.messages.push(json!({
                            "role": "tool",
                            "tool_call_id": call.id,
                            "content": text,
                        }));
                    }
                    Err(error) => {
                        emit(Event::Tool {
                            name: call.name.clone(),
                            status: "error".into(),
                            detail: truncate(&error, 160),
                        });
                        self.messages.push(json!({
                            "role": "tool",
                            "tool_call_id": call.id,
                            "content": json!({"error": error}).to_string(),
                        }));
                    }
                }
            }
        }
        let error = format!("tool loop limit reached ({} rounds)", self.max_rounds);
        emit(Event::Error(error.clone()));
        Err(error)
    }

    /// Bound history so a long session does not grow without limit.
    pub fn trim_history(&mut self) {
        const KEEP: usize = 24;
        if self.messages.len() > KEEP + 1 {
            let tail = self.messages.split_off(self.messages.len() - KEEP);
            self.messages.truncate(1);
            self.messages.extend(tail);
        }
    }
}

fn tool_call_json(call: &ToolCall) -> Value {
    json!({
        "id": call.id,
        "type": "function",
        "function": {
            "name": call.name,
            "arguments": serde_json::to_string(&call.arguments).unwrap_or_else(|_| "{}".into()),
        }
    })
}

fn chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for part in text.split_inclusive(char::is_whitespace) {
        current.push_str(part);
        if current.chars().count() >= 48 || part.ends_with('\n') {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn truncate(text: &str, limit: usize) -> String {
    let cleaned = text.replace('\n', " ");
    if cleaned.chars().count() <= limit {
        cleaned
    } else {
        let mut shortened: String = cleaned.chars().take(limit).collect();
        shortened.push('…');
        shortened
    }
}

/// Human-readable one-line summaries for the common read tools. Everything
/// else falls back to compact JSON with spaces so the drawer can wrap it.
fn tool_detail(name: &str, value: &Value) -> String {
    match name {
        "aether_system" => {
            let platform = value["platform"].as_str().unwrap_or("?");
            let processes = value["process_count"].as_u64().unwrap_or_default();
            let connected = value["compositor"]["connected"].as_bool().unwrap_or(false);
            format!(
                "{platform}, {processes} processes, compositor {}",
                if connected { "connected" } else { "offline" }
            )
        }
        "aether_windows" => format!(
            "{} windows",
            value["windows"]
                .as_array()
                .map(Vec::len)
                .unwrap_or_default()
        ),
        "aether_workspaces" => {
            let active = value["active"].as_u64().unwrap_or_default();
            let total = value["workspaces"]
                .as_array()
                .map(Vec::len)
                .unwrap_or_default();
            format!("workspace {active} of {total}")
        }
        "aether_apps" => format!(
            "{} applications",
            value["count"].as_u64().unwrap_or_default()
        ),
        "aether_files_search" => format!(
            "{} results ({})",
            value["results"]
                .as_array()
                .map(Vec::len)
                .unwrap_or_default(),
            value["source"].as_str().unwrap_or("?")
        ),
        _ => {
            let text = serde_json::to_string(value).unwrap_or_default();
            let spaced = text.replace(',', ", ").replace(':', ": ");
            truncate(&spaced, 120)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_are_bounded_and_lossless() {
        let text = "one two three four five six seven eight nine ten eleven twelve";
        let joined: String = chunks(text).concat();
        assert_eq!(joined, text);
        assert!(chunks(text).iter().all(|chunk| chunk.chars().count() <= 49));
    }

    #[test]
    fn truncation_keeps_the_limit() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5).chars().count(), 6);
    }

    #[test]
    fn system_detail_is_human_readable() {
        let detail = tool_detail(
            "aether_system",
            &json!({
                "platform": "linux",
                "process_count": 120,
                "compositor": {"connected": true},
            }),
        );
        assert_eq!(detail, "linux, 120 processes, compositor connected");
    }
}
