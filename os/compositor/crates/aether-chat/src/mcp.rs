//! Minimal MCP client over stdio.
//!
//! The chat backend owns one MCP server child process. It initializes the
//! session, lists tools once and reuses the same process for every prompt.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    tools: Vec<Value>,
}

impl McpClient {
    pub fn connect(command: &PathBuf) -> Result<Self, String> {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", command.display()))?;
        let stdin = child.stdin.take().ok_or("MCP stdin unavailable")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("MCP stdout unavailable")?);
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
            tools: Vec::new(),
        };
        client.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "aether-chat", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        client.notify("notifications/initialized", json!({}))?;
        let listed = client.request("tools/list", json!({}))?;
        client.tools = listed
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(client)
    }

    /// Tools in OpenAI function-calling form.
    pub fn tool_defs(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool["name"],
                        "description": tool["description"],
                        "parameters": tool["inputSchema"],
                    }
                })
            })
            .collect()
    }

    /// Call a tool and return its structured result or error text.
    pub fn call(&mut self, name: &str, arguments: &Value) -> Result<Value, String> {
        let response = self.request("tools/call", json!({"name": name, "arguments": arguments}))?;
        if response.get("isError").and_then(Value::as_bool) == Some(true) {
            let text = response
                .get("content")
                .and_then(Value::as_array)
                .and_then(|content| content.first())
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("tool failed");
            return Err(text.to_string());
        }
        Ok(response
            .get("structuredContent")
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.stdin, "{request}").map_err(|error| format!("MCP write failed: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("MCP flush failed: {error}"))?;
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|error| format!("MCP read failed: {error}"))?;
        if line.trim().is_empty() {
            return Err("MCP server closed the stream".into());
        }
        let response: Value = serde_json::from_str(line.trim())
            .map_err(|error| format!("MCP response is not JSON: {error}"))?;
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("MCP request failed");
            return Err(message.to_string());
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| "MCP response has no result".into())
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = json!({"jsonrpc": "2.0", "method": method, "params": params});
        writeln!(self.stdin, "{notification}")
            .map_err(|error| format!("MCP write failed: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("MCP flush failed: {error}"))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
