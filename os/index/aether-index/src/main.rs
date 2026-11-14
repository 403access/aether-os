//! Aether index daemon.
//!
//! Scans configured roots, keeps an in-memory index fresh with inotify, serves
//! newline-delimited JSON queries over a Unix socket, and optionally mirrors
//! every change into a file-backed event ring.

mod index;
mod ring;

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use ring::{EventKind, RingWriter};

use crate::index::Index;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--ring-tail") {
        let path = args
            .get(2)
            .ok_or("usage: aether-index --ring-tail <path>")?;
        let mut reader = ring::RingReader::open(Path::new(path))?;
        while let Some((kind, path)) = reader.next_event() {
            println!("{kind:?} {path}");
        }
        return Ok(());
    }
    let roots: Vec<PathBuf> = std::env::var("AETHER_INDEX_ROOTS")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default()
        .split(':')
        .filter(|root| !root.is_empty())
        .map(|root| {
            let path = PathBuf::from(root);
            fs::canonicalize(&path).unwrap_or(path)
        })
        .collect();
    let limit = env_usize("AETHER_INDEX_LIMIT", 20_000);
    let depth = env_usize("AETHER_INDEX_DEPTH", 12);
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let socket_path = std::env::var("AETHER_INDEX_SOCKET")
        .unwrap_or_else(|_| format!("{runtime}/aether-index.sock"));
    let delay_ms = env_usize("AETHER_INDEX_DELAY_MS", 0);

    let index = Arc::new(RwLock::new(Index::build(&roots, limit, depth)));
    let ring = std::env::var_os("AETHER_INDEX_RING").map(|path| {
        let path = PathBuf::from(path);
        let ring = RingWriter::open(&path, 1024 * 1024)
            .unwrap_or_else(|error| panic!("cannot open ring {}: {error}", path.display()));
        Arc::new(ring)
    });

    if let Some(ring) = &ring {
        for path in index.read().unwrap().paths() {
            ring.push(EventKind::Indexed, &path.to_string_lossy());
        }
    }

    let _ = fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    println!(
        "aether-index: ready at {socket_path} ({} documents, {} indexed)",
        index.read().unwrap().len(),
        index.read().unwrap().len()
    );

    if std::env::var("AETHER_INDEX_WATCH").ok().as_deref() != Some("0") {
        let roots = roots.clone();
        let index = index.clone();
        let ring = ring.clone();
        thread::spawn(move || watch_roots(roots, index, ring, depth));
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let index = index.clone();
                thread::spawn(move || serve(stream, index, delay_ms));
            }
            Err(error) => eprintln!("aether-index: accept failed: {error}"),
        }
    }
    Ok(())
}

fn serve(stream: UnixStream, index: Arc<RwLock<Index>>, delay_ms: usize) {
    let mut writer = match stream.try_clone() {
        Ok(stream) => stream,
        Err(_) => return,
    };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(_) => continue,
        };
        let query = request
            .get("query")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let limit = request
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(8)
            .clamp(1, 20_000) as usize;
        let hits = index.read().unwrap().query(&query, limit);
        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms as u64));
        }
        let results: Vec<serde_json::Value> = hits
            .into_iter()
            .map(|hit| {
                serde_json::json!({
                    "title": hit.title,
                    "subtitle": hit.subtitle,
                    "path": hit.path,
                })
            })
            .collect();
        if writeln!(writer, "{}", serde_json::json!({ "results": results })).is_err() {
            break;
        }
    }
}

fn watch_roots(
    roots: Vec<PathBuf>,
    index: Arc<RwLock<Index>>,
    ring: Option<Arc<RingWriter>>,
    depth: usize,
) {
    let mut inotify = match Inotify::init() {
        Ok(inotify) => inotify,
        Err(error) => {
            eprintln!("aether-index: inotify unavailable: {error}");
            return;
        }
    };
    let mut watches: HashMap<WatchDescriptor, PathBuf> = HashMap::new();
    for root in &roots {
        add_watches(&mut inotify, root, &mut watches, depth);
    }
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let events = match inotify.read_events_blocking(&mut buffer) {
            Ok(events) => events,
            Err(error) => {
                eprintln!("aether-index: inotify read failed: {error}");
                return;
            }
        };
        for event in events {
            let Some(directory) = watches.get(&event.wd) else {
                continue;
            };
            let Some(name) = event.name.as_ref() else {
                continue;
            };
            let path = directory.join(name);
            if event.mask.contains(EventMask::ISDIR) {
                if event
                    .mask
                    .intersects(EventMask::CREATE | EventMask::MOVED_TO | EventMask::MOVED_FROM)
                {
                    add_watches(&mut inotify, &path, &mut watches, depth);
                    index_directory(&path, &index, &ring);
                }
                continue;
            }
            if event
                .mask
                .intersects(EventMask::DELETE | EventMask::MOVED_FROM)
            {
                if index.write().unwrap().remove(&path) {
                    println!("aether-index: removed {}", path.display());
                    if let Some(ring) = &ring {
                        ring.push(EventKind::Removed, &path.to_string_lossy());
                    }
                }
            } else if event.mask.intersects(
                EventMask::CREATE
                    | EventMask::MOVED_TO
                    | EventMask::CLOSE_WRITE
                    | EventMask::ATTRIB,
            ) {
                if index.write().unwrap().upsert(&path) {
                    println!("aether-index: indexed {}", path.display());
                    if let Some(ring) = &ring {
                        ring.push(EventKind::Indexed, &path.to_string_lossy());
                    }
                }
            }
        }
    }
}

fn add_watches(
    inotify: &mut Inotify,
    root: &Path,
    watches: &mut HashMap<WatchDescriptor, PathBuf>,
    depth: usize,
) {
    let mask = WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::MOVED_FROM
        | WatchMask::MOVED_TO
        | WatchMask::CLOSE_WRITE
        | WatchMask::ATTRIB;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((directory, level)) = stack.pop() {
        if !directory.is_dir() {
            continue;
        }
        if let Ok(watch) = inotify.watches().add(&directory, mask) {
            watches.insert(watch, directory.clone());
        }
        if level >= depth {
            continue;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !ignored(&path) {
                stack.push((path, level + 1));
            }
        }
    }
}

fn index_directory(path: &Path, index: &Arc<RwLock<Index>>, ring: &Option<Arc<RingWriter>>) {
    let mut stack = vec![path.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !ignored(&path) {
                    stack.push(path);
                }
            } else if index.write().unwrap().upsert(&path) {
                if let Some(ring) = ring {
                    ring.push(EventKind::Indexed, &path.to_string_lossy());
                }
            }
        }
    }
}

fn ignored(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            name.starts_with('.')
                || matches!(
                    name,
                    "node_modules" | "target" | "__pycache__" | ".git" | ".cache" | ".direnv"
                )
        })
        .unwrap_or(false)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
