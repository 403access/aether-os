//! Backend dispatch for the Aether MCP tools.

mod apps;
mod audio;
mod audit;
mod files;
mod probe;

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::control::ControlClient;

pub use audit::Audit;

/// Something that can execute a tool call. The MCP server is transport-only;
/// this trait is what tests replace.
pub trait Backend {
    fn call(&mut self, tool: &str, arguments: &Value) -> Result<Value, String>;
}

#[derive(Clone, Debug)]
pub struct Config {
    pub control: Option<PathBuf>,
    pub index: Option<PathBuf>,
    pub search_roots: Vec<PathBuf>,
    pub audit: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Self {
        let control = std::env::var_os("AETHER_CONTROL_SOCKET")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_RUNTIME_DIR")
                    .map(|dir| PathBuf::from(dir).join("aether-control.sock"))
            });
        let index = std::env::var_os("AETHER_INDEX_SOCKET")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_RUNTIME_DIR")
                    .map(|dir| PathBuf::from(dir).join("aether-index.sock"))
            });
        let search_roots = std::env::var_os("AETHER_MCP_SEARCH_ROOTS")
            .map(|roots| std::env::split_paths(&roots).collect())
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .into_iter()
                    .collect()
            });
        let audit = std::env::var_os("AETHER_MCP_AUDIT")
            .map(PathBuf::from)
            .or_else(default_audit_path);
        Self {
            control,
            index,
            search_roots,
            audit,
        }
    }
}

fn default_audit_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
        })?;
    Some(base.join("aether/mcp-audit.jsonl"))
}

pub struct SystemBackend {
    config: Config,
    control: Option<ControlClient>,
    audit: Option<Audit>,
}

impl SystemBackend {
    pub fn from_env() -> Self {
        Self::with_config(Config::from_env())
    }

    pub fn with_config(config: Config) -> Self {
        let control = config.control.clone().map(ControlClient::new);
        let audit = config.audit.clone().map(Audit::new);
        Self {
            config,
            control,
            audit,
        }
    }

    fn control(&self) -> Result<&ControlClient, String> {
        self.control.as_ref().ok_or_else(|| {
            "compositor is not reachable; start the Aether session or set \
             AETHER_CONTROL_SOCKET"
                .to_string()
        })
    }

    fn record(&mut self, tool: &str, arguments: &Value, outcome: &Result<Value, String>) {
        let Some(audit) = self.audit.as_mut() else {
            return;
        };
        let _ = audit.record(tool, arguments, outcome);
    }

    fn dispatch(&mut self, tool: &str, arguments: &Value) -> Result<Value, String> {
        match tool {
            "aether_system" => probe::status(self.control.as_ref()),
            "aether_processes" => probe::processes(
                arg_u64(arguments, "limit").unwrap_or(20).clamp(1, 500) as usize,
                arguments
                    .get("sort")
                    .and_then(Value::as_str)
                    .unwrap_or("cpu"),
                arguments.get("query").and_then(Value::as_str),
            ),
            "aether_files_search" => {
                let query = required_str(arguments, "query")?;
                files::search(
                    self.config.index.as_deref(),
                    &self.config.search_roots,
                    query,
                    arg_u64(arguments, "limit").unwrap_or(10).clamp(1, 50) as usize,
                )
            }
            "aether_apps" => apps::list(arguments.get("query").and_then(Value::as_str)),
            "aether_windows" => self.control()?.call("windows.list", &json!({})),
            "aether_workspaces" => self.control()?.call("workspaces.list", &json!({})),
            "aether_audio" => match arguments
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("status")
            {
                "status" => audio::status(),
                "volume" => {
                    let percent = arg_u64(arguments, "percent")
                        .ok_or("missing percent (0-150)")?
                        .min(150);
                    audio::set_volume(percent)
                }
                other => Err(format!("unknown audio action {other:?}")),
            },
            "aether_launch_app" => {
                let app = required_str(arguments, "app")?;
                match self.control.as_ref() {
                    Some(control) => control.call("apps.launch", &json!({"name": app})),
                    None => apps::launch(app),
                }
            }
            "aether_focus_window" => {
                let id = arg_u64(arguments, "id").ok_or("missing window id")?;
                self.control()?.call("windows.focus", &json!({"id": id}))
            }
            "aether_close_window" => {
                let id = arg_u64(arguments, "id").ok_or("missing window id")?;
                if arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
                    return Err(format!(
                        "closing window {id} is destructive; repeat with confirm=true"
                    ));
                }
                self.control()?.call("windows.close", &json!({"id": id}))
            }
            "aether_switch_workspace" => {
                let index = arg_u64(arguments, "index").ok_or("missing workspace index")?;
                self.control()?
                    .call("workspaces.switch", &json!({"index": index}))
            }
            "aether_open_surface" => {
                let surface = required_str(arguments, "surface")?;
                self.control()?
                    .call("surface.open", &json!({"surface": surface}))
            }
            "aether_notify" => {
                let title = required_str(arguments, "title")?;
                let body = arguments
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                probe::notify(title, body)
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

impl Backend for SystemBackend {
    fn call(&mut self, tool: &str, arguments: &Value) -> Result<Value, String> {
        let outcome = self.dispatch(tool, arguments);
        self.record(tool, arguments, &outcome);
        outcome
    }
}

fn arg_u64(arguments: &Value, key: &str) -> Option<u64> {
    arguments.get(key).and_then(Value::as_u64)
}

fn required_str<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing {key}"))
}
