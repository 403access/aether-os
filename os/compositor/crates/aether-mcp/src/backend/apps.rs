//! Application discovery and launching.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

pub fn list(query: Option<&str>) -> Result<Value, String> {
    let needle = query.unwrap_or_default().to_lowercase();
    let apps: Vec<Value> = entries()
        .into_iter()
        .filter(|app| {
            needle.is_empty()
                || app["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
                || app["program"]
                    .as_str()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
        })
        .collect();
    Ok(json!({"apps": apps, "count": apps.len()}))
}

pub fn launch(name: &str) -> Result<Value, String> {
    let app = entries()
        .into_iter()
        .find(|app| {
            app["name"]
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(name))
                || app["program"]
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| format!("no application named {name:?}"))?;
    let program = app["program"].as_str().unwrap_or_default().to_string();
    let args: Vec<String> = app["args"]
        .as_array()
        .map(|args| {
            args.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut command = Command::new(&program);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Ok(display) = std::env::var("AETHER_WAYLAND_DISPLAY") {
            command.env("WAYLAND_DISPLAY", display);
        }
    }
    let child = command
        .spawn()
        .map_err(|error| format!("cannot launch {program}: {error}"))?;
    Ok(json!({
        "launched": app["name"],
        "program": program,
        "pid": child.id(),
    }))
}

fn entries() -> Vec<Value> {
    #[allow(unused_mut)]
    let mut apps = desktop_entries();
    #[cfg(target_os = "macos")]
    {
        apps.extend(macos_entries());
    }
    apps
}

fn desktop_entries() -> Vec<Value> {
    let mut dirs = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/share:/usr/share"));
    dirs.extend(std::env::split_paths(&data_dirs));
    dirs.dedup();
    aether_desktop::discover(&dirs)
        .into_iter()
        .map(|entry| {
            let detail = if entry.comment.is_empty() {
                "application".to_string()
            } else {
                entry.comment.clone()
            };
            json!({
                "name": entry.name,
                "detail": detail,
                "program": entry.program,
                "args": entry.args,
                "icon": entry.icon,
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn macos_entries() -> Vec<Value> {
    let mut apps = Vec::new();
    for root in ["/Applications", "/System/Applications"] {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(name) = name.strip_suffix(".app") else {
                continue;
            };
            apps.push(json!({
                "name": name,
                "detail": "macOS application",
                "program": "open",
                "args": ["-a", name],
                "icon": null,
            }));
        }
    }
    apps.sort_by_key(|app| app["name"].as_str().unwrap_or_default().to_lowercase());
    apps
}
