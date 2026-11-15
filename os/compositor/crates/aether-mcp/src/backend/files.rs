//! File search: prefer the Aether index daemon, fall back to a bounded walk.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

const WALK_DEPTH: usize = 6;
const WALK_ENTRIES: usize = 40_000;

pub fn search(
    index: Option<&Path>,
    roots: &[PathBuf],
    query: &str,
    limit: usize,
) -> Result<Value, String> {
    if let Some(socket) = index {
        match query_index(socket, query, limit) {
            Ok(results) => {
                return Ok(json!({
                    "query": query,
                    "source": "index",
                    "results": results,
                }))
            }
            Err(error) => {
                eprintln!("aether-mcp: index query failed: {error}; falling back to walk");
            }
        }
    }
    let mut results = Vec::new();
    let mut scanned = 0usize;
    let needle = query.to_lowercase();
    for root in roots {
        walk(root, 0, &needle, limit, &mut results, &mut scanned);
        if results.len() >= limit || scanned >= WALK_ENTRIES {
            break;
        }
    }
    Ok(json!({
        "query": query,
        "source": "walk",
        "results": results,
    }))
}

fn query_index(socket: &Path, query: &str, limit: usize) -> Result<Vec<Value>, String> {
    let stream =
        UnixStream::connect(socket).map_err(|error| format!("{}: {error}", socket.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let mut writer = stream.try_clone().map_err(|error| error.to_string())?;
    writeln!(writer, "{}", json!({"query": query, "limit": limit}))
        .map_err(|error| error.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    let response: Value = serde_json::from_str(line.trim()).map_err(|error| error.to_string())?;
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(results)
}

fn walk(
    directory: &Path,
    depth: usize,
    needle: &str,
    limit: usize,
    results: &mut Vec<Value>,
    scanned: &mut usize,
) {
    if depth > WALK_DEPTH || results.len() >= limit || *scanned >= WALK_ENTRIES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if results.len() >= limit || *scanned >= WALK_ENTRIES {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        *scanned += 1;
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            walk(&path, depth + 1, needle, limit, results, scanned);
        } else if name.to_lowercase().contains(needle) {
            results.push(json!({
                "title": name,
                "subtitle": directory.display().to_string(),
                "path": path.display().to_string(),
            }));
        }
    }
}
