//! Model backends: an OpenAI-compatible local endpoint or a scripted model.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Turn {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

pub trait Model {
    fn complete(&mut self, messages: &[Value], tools: &[Value]) -> Result<Turn, String>;
}

/// Deterministic model for tests and demos. The script is a JSON file:
///
/// ```json
/// {"turns":[{"content":"checking","tool_calls":[{"name":"aether_system","arguments":{}}]},
///           {"content":"done: {{last_tool}}"}]}
/// ```
pub struct ScriptModel {
    turns: Vec<Value>,
    index: usize,
}

impl ScriptModel {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read script {}: {error}", path.display()))?;
        let script: Value =
            serde_json::from_str(&text).map_err(|error| format!("script is not JSON: {error}"))?;
        let turns = script
            .get("turns")
            .and_then(Value::as_array)
            .cloned()
            .ok_or("script has no turns array")?;
        Ok(Self { turns, index: 0 })
    }
}

impl Model for ScriptModel {
    fn complete(&mut self, messages: &[Value], _tools: &[Value]) -> Result<Turn, String> {
        let Some(turn) = self.turns.get(self.index).cloned() else {
            return Ok(Turn {
                content: "\n_script finished_\n".into(),
                tool_calls: Vec::new(),
            });
        };
        self.index += 1;
        let last_tool = messages
            .iter()
            .rev()
            .find(|message| message["role"] == "tool")
            .and_then(|message| message["content"].as_str())
            .unwrap_or_default()
            .to_string();
        let content = turn
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .replace("{{last_tool}}", &last_tool);
        let tool_calls = turn
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|calls| {
                calls
                    .iter()
                    .enumerate()
                    .map(|(index, call)| ToolCall {
                        id: call
                            .get("id")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("script-{}-{index}", self.index)),
                        name: call
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        arguments: call.get("arguments").cloned().unwrap_or_else(|| json!({})),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Turn {
            content,
            tool_calls,
        })
    }
}

/// OpenAI-compatible chat completions over HTTP or a Unix socket.
pub struct OpenAiModel {
    endpoint: Endpoint,
    model: String,
    api_key: Option<String>,
}

enum Endpoint {
    Tcp {
        host: String,
        port: u16,
        path: String,
    },
    Unix {
        path: PathBuf,
        request_path: String,
    },
}

impl OpenAiModel {
    pub fn new(endpoint: &str, model: &str, api_key: Option<String>) -> Result<Self, String> {
        Ok(Self {
            endpoint: Endpoint::parse(endpoint)?,
            model: model.to_string(),
            api_key,
        })
    }
}

impl Endpoint {
    fn parse(raw: &str) -> Result<Self, String> {
        if let Some(path) = raw
            .strip_prefix("unix://")
            .or_else(|| raw.strip_prefix("unix:"))
        {
            let path = PathBuf::from(path);
            return Ok(Self::Unix {
                path,
                request_path: "/v1/chat/completions".into(),
            });
        }
        let rest = raw
            .strip_prefix("http://")
            .ok_or_else(|| format!("endpoint must start with http:// or unix:: {raw}"))?;
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/".to_string()),
        };
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (
                host.to_string(),
                port.parse::<u16>()
                    .map_err(|error| format!("invalid port: {error}"))?,
            ),
            None => (authority.to_string(), 80),
        };
        let path = if path.ends_with("/chat/completions") {
            path
        } else {
            format!("{}/chat/completions", path.trim_end_matches('/'))
        };
        Ok(Self::Tcp { host, port, path })
    }
}

