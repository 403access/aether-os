//! Model Context Protocol server for Aether OS.
//!
//! The server exposes a small, auditable surface of the running system to a
//! local model or agent: status, processes, files, applications, windows,
//! workspaces, audio and notifications. Read tools are unrestricted; mutating
//! tools are limited to reversible desktop actions, and closing a window
//! requires an explicit `confirm` argument.
//!
//! Transport is newline-delimited JSON-RPC 2.0 over stdio, matching the MCP
//! stdio transport. Every call is appended to an audit log.

#![forbid(unsafe_code)]

pub mod backend;
pub mod control;
pub mod tools;

use serde_json::{json, Value};

pub use backend::{Backend, SystemBackend};
pub use control::ControlClient;

/// MCP revision this server implements. Clients may negotiate down.
pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "aether-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One MCP connection. Owns the backend, so a system service or the built-in
/// chat can keep a single instance for its lifetime.
pub struct Server {
    backend: Box<dyn Backend>,
    initialized: bool,
}

impl Server {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            initialized: false,
        }
    }

    /// Handle one decoded JSON-RPC message. Notifications return `None`.
    pub fn handle(&mut self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

        // Notifications carry no id and must not receive a response.
        if id.is_none() {
            if method == "notifications/initialized" {
                self.initialized = true;
            }
            return None;
        }
        let id = id.unwrap_or(Value::Null);

        let result = match method.as_str() {
            "initialize" => Ok(self.initialize_result(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools::catalog()})),
            "tools/call" => self.call_tool(&params),
            other => Err((-32601, format!("method not found: {other}"))),
        };

        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": code, "message": message},
            }),
        })
    }

    /// Parse and handle one transport line. Blank lines are ignored.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let message = match serde_json::from_str::<Value>(trimmed) {
            Ok(message) => message,
            Err(error) => {
                return Some(
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {"code": -32700, "message": format!("parse error: {error}")},
                    })
                    .to_string(),
                )
            }
        };
        self.handle(message).map(|response| response.to_string())
    }

    fn initialize_result(&self, params: &Value) -> Value {
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(PROTOCOL_VERSION);
        let version = if requested.is_empty() {
            PROTOCOL_VERSION
        } else {
            requested
        };
        json!({
            "protocolVersion": version,
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            },
            "instructions": "Aether OS system control. Prefer read tools; actions are audited. \
                Closing a window requires confirm=true.",
        })
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "missing tool name".to_string()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !tools::exists(name) {
            return Err((-32602, format!("unknown tool: {name}")));
        }
        match self.backend.call(name, &arguments) {
            Ok(value) => {
                let text =
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                Ok(json!({
                    "content": [{"type": "text", "text": text}],
                    "structuredContent": value,
                    "isError": false,
                }))
            }
            Err(message) => Ok(json!({
                "content": [{"type": "text", "text": message}],
                "isError": true,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock;

    impl Backend for Mock {
        fn call(&mut self, tool: &str, arguments: &Value) -> Result<Value, String> {
            if tool == "aether_notify" {
                return Err("expected failure".into());
            }
            Ok(json!({"tool": tool, "arguments": arguments}))
        }
    }

    fn request() -> Server {
        let mut server = Server::new(Box::new(Mock));
        server.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
        server
    }

    #[test]
    fn initialize_negotiates_the_requested_revision() {
        let mut server = Server::new(Box::new(Mock));
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "2025-06-18"},
            }))
            .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(response["result"]["serverInfo"]["name"], "aether-mcp");
    }

    #[test]
    fn notifications_do_not_receive_a_response() {
        let mut server = Server::new(Box::new(Mock));
        assert!(server
            .handle(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .is_none());
    }

    #[test]
    fn tools_list_is_mcp_shaped() {
        let mut server = request();
        let response = server
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|tool| tool["name"] == "aether_system"));
        assert!(tools
            .iter()
            .all(|tool| tool["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn tool_errors_are_content_not_protocol_errors() {
        let mut server = request();
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "aether_notify", "arguments": {}},
            }))
            .unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert_eq!(response["result"]["content"][0]["text"], "expected failure");
    }

    #[test]
    fn parse_errors_are_reported_with_a_null_id() {
        let mut server = Server::new(Box::new(Mock));
        let response = server.handle_line("{").unwrap();
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32700);
        assert!(value["id"].is_null());
    }
}
