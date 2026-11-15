//! JSON-RPC-shaped shell control protocol.
//!
//! The shell binds the socket and handles one request per connection. Callers
//! send [`ControlRequest`] and read a success/error object built by
//! [`success_response`] or [`error_response`]. [`Client`] wraps that exchange
//! for `aether-mcp` and future supervisors.

use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

/// One control call. `id` is echoed by the shell so pipelined clients can match
/// responses; the current transport handles one call per connection.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ControlRequest {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

impl ControlRequest {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            id: json!(1),
            method: method.into(),
            params,
        }
    }

    /// Decode an already-parsed JSON object, applying the shell's defaults.
    pub fn from_value(value: Value) -> Self {
        Self {
            id: value.get("id").cloned().unwrap_or(Value::Null),
            method: value
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            params: value.get("params").cloned().unwrap_or_else(|| json!({})),
        }
    }

    pub fn to_value(&self) -> Value {
        json!({"id": self.id, "method": self.method, "params": self.params})
    }

    pub fn to_line(&self) -> String {
        self.to_value().to_string()
    }
}

/// Build the successful response object.
pub fn success_response(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Build the error response object.
pub fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
}

/// Client for the shell control socket.
#[derive(Clone, Debug)]
pub struct Client {
    path: PathBuf,
}

impl Client {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Read the socket path from `AETHER_CONTROL_SOCKET`.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("AETHER_CONTROL_SOCKET").map(Self::new)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connected(&self) -> bool {
        self.path.exists()
    }

    /// Call one shell method and return its `result` object.
    pub fn call(&self, method: &str, params: &Value) -> Result<Value, String> {
        let stream = UnixStream::connect(&self.path).map_err(|error| {
            format!(
                "compositor control socket {} unavailable: {error}",
                self.path.display()
            )
        })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|error| format!("cannot configure control socket: {error}"))?;
        let mut writer = stream
            .try_clone()
            .map_err(|error| format!("cannot clone control socket: {error}"))?;
        let request = ControlRequest::new(method, params.clone());
        crate::line::write_json_line(&mut writer, &request)
            .map_err(|error| format!("control write failed: {error}"))?;

        let response: Value = crate::line::read_json_line(&mut BufReader::new(stream))
            .map_err(|error| format!("control read failed: {error}"))?
            .ok_or_else(|| "control response is empty".to_string())?;
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("control call failed");
            return Err(message.to_string());
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| "control response has no result".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_decode_with_defaults() {
        let request = ControlRequest::from_value(json!({"id": 7, "method": "windows.list"}));
        assert_eq!(request.id, json!(7));
        assert_eq!(request.method, "windows.list");
        assert_eq!(request.params, json!({}));
    }

    #[test]
    fn requests_round_trip() {
        let request = ControlRequest::new("windows.focus", json!({"id": 3}));
        let value: Value = serde_json::from_str(&request.to_line()).unwrap();
        assert_eq!(ControlRequest::from_value(value), request);
    }

    #[test]
    fn responses_are_jsonrpc_shaped() {
        let success = success_response(json!(3), json!({"focused": 3}));
        assert_eq!(success["jsonrpc"], "2.0");
        assert_eq!(success["result"]["focused"], 3);

        let error = error_response(json!(3), -32000, "failed");
        assert_eq!(error["jsonrpc"], "2.0");
        assert_eq!(error["error"]["code"], -32000);
        assert_eq!(error["error"]["message"], "failed");
    }
}
