//! Runtime registry of sandboxed agents.
//!
//! `aether-sandbox run` writes one JSON record before waiting for the child and
//! removes it afterwards. The shell reads the directory to render the Agent
//! sandboxes surface. The registry is advisory: a record whose PID no longer
//! exists is shown as stale, and a killed CLI leaves the record behind until
//! the UI prunes it.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::SandboxPlan;

/// One live or recently stopped sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxRecord {
    pub id: String,
    pub label: String,
    pub pid: u32,
    pub trust: String,
    pub tier: String,
    pub enforcement: String,
    pub uid: Option<u32>,
    pub memory_mib: u64,
    pub cpu_weight: u16,
    pub timeout_secs: Option<u64>,
    pub allow_net: bool,
    pub capabilities: Vec<String>,
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
    pub program: String,
    pub args: Vec<String>,
    pub started_unix: u64,
    pub log_path: Option<String>,
    /// Unix socket owned by the launcher for stop requests.
    #[serde(default)]
    pub control_socket: Option<String>,
    pub warnings: Vec<String>,
}

impl SandboxRecord {
    pub fn from_plan(
        plan: &SandboxPlan,
        trust: crate::Trust,
        pid: u32,
        label: &str,
        program: &str,
        args: &[String],
        log_path: Option<&Path>,
    ) -> Self {
        Self {
            id: plan.agent_id.clone(),
            label: if label.is_empty() {
                program.to_string()
            } else {
                label.to_string()
            },
            pid,
            trust: trust.label().to_string(),
            tier: plan.tier.label().to_string(),
            enforcement: plan.enforcement.to_string(),
            uid: plan.uid,
            memory_mib: plan.memory_mib,
            cpu_weight: plan.cpu_weight,
            timeout_secs: plan.timeout_secs,
            allow_net: plan.allow_net,
            capabilities: Vec::new(),
            read_paths: plan
                .read_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            write_paths: plan
                .write_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            program: program.to_string(),
            args: args.to_vec(),
            started_unix: now_unix(),
            log_path: log_path.map(|path| path.display().to_string()),
            control_socket: None,
            warnings: plan.warnings.clone(),
        }
    }
}

/// Directory holding records, overridable for tests and alternate sessions.
pub fn registry_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("AETHER_SANDBOX_REGISTRY") {
        return PathBuf::from(path);
    }
    if is_root() {
        // Sandbox launchers need root for setuid; keep the registry where the
        // unprivileged shell can still read it and request a stop.
        return PathBuf::from("/run/aether/sandboxes");
    }
    let shared = PathBuf::from("/run/aether/sandboxes");
    if shared.exists() {
        return shared;
    }
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime).join("aether/sandboxes");
    }
    std::env::temp_dir().join("aether-sandboxes")
}

fn is_root() -> bool {
    // SAFETY: `geteuid` has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

fn dir_mode() -> u32 {
    if is_root() {
        0o755
    } else {
        0o700
    }
}

fn record_mode() -> u32 {
    if is_root() {
        0o644
    } else {
        0o600
    }
}

/// Write or replace one record in the default registry.
pub fn write(record: &SandboxRecord) -> io::Result<PathBuf> {
    write_in(&registry_dir(), record)
}

/// Remove one record from the default registry. Missing files are not errors.
pub fn remove(id: &str) -> io::Result<()> {
    remove_in(&registry_dir(), id)
}

/// Read every valid record, newest first. Invalid JSON is skipped.
pub fn list() -> Vec<SandboxRecord> {
    list_in(&registry_dir())
}

pub fn ensure_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(dir_mode()));
    Ok(())
}

pub fn write_in(dir: &Path, record: &SandboxRecord) -> io::Result<PathBuf> {
    ensure_dir(dir)?;
    let path = record_path_in(dir, &record.id);
    let temporary = dir.join(format!(
        ".{}.tmp.{}",
        safe_id(&record.id),
        std::process::id()
    ));
    let body = serde_json::to_vec_pretty(record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(&temporary, body)?;
    let _ = fs::set_permissions(&temporary, fs::Permissions::from_mode(record_mode()));
    fs::rename(&temporary, &path)?;
    Ok(path)
}

pub fn remove_in(dir: &Path, id: &str) -> io::Result<()> {
    match fs::remove_file(record_path_in(dir, id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn list_in(dir: &Path) -> Vec<SandboxRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records: Vec<SandboxRecord> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".json") && !name.starts_with('.'))
        })
        .filter_map(|entry| {
            let text = fs::read_to_string(entry.path()).ok()?;
            serde_json::from_str(&text).ok()
        })
        .collect();
    records.sort_by(|a, b| {
        b.started_unix
            .cmp(&a.started_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    records
}

pub fn record_path_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{}.json", safe_id(id)))
}

pub fn control_path_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{}.sock", safe_id(id)))
}

