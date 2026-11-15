//! Client for the shell control socket.
//!
//! One request per connection keeps the compositor free of per-client state.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct ControlClient {
    path: PathBuf,
}

impl ControlClient {
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
        let request = json!({"id": 1, "method": method, "params": params});
        writeln!(writer, "{request}").map_err(|error| format!("control write failed: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("control flush failed: {error}"))?;

        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .map_err(|error| format!("control read failed: {error}"))?;
        let response: Value = serde_json::from_str(line.trim())
            .map_err(|error| format!("control response is not JSON: {error}"))?;
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
