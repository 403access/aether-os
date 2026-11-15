//! Non-blocking palette index queries.
//!
//! The compositor thread only publishes the newest query. A worker coalesces
//! keystrokes, talks to the index socket off-thread and returns tagged results;
//! stale generations are discarded by the caller.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use aether_shell::PaletteFile;

const DEBOUNCE: Duration = Duration::from_millis(30);
const SOCKET_TIMEOUT: Duration = Duration::from_millis(250);
const RESULT_LIMIT: usize = 5;
const EXPLORER_LIMIT: usize = 5000;

struct Shared {
    slot: Mutex<Option<(u64, String, usize)>>,
    signal: Condvar,
    alive: AtomicBool,
}

pub struct IndexWorker {
    shared: Arc<Shared>,
    results: Receiver<(u64, Vec<PaletteFile>)>,
    generation: u64,
}

impl IndexWorker {
    pub fn spawn(socket: PathBuf) -> Self {
        let shared = Arc::new(Shared {
            slot: Mutex::new(None),
            signal: Condvar::new(),
            alive: AtomicBool::new(true),
        });
        let (tx, results) = mpsc::channel();
        let worker_state = shared.clone();
        std::thread::spawn(move || {
            while worker_state.alive.load(Ordering::SeqCst) {
                let first = {
                    let mut slot = worker_state.slot.lock().unwrap();
                    loop {
                        if !worker_state.alive.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(request) = slot.take() {
                            break Some(request);
                        }
                        let (guard, _) = worker_state
                            .signal
                            .wait_timeout(slot, Duration::from_millis(200))
                            .unwrap();
                        slot = guard;
                    }
                };
                let Some((mut generation, mut query, mut limit)) = first else {
                    continue;
                };
                // Coalesce a burst of keystrokes before touching the socket.
                std::thread::sleep(DEBOUNCE);
                {
                    let mut slot = worker_state.slot.lock().unwrap();
                    if let Some((newer, newer_query, newer_limit)) = slot.take() {
                        generation = newer;
                        query = newer_query;
                        limit = newer_limit;
                    }
                }
                let files = query_index(&socket, &query, limit).unwrap_or_default();
                if tx.send((generation, files)).is_err() {
                    return;
                }
            }
        });
        Self {
            shared,
            results,
            generation: 0,
        }
    }

    /// Publish the newest query; returns its generation. Never blocks and
    /// never grows a queue: only the latest request is kept.
    pub fn request(&mut self, query: String) -> u64 {
        self.request_with_limit(query, RESULT_LIMIT)
    }

    /// Request filtered files for the explorer (larger than palette rows).
    pub fn request_explorer(&mut self) -> u64 {
        self.request_with_limit(String::new(), EXPLORER_LIMIT)
    }

    fn request_with_limit(&mut self, query: String, limit: usize) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        let mut slot = self.shared.slot.lock().unwrap();
        *slot = Some((self.generation, query, limit));
        self.shared.signal.notify_one();
        self.generation
    }

    pub fn current_generation(&self) -> u64 {
        self.generation
    }

    /// Supersede any in-flight query (empty query, palette closed).
    pub fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn poll(&self) -> Option<(u64, Vec<PaletteFile>)> {
        self.results.try_recv().ok()
    }
}

impl Drop for IndexWorker {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::SeqCst);
        self.shared.signal.notify_all();
    }
}

/// Apply a result only when it belongs to the newest request.
pub fn should_apply(generation: u64, current: u64) -> bool {
    generation == current
}

fn query_index(socket: &PathBuf, query: &str, limit: usize) -> Option<Vec<PaletteFile>> {
    let started = Instant::now();
    let mut stream = UnixStream::connect(socket).ok()?;
    let _ = stream.set_read_timeout(Some(SOCKET_TIMEOUT));
    let _ = stream.set_write_timeout(Some(SOCKET_TIMEOUT));
    let request = serde_json::json!({ "query": query, "limit": limit });
    writeln!(stream, "{request}").ok()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        eprintln!(
            "aether-shell: index query failed after {}µs",
            started.elapsed().as_micros()
        );
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(&line).ok()?;
    Some(
        value
            .get("results")
            .and_then(|results| results.as_array())
            .map(|results| {
                results
                    .iter()
                    .filter_map(|result| {
                        Some(PaletteFile {
                            name: result.get("title")?.as_str()?.to_string(),
                            detail: result
                                .get("subtitle")
                                .and_then(|value| value.as_str())
                                .unwrap_or("indexed file")
                                .to_string(),
                            path: result.get("path")?.as_str()?.to_string(),
                        })
                    })
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generations_are_discarded() {
        assert!(should_apply(7, 7));
        assert!(!should_apply(6, 7));
    }

    #[test]
    fn request_keeps_only_the_latest_query() {
        let shared = Arc::new(Shared {
            slot: Mutex::new(None),
            signal: Condvar::new(),
            alive: AtomicBool::new(true),
        });
        let (tx, _rx) = mpsc::channel();
        let mut worker = IndexWorker {
            shared: shared.clone(),
            results: _rx,
            generation: 0,
        };
        // No thread is running; inspect the slot directly.
        worker.request("a".into());
        worker.request("ab".into());
        let slot = shared.slot.lock().unwrap();
        assert_eq!(slot.as_ref().unwrap().1, "ab");
        drop(slot);
        let _ = tx;
        worker.shared.alive.store(false, Ordering::SeqCst);
    }
}