/// Ask a launcher to stop its child. The launcher owns the signal because the
/// child runs under a reserved UID the shell cannot signal directly.
pub fn request_stop(record: &SandboxRecord, force: bool) -> io::Result<()> {
    let Some(path) = record.control_socket.as_ref() else {
        return if force {
            crate::force_stop(record.pid)
        } else {
            crate::terminate(record.pid)
        };
    };
    let mut stream = std::os::unix::net::UnixStream::connect(path)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
    use std::io::{Read, Write};
    stream.write_all(if force { b"kill\n" } else { b"term\n" })?;
    stream.flush()?;
    let mut response = [0u8; 32];
    let read = stream.read(&mut response)?;
    let text = String::from_utf8_lossy(&response[..read]);
    if text.trim() == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(if text.trim().is_empty() {
            "launcher did not acknowledge the stop".to_string()
        } else {
            text.trim().to_string()
        }))
    }
}

/// Keep ids to a portable filename alphabet.
pub fn safe_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .take(64)
        .collect();
    if cleaned.is_empty() {
        "agent".to_string()
    } else {
        cleaned
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan, AgentSpec};

    fn temporary_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("aether-registry-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn record(id: &str) -> SandboxRecord {
        let spec = AgentSpec::new(id, crate::Trust::Untrusted);
        let planned = plan(&spec);
        SandboxRecord::from_plan(
            &planned,
            crate::Trust::Untrusted,
            4242,
            "test agent",
            "/bin/sh",
            &["-c".to_string(), "true".to_string()],
            None,
        )
    }

    #[test]
    fn write_list_and_remove_round_trip() {
        let dir = temporary_dir("round-trip");
        let entry = record("agent-one");
        write_in(&dir, &entry).unwrap();
        let listed = list_in(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "agent-one");
        assert_eq!(listed[0].pid, 4242);
        assert_eq!(listed[0].tier, "process-landlock");
        remove_in(&dir, "agent-one").unwrap();
        assert!(list_in(&dir).is_empty());
        assert!(remove_in(&dir, "agent-one").is_ok());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ids_are_sanitized_and_bounded() {
        assert_eq!(safe_id("../../etc/passwd"), "....etcpasswd");
        assert_eq!(safe_id(""), "agent");
        assert_eq!(safe_id(&"a".repeat(100)).len(), 64);
        let dir = Path::new("/tmp/registry");
        assert_eq!(record_path_in(dir, "a/b").file_name().unwrap(), "ab.json");
    }

    #[test]
    fn invalid_json_is_skipped() {
        let dir = temporary_dir("invalid");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bad.json"), "{").unwrap();
        write_in(&dir, &record("good")).unwrap();
        let listed = list_in(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "good");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn records_sort_newest_first() {
        let dir = temporary_dir("sort");
        let mut older = record("older");
        older.started_unix = 10;
        let mut newer = record("newer");
        newer.started_unix = 20;
        write_in(&dir, &older).unwrap();
        write_in(&dir, &newer).unwrap();
        let listed = list_in(&dir);
        assert_eq!(listed[0].id, "newer");
        assert_eq!(listed[1].id, "older");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn request_stop_uses_the_launcher_socket() {
        let dir = temporary_dir("stop-socket");
        fs::create_dir_all(&dir).unwrap();
        let socket = control_path_in(&dir, "demo");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let worker = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader, Write};
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            assert_eq!(line.trim(), "term");
            let mut stream = stream;
            stream.write_all(b"ok\n").unwrap();
        });
        let mut entry = record("demo");
        entry.control_socket = Some(socket.display().to_string());
        request_stop(&entry, false).unwrap();
        worker.join().unwrap();
        let _ = fs::remove_dir_all(dir);
    }
}