impl Model for OpenAiModel {
    fn complete(&mut self, messages: &[Value], tools: &[Value]) -> Result<Turn, String> {
        let body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "stream": false,
        })
        .to_string();
        let response = self.post(&body)?;
        let value: Value = serde_json::from_str(&response)
            .map_err(|error| format!("model response is not JSON: {error}"))?;
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("model error");
            return Err(message.to_string());
        }
        let message = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .ok_or("model response has no message")?;
        let content = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let tool_calls = message
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|calls| {
                calls
                    .iter()
                    .enumerate()
                    .filter_map(|(index, call)| {
                        let function = call.get("function")?;
                        Some(ToolCall {
                            id: call
                                .get("id")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                                .unwrap_or_else(|| format!("call-{index}")),
                            name: function.get("name")?.as_str()?.to_string(),
                            arguments: match function.get("arguments") {
                                Some(Value::String(text)) => {
                                    serde_json::from_str(text).unwrap_or_else(|_| json!({}))
                                }
                                Some(value) => value.clone(),
                                None => json!({}),
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Turn {
            content,
            tool_calls,
        })
    }
}

impl OpenAiModel {
    fn post(&self, body: &str) -> Result<String, String> {
        let mut headers = format!(
            "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n",
            match &self.endpoint {
                Endpoint::Tcp { path, .. } => path,
                Endpoint::Unix { request_path, .. } => request_path,
            },
            body.len()
        );
        if let Some(key) = &self.api_key {
            headers.push_str(&format!("Authorization: Bearer {key}\r\n"));
        }
        headers.push_str("\r\n");
        let request = format!("{headers}{body}");

        let mut stream: Box<dyn ReadWrite> = match &self.endpoint {
            Endpoint::Tcp { host, port, .. } => Box::new(
                TcpStream::connect((host.as_str(), *port))
                    .map_err(|error| format!("cannot reach {host}:{port}: {error}"))?,
            ),
            Endpoint::Unix { path, .. } => Box::new(
                UnixStream::connect(path)
                    .map_err(|error| format!("cannot reach {}: {error}", path.display()))?,
            ),
        };
        stream
            .set_timeout(Some(Duration::from_secs(120)))
            .map_err(|error| format!("cannot set model timeout: {error}"))?;
        stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("model write failed: {error}"))?;
        stream
            .flush()
            .map_err(|error| format!("model flush failed: {error}"))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| format!("model read failed: {error}"))?;
        decode_http(&response)
    }
}

trait ReadWrite: Read + Write {
    fn set_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
}

impl ReadWrite for TcpStream {
    fn set_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)
    }
}

impl ReadWrite for UnixStream {
    fn set_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)
    }
}

fn decode_http(response: &[u8]) -> Result<String, String> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("model response has no header terminator")?;
    let head = String::from_utf8_lossy(&response[..split]);
    let body = &response[split + 4..];
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let chunked = head.lines().any(|line| {
        line.to_lowercase().starts_with("transfer-encoding:")
            && line.to_lowercase().contains("chunked")
    });
    let body = if chunked {
        decode_chunked(body)?
    } else {
        body.to_vec()
    };
    let text = String::from_utf8_lossy(&body).into_owned();
    if !(200..300).contains(&status) {
        return Err(format!("model returned HTTP {status}: {text}"));
    }
    Ok(text)
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let mut rest = body;
    loop {
        let line_end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("chunked body is truncated")?;
        let size_text = String::from_utf8_lossy(&rest[..line_end]);
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|error| format!("invalid chunk size: {error}"))?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Ok(result);
        }
        if rest.len() < size + 2 {
            return Err("chunked body is truncated".into());
        }
        result.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoints() {
        let Endpoint::Tcp { host, port, path } =
            Endpoint::parse("http://127.0.0.1:11434/v1").unwrap()
        else {
            panic!("expected TCP");
        };
        assert_eq!(
            (host.as_str(), port, path.as_str()),
            ("127.0.0.1", 11434, "/v1/chat/completions")
        );

        let endpoint = Endpoint::parse("unix:///run/aether/inference.sock").unwrap();
        assert!(
            matches!(endpoint, Endpoint::Unix { request_path, .. } if request_path == "/v1/chat/completions")
        );
    }

    #[test]
    fn decodes_chunked_bodies() {
        let body = b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(body).unwrap(), b"hello world");
    }

    #[test]
    fn script_model_substitutes_the_last_tool_result() {
        let path =
            std::env::temp_dir().join(format!("aether-chat-script-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"turns":[{"content":"result: {{last_tool}}"}]}"#).unwrap();
        let mut model = ScriptModel::from_file(&path).unwrap();
        let turn = model
            .complete(&[json!({"role":"tool","content":"ok"})], &[])
            .unwrap();
        assert_eq!(turn.content, "result: ok");
        let _ = std::fs::remove_file(path);
    }
}
