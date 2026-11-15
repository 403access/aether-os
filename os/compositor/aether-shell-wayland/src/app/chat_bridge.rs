//! Chat backend bridge.
//!
//! `aether-chat` is a separate process: it owns the model loop and the MCP
//! client, while the drawer stays a thin, non-blocking view. The shell binds
//! `AETHER_CHAT_SOCKET`; the backend connects and exchanges newline-delimited
//! JSON in both directions:
//!
//! ```text
//! shell -> backend   {"type":"prompt","prompt":"what is using RAM?"}
//! backend -> shell   {"type":"token","text":"..."}
//!                    {"type":"tool","name":"...","status":"start|ok|error"}
//!                    {"type":"done"}
//!                    {"type":"error","message":"..."}
//! ```
//!
//! Reads are non-blocking. A slow or absent backend never stalls a frame.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use aether_protocol::chat::{self, ChatEvent};

pub(crate) struct ChatBridge {
    listener: Option<UnixListener>,
    client: Option<UnixStream>,
    buffer: String,
    path: Option<PathBuf>,
}

impl ChatBridge {
    pub(crate) fn bind(path: Option<PathBuf>) -> Self {
        let mut bridge = Self {
            listener: None,
            client: None,
            buffer: String::new(),
            path: None,
        };
        let Some(path) = path else {
            return bridge;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&path);
        match UnixListener::bind(&path) {
            Ok(listener) => {
                let _ = listener.set_nonblocking(true);
                println!("aether-shell: chat backend socket {}", path.display());
                bridge.listener = Some(listener);
                bridge.path = Some(path);
            }
            Err(error) => {
                eprintln!(
                    "aether-shell: chat socket {} unavailable: {error}",
                    path.display()
                );
            }
        }
        bridge
    }

    pub(crate) fn connected(&self) -> bool {
        self.client.is_some()
    }

    /// Accept a backend and buffer complete events. Called once per frame.
    pub(crate) fn poll(&mut self) -> Vec<ChatEvent> {
        if self.client.is_none() {
            if let Some(listener) = self.listener.as_ref() {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(true);
                        println!("aether-shell: chat backend connected");
                        self.client = Some(stream);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => eprintln!("aether-shell: chat accept failed: {error}"),
                }
            }
        }
        let Some(client) = self.client.as_mut() else {
            return Vec::new();
        };
        let mut chunk = [0u8; 4096];
        loop {
            match client.read(&mut chunk) {
                Ok(0) => {
                    println!("aether-shell: chat backend disconnected");
                    self.client = None;
                    self.buffer.clear();
                    break;
                }
                Ok(read) => {
                    self.buffer
                        .push_str(&String::from_utf8_lossy(&chunk[..read]));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    eprintln!("aether-shell: chat read failed: {error}");
                    self.client = None;
                    break;
                }
            }
        }
        let mut events = Vec::new();
        while let Some(newline) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=newline).collect();
            if let Some(event) = parse_event(line.trim()) {
                events.push(event);
            }
        }
        events
    }

    /// Send a prompt. Returns false when no backend is connected or the write
    /// would block; the caller should surface that to the user.
    pub(crate) fn send_prompt(&mut self, prompt: &str) -> bool {
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        let line = chat::prompt_line(prompt);
        match client.write_all(format!("{line}\n").as_bytes()) {
            Ok(()) => client.flush().is_ok(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => false,
            Err(error) => {
                eprintln!("aether-shell: chat write failed: {error}");
                self.client = None;
                false
            }
        }
    }
}

impl Drop for ChatBridge {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn parse_event(line: &str) -> Option<ChatEvent> {
    ChatEvent::parse_line(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tokens_and_tool_status() {
        assert_eq!(
            parse_event(r#"{"type":"token","text":"hello"}"#),
            Some(ChatEvent::Token("hello".into()))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"tool","name":"aether_status","status":"ok","detail":"3 windows"}"#
            ),
            Some(ChatEvent::Tool {
                name: "aether_status".into(),
                status: "ok".into(),
                detail: "3 windows".into(),
            })
        );
        assert_eq!(parse_event(r#"{"type":"done"}"#), Some(ChatEvent::Done));
        assert_eq!(parse_event("not json"), None);
        assert_eq!(parse_event(r#"{"type":"unknown"}"#), None);
    }
}
