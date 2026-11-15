//! Shell control socket for `aether-mcp` and other local supervisors.
//!
//! A supervisor connects to `AETHER_CONTROL_SOCKET` and sends one
//! newline-delimited JSON request per connection:
//!
//! ```text
//! {"id":1,"method":"windows.list","params":{}}
//! ```
//!
//! The shell replies with a single JSON-RPC-shaped line and closes the stream.
//! Requests are parsed on a background thread and executed on the compositor
//! thread, because window, workspace and launch actions must not race the frame
//! loop. Parsing never blocks presentation.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use aether_protocol::control::{error_response, success_response, ControlRequest};
use aether_shell::PaletteLauncher;
use serde_json::{json, Value};

use super::App;

/// Maximum requests executed in one frame. A busy client cannot starve input.
const CONTROL_BUDGET: usize = 8;

struct ControlCall {
    request: Value,
    stream: UnixStream,
}

pub(crate) struct ControlServer {
    calls: Receiver<ControlCall>,
    path: PathBuf,
}

impl ControlServer {
    pub(crate) fn bind(path: Option<PathBuf>) -> Option<Self> {
        let path = path?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!(
                    "aether-shell: control socket {} unavailable: {error}",
                    path.display()
                );
                return None;
            }
        };
        let (tx, calls) = mpsc::channel();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let Ok(mut writer) = stream.try_clone() else {
                    continue;
                };
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                match serde_json::from_str::<Value>(&line) {
                    Ok(request) => {
                        if tx
                            .send(ControlCall {
                                request,
                                stream: writer,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = writeln!(
                            writer,
                            "{}",
                            error_response(Value::Null, -32700, &format!("parse error: {error}"))
                        );
                    }
                }
            }
        });
        println!("aether-shell: control socket {}", path.display());
        Some(Self { calls, path })
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl App {
    /// Execute up to [`CONTROL_BUDGET`] queued control requests.
    pub(crate) fn pump_control(&mut self) {
        let mut calls = Vec::new();
        for _ in 0..CONTROL_BUDGET {
            let Some(server) = self.control.as_ref() else {
                return;
            };
            match server.calls.try_recv() {
                Ok(call) => calls.push(call),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        for mut call in calls {
            let request = ControlRequest::from_value(call.request);
            let id = request.id.clone();
            let response = match self.handle_control(&request.method, &request.params) {
                Ok(result) => success_response(id, result),
                Err(error) => error_response(id, -32000, &error),
            };
            let _ = writeln!(call.stream, "{response}");
            let _ = call.stream.flush();
        }
    }

    fn handle_control(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        match method {
            "status" => Ok(self.control_status()),
            "windows.list" => Ok(json!({"windows": self.window_json()})),
            "windows.focus" => {
                let id = param_u64(params, "id")?;
                self.focus_window_by_id(id)?;
                Ok(json!({"focused": id}))
            }
            "windows.close" => {
                let id = param_u64(params, "id")?;
                let Some(toplevel) = self.windows.get(&id) else {
                    return Err(format!("no window {id}"));
                };
                toplevel.send_close();
                println!("aether-shell: control closed window {id}");
                Ok(json!({"closed": id}))
            }
            "workspaces.list" => Ok(json!({
                "active": self.shell.active_workspace(),
                "workspaces": self.shell.workspace_names(),
            })),
            "workspaces.switch" => {
                let index = param_u64(params, "index")? as usize;
                if index >= self.shell.workspace_count() {
                    return Err(format!(
                        "workspace {index} out of range (0..{})",
                        self.shell.workspace_count()
                    ));
                }
                self.switch_workspace(index);
                Ok(json!({"active": index}))
            }
            "apps.list" => {
                let query = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_lowercase();
                let apps: Vec<Value> = self
                    .shell
                    .launchers()
                    .iter()
                    .filter(|launcher| {
                        query.is_empty()
                            || launcher.name.to_lowercase().contains(&query)
                            || launcher.program.to_lowercase().contains(&query)
                    })
                    .map(app_json)
                    .collect();
                Ok(json!({"apps": apps}))
            }
            "apps.launch" => {
                let name = params
                    .get("name")
                    .or_else(|| params.get("app"))
                    .and_then(Value::as_str)
                    .ok_or("missing app name")?;
                let launcher = self
                    .shell
                    .launchers()
                    .iter()
                    .find(|launcher| {
                        launcher.name.eq_ignore_ascii_case(name)
                            || launcher.program.eq_ignore_ascii_case(name)
                    })
                    .cloned();
                let Some(launcher) = launcher else {
                    return Err(format!("no application named {name:?}"));
                };
                self.shell.close_overlay();
                self.spawn_launcher(launcher.clone());
                Ok(json!({"launched": launcher.name, "program": launcher.program}))
            }
            "surface.open" => {
                let surface = params
                    .get("surface")
                    .and_then(Value::as_str)
                    .ok_or("missing surface")?;
                self.open_surface(surface)?;
                Ok(json!({"surface": surface}))
            }
            "chat.prompt" => {
                let prompt = params
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or("missing prompt")?;
                self.shell.chat_push_user_line(prompt);
                let sent = self.chat_bridge.send_prompt(prompt);
                self.mark_chat_damage();
                if !sent {
                    self.shell
                        .chat_push_token("\n_chat backend is not connected_\n\n");
                    self.shell.chat_finish_stream();
                    self.mark_chat_damage();
                }
                Ok(json!({"queued": sent}))
            }
            other => Err(format!("unknown method {other:?}")),
        }
    }

    fn control_status(&self) -> Value {
        json!({
            "windows": self.shell.window_count(),
            "workspace": self.shell.active_workspace(),
            "workspaces": self.shell.workspace_names(),
            "apps": self.shell.launchers().len(),
            "chat_open": self.shell.chat_open(),
            "palette_open": self.shell.overlay().is_some(),
            "socket": self.socket_name,
        })
    }

    fn window_json(&self) -> Vec<Value> {
        self.shell
            .windows()
            .into_iter()
            .map(|window| {
                json!({
                    "id": window.id,
                    "title": window.title,
                    "app_id": window.app_id,
                    "workspace": window.workspace,
                    "output": window.output,
                    "floating": window.floating,
                    "maximized": window.maximized,
                    "fullscreen": window.fullscreen,
                    "focused": window.focused,
                    "rect": {
                        "x": window.rect.x,
                        "y": window.rect.y,
                        "width": window.rect.width,
                        "height": window.rect.height,
                    },
                })
            })
            .collect()
    }

    fn focus_window_by_id(&mut self, id: u64) -> Result<(), String> {
        let target = self
            .shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| (window.output, window.workspace))
            .ok_or_else(|| format!("no window {id}"))?;
        let (output, workspace) = target;
        if self.shell.output_workspace(output) != workspace {
            let primary = self.shell.primary_output().map(|output| output.id);
            if Some(output) == primary {
                self.switch_workspace(workspace);
            } else {
                self.shell.set_output_workspace(output, workspace);
            }
        }
        if !self.shell.focus_window(id) {
            return Err(format!("window {id} cannot be focused"));
        }
        self.sync_focus();
        self.configure_all();
        self.mark_full_damage();
        Ok(())
    }

    fn open_surface(&mut self, surface: &str) -> Result<(), String> {
        match surface {
            "palette" if self.shell.overlay().is_none() => self.toggle_overlay(),
            "chat" if !self.shell.chat_open() => self.toggle_chat(),
            "explorer" if !self.shell.explorer_open() => self.toggle_explorer(),
            "audio" if !self.shell.audio_open() => self.toggle_audio(),
            "task_manager" if !self.task_manager_open() => self.open_task_manager(),
            "shortcuts" if !self.shortcuts_open => self.toggle_shortcuts(),
            "palette" | "chat" | "explorer" | "audio" | "task_manager" | "shortcuts" => {}
            other => return Err(format!("unknown surface {other:?}")),
        }
        Ok(())
    }
}

fn app_json(launcher: &PaletteLauncher) -> Value {
    json!({
        "name": launcher.name,
        "detail": launcher.detail,
        "program": launcher.program,
        "args": launcher.args,
    })
}

fn param_u64(params: &Value, key: &str) -> Result<u64, String> {
    params
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing numeric {key}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn param_u64_reports_the_missing_key() {
        assert_eq!(
            param_u64(&json!({}), "id"),
            Err("missing numeric id".to_string())
        );
        assert_eq!(param_u64(&json!({"id": 7}), "id"), Ok(7));
    }

    #[test]
    fn error_response_is_jsonrpc_shaped() {
        let response = error_response(json!(3), -32000, "failed");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 3);
        assert_eq!(response["error"]["code"], -32000);
        assert_eq!(response["error"]["message"], "failed");
    }
}
