//! File search: prefer the Aether index daemon, fall back to a bounded walk.

use std::path::{Path, PathBuf};
use std::time::Duration;

use aether_protocol::index::{self, IndexHit, IndexRequest};
use serde_json::{json, Value};

const WALK_DEPTH: usize = 6;
const WALK_ENTRIES: usize = 40_000;
const INDEX_TIMEOUT: Duration = Duration::from_secs(2);

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

fn query_index(socket: &Path, query: &str, limit: usize) -> Result<Vec<IndexHit>, String> {
    let request = IndexRequest::new(query, limit);
    index::query(socket, &request, INDEX_TIMEOUT)
        .map(|response| response.results)
        .map_err(|error| format!("{}: {error}", socket.display()))
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
