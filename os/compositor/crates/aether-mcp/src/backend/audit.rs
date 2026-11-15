//! Append-only audit log for tool calls.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Audit {
    path: PathBuf,
}

impl Audit {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn record(
        &mut self,
        tool: &str,
        arguments: &Value,
        outcome: &Result<Value, String>,
    ) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        if fs::metadata(&self.path)
            .map(|metadata| metadata.len() > 1_048_576)
            .unwrap_or(false)
        {
            let rotated = self.path.with_extension("jsonl.1");
            let _ = fs::rename(&self.path, rotated);
        }
        let entry = json!({
            "ts": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default(),
            "tool": tool,
            "arguments": summarize(arguments),
            "ok": outcome.is_ok(),
            "error": match outcome {
                Ok(_) => None,
                Err(error) => Some(error.as_str()),
            },
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{entry}")
    }
}

/// Keep audit entries small and avoid persisting prompt or notification text.
fn summarize(arguments: &Value) -> Value {
    let mut summarized = serde_json::Map::new();
    if let Some(object) = arguments.as_object() {
        for (key, value) in object {
            if matches!(key.as_str(), "prompt" | "body") {
                summarized.insert(
                    key.clone(),
                    json!({"redacted": value.as_str().map(|text| text.chars().count())}),
                );
            } else {
                summarized.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(summarized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_redacted_but_shape_is_kept() {
        let summary = summarize(&json!({"prompt": "secret", "surface": "chat"}));
        assert_eq!(summary["prompt"]["redacted"], 6);
        assert!(summary["prompt"].get("prompt").is_none());
        assert_eq!(summary["surface"], "chat");
    }
}
