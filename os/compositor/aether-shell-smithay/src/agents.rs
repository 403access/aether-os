//! Live agent-session launcher IPC.
//!
//! A supervisor connects to `AETHER_AGENT_SOCKET` and writes newline-delimited
//! JSON snapshots. Each snapshot is an array of launcher objects (or an object
//! with a `launchers` array). A snapshot replaces the previous agent entries;
//! disconnecting the last publisher clears them.

use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use aether_shell::PaletteLauncher;

pub struct AgentSessions {
    listener: Option<UnixListener>,
    streams: Vec<AgentStream>,
    launchers: Vec<PaletteLauncher>,
}

struct AgentStream {
    stream: UnixStream,
    buffer: String,
}

impl AgentSessions {
    pub fn bind(path: Option<PathBuf>) -> Self {
        let listener = path.and_then(|path| {
            let _ = std::fs::remove_file(&path);
            let listener = UnixListener::bind(&path).ok()?;
            listener.set_nonblocking(true).ok()?;
            println!("aether-shell-smithay: agent socket {}", path.display());
            Some(listener)
        });
        Self {
            listener,
            streams: Vec::new(),
            launchers: Vec::new(),
        }
    }

    pub fn launchers(&self) -> &[PaletteLauncher] {
        &self.launchers
    }

    /// Accept publishers and process complete snapshots. Returns true when the
    /// launcher list changed.
    pub fn poll(&mut self) -> bool {
        if let Some(listener) = &self.listener {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if stream.set_nonblocking(true).is_ok() {
                            self.streams.push(AgentStream {
                                stream,
                                buffer: String::new(),
                            });
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        let mut changed = false;
        let mut disconnected = Vec::new();
        for (index, agent) in self.streams.iter_mut().enumerate() {
            let mut chunk = [0u8; 8192];
            loop {
                match agent.stream.read(&mut chunk) {
                    Ok(0) => {
                        disconnected.push(index);
                        break;
                    }
                    Ok(read) => {
                        agent
                            .buffer
                            .push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        disconnected.push(index);
                        break;
                    }
                }
            }
            while let Some(newline) = agent.buffer.find('\n') {
                let line = agent.buffer[..newline].to_string();
                agent.buffer.drain(..=newline);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    self.launchers = parse_launchers(&value);
                    changed = true;
                }
            }
        }

        if !disconnected.is_empty() {
            disconnected.sort_unstable();
            for index in disconnected.into_iter().rev() {
                self.streams.remove(index);
            }
            if self.streams.is_empty() {
                self.launchers.clear();
                changed = true;
            }
        }
        changed
    }
}

/// Parse an array or `{ "launchers": [...] }` snapshot.
pub fn parse_launchers(value: &serde_json::Value) -> Vec<PaletteLauncher> {
    let entries = value
        .as_array()
        .or_else(|| value.get("launchers").and_then(|value| value.as_array()));
    let Some(entries) = entries else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let name = entry.get("name")?.as_str()?.to_string();
            let program = entry.get("program")?.as_str()?.to_string();
            let detail = entry
                .get("detail")
                .and_then(|value| value.as_str())
                .unwrap_or("agent session")
                .to_string();
            let icon = entry
                .get("icon")
                .and_then(|value| value.as_str())
                .map(str::to_string);
            let args = entry
                .get("args")
                .and_then(|value| value.as_array())
                .map(|args| {
                    args.iter()
                        .filter_map(|arg| arg.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(PaletteLauncher {
                name,
                detail,
                icon,
                program,
                args,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snapshots_and_skips_bad_entries() {
        let value = serde_json::json!([
            {"name":"Build agent","detail":"running","program":"agentctl","args":["attach","1"]},
            {"name":"Missing program"}
        ]);
        let launchers = parse_launchers(&value);
        assert_eq!(launchers.len(), 1);
        assert_eq!(launchers[0].name, "Build agent");
        assert_eq!(launchers[0].args, ["attach", "1"]);
        assert!(parse_launchers(&serde_json::json!({"nope":true})).is_empty());
    }
}
