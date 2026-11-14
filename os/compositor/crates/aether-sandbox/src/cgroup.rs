//! cgroup v2 resource limits for a sandboxed agent.
//!
//! The supervisor passes a writable cgroup root (normally `/sys/fs/cgroup`).
//! Tests use a temporary directory and assert the controller files that would
//! be written; the kernel applies them when the root is real.

use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CgroupLimits {
    pub agent_id: String,
    pub memory_mib: u64,
    pub cpu_weight: u16,
    pub pids_max: u32,
}

impl CgroupLimits {
    pub fn new(agent_id: impl Into<String>, memory_mib: u64, cpu_weight: u16) -> Self {
        Self {
            agent_id: agent_id.into(),
            memory_mib,
            cpu_weight,
            pids_max: 256,
        }
    }

    pub fn path(&self, root: &Path) -> PathBuf {
        root.join("aether").join(sanitize(&self.agent_id))
    }

    /// Create the cgroup and write limits. `root` must be a cgroup v2 mount.
    pub fn materialize(&self, root: &Path) -> io::Result<PathBuf> {
        let path = self.path(root);
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("memory.max"), format!("{}M", self.memory_mib))?;
        std::fs::write(
            path.join("cpu.weight"),
            ((self.cpu_weight.max(1) as u32) * 100).to_string(),
        )?;
        std::fs::write(path.join("pids.max"), self.pids_max.to_string())?;
        Ok(path)
    }

    pub fn attach(&self, cgroup: &Path, pid: u32) -> io::Result<()> {
        std::fs::write(cgroup.join("cgroup.procs"), pid.to_string())
    }
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_limits_into_a_cgroup_tree() {
        let root = std::env::temp_dir().join(format!(
            "aether-cgroup-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let limits = CgroupLimits::new("agent/one", 512, 25);
        let path = limits.materialize(&root).unwrap();
        assert_eq!(
            std::fs::read_to_string(path.join("memory.max")).unwrap(),
            "512M"
        );
        assert_eq!(
            std::fs::read_to_string(path.join("cpu.weight")).unwrap(),
            "2500"
        );
        assert_eq!(
            std::fs::read_to_string(path.join("pids.max")).unwrap(),
            "256"
        );
        assert!(path.ends_with("aether/agent-one"));
        std::fs::remove_dir_all(root).ok();
    }
}
