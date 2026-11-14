//! Kernel-enforced agent sandbox.
//!
//! The process tier is the workhorse on the i7-7700K: one process per agent,
//! Landlock filesystem rules, a seccomp deny-list, per-agent UID, rlimits and
//! optional cgroup v2 limits. MicroVM execution is planned elsewhere; this
//! crate implements the tier that ships first.
//!
//! Policy is pure and testable on any host. Kernel enforcement is Linux-only;
//! macOS builds keep the planner and return `Unsupported` from `spawn`.

use std::path::PathBuf;

pub mod cgroup;

#[cfg(target_os = "linux")]
pub mod linux;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    Trusted,
    Untrusted,
    Hostile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    InProcess,
    ProcessLandlock,
    Microvm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub scope: String,
    pub resource: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSpec {
    pub id: String,
    pub trust: Trust,
    pub tier: Tier,
    pub image: Option<String>,
    pub cpu_weight: u16,
    pub memory_mib: u64,
    pub timeout_secs: Option<u64>,
    pub capabilities: Vec<Capability>,
    pub read_paths: Vec<PathBuf>,
    pub write_paths: Vec<PathBuf>,
    pub allow_net: bool,
    /// None lets the supervisor choose a UID from the reserved range.
    pub uid: Option<u32>,
    /// False is a development/test escape hatch. Production must stay true.
    pub isolate_uid: bool,
}

impl AgentSpec {
    pub fn new(id: impl Into<String>, trust: Trust) -> Self {
        Self {
            id: id.into(),
            trust,
            tier: match trust {
                Trust::Trusted => Tier::InProcess,
                _ => Tier::ProcessLandlock,
            },
            image: None,
            cpu_weight: 25,
            memory_mib: 512,
            timeout_secs: Some(60),
            capabilities: Vec::new(),
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            allow_net: false,
            uid: None,
            isolate_uid: true,
        }
    }

    pub fn with_workspace(mut self, root: impl Into<PathBuf>, scratch: impl Into<PathBuf>) -> Self {
        self.read_paths.push(root.into());
        self.write_paths.push(scratch.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxPlan {
    pub agent_id: String,
    pub tier: Tier,
    pub enforcement: &'static str,
    pub read_paths: Vec<PathBuf>,
    pub write_paths: Vec<PathBuf>,
    pub allow_net: bool,
    pub uid: Option<u32>,
    pub memory_mib: u64,
    pub cpu_weight: u16,
    pub timeout_secs: Option<u64>,
    pub syscall_profile: &'static str,
    pub blocked: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl SandboxPlan {
    /// The process tier can be spawned by this crate.
    pub fn is_kernel_sandbox(&self) -> bool {
        self.tier == Tier::ProcessLandlock && !self.blocked
    }
}

/// Plan the process sandbox. This mirrors the TypeScript policy but stays small
/// enough to run in the supervisor's critical path.
pub fn plan(spec: &AgentSpec) -> SandboxPlan {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut tier = spec.tier;

    match spec.trust {
        Trust::Hostile => {
            tier = Tier::Microvm;
            if spec.image.as_deref().unwrap_or("").is_empty() {
                blockers.push("microVM tier requires an image id".into());
            }
        }
        Trust::Untrusted if tier == Tier::InProcess => {
            tier = Tier::ProcessLandlock;
            warnings.push("untrusted in-process request escalated to a Landlock process".into());
        }
        Trust::Untrusted => {}
        Trust::Trusted => {}
    }

    if tier == Tier::Microvm {
        warnings.push("microVM execution is planned, not implemented by this crate".into());
    }
    if spec.timeout_secs.is_none() {
        warnings.push("no timeout: the agent may run indefinitely".into());
    }
    if spec.memory_mib > 2048 {
        warnings.push("agent memory exceeds the practical 2 GiB budget".into());
    }
    if spec.cpu_weight > 50 {
        warnings.push("CPU weight above 50% will contend on four cores".into());
    }
    if spec
        .capabilities
        .iter()
        .any(|capability| capability.scope == "fs.write" && capability.resource == "*")
    {
        warnings.push("whole-filesystem write capability defeats isolation".into());
    }

    let uid = if spec.isolate_uid {
        spec.uid.or_else(|| {
            if tier == Tier::ProcessLandlock {
                Some(uid_for(&spec.id))
            } else {
                None
            }
        })
    } else {
        None
    };

    // Runtime files are read-only. A missing path is skipped by the enforcer,
    // so container images without /lib64 still work.
    let mut read_paths = vec![
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/etc/ld.so.cache"),
        PathBuf::from("/dev/null"),
        PathBuf::from("/dev/urandom"),
        PathBuf::from("/proc"),
        PathBuf::from("/nix/store"),
    ];
    read_paths.extend(spec.read_paths.iter().cloned());
    read_paths.sort();
    read_paths.dedup();

    SandboxPlan {
        agent_id: spec.id.clone(),
        tier,
        enforcement: match tier {
            Tier::InProcess => "none",
            Tier::ProcessLandlock => "landlock-seccomp",
            Tier::Microvm => "microvm-vfio",
        },
        read_paths,
        write_paths: spec.write_paths.clone(),
        allow_net: spec.allow_net,
        uid,
        memory_mib: spec.memory_mib,
        cpu_weight: spec.cpu_weight,
        timeout_secs: spec.timeout_secs,
        syscall_profile: "baseline-deny",
        blocked: !blockers.is_empty(),
        blockers,
        warnings,
    }
}

/// Stable UID in the reserved 4001..4095 range.
pub fn uid_for(id: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in id.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    4001 + (hash % 95)
}

#[derive(Debug)]
pub struct SandboxError(pub String);

impl std::fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SandboxError {}

#[cfg(not(target_os = "linux"))]
pub fn unsupported() -> &'static str {
    "kernel sandboxing is Linux-only (Landlock + seccomp-BPF)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrusted_agents_get_process_landlock_and_a_stable_uid() {
        let spec = AgentSpec::new("agent-a", Trust::Untrusted);
        let planned = plan(&spec);
        assert_eq!(planned.tier, Tier::ProcessLandlock);
        assert_eq!(planned.enforcement, "landlock-seccomp");
        assert!(planned.is_kernel_sandbox());
        assert!(planned.uid.unwrap() >= 4001 && planned.uid.unwrap() <= 4095);
        assert_eq!(planned.uid, plan(&spec).uid);
        assert!(planned
            .read_paths
            .iter()
            .any(|path| path == &PathBuf::from("/usr")));
    }

    #[test]
    fn untrusted_in_process_is_escalated() {
        let mut spec = AgentSpec::new("agent-a", Trust::Untrusted);
        spec.tier = Tier::InProcess;
        let plan = plan(&spec);
        assert_eq!(plan.tier, Tier::ProcessLandlock);
        assert!(plan.warnings.iter().any(|line| line.contains("escalated")));
    }

    #[test]
    fn hostile_requires_a_microvm_image() {
        let spec = AgentSpec::new("hostile", Trust::Hostile);
        let plan = plan(&spec);
        assert_eq!(plan.tier, Tier::Microvm);
        assert!(plan.blocked);
        assert!(plan.blockers.iter().any(|line| line.contains("image")));
        assert!(!plan.is_kernel_sandbox());
    }

    #[test]
    fn workspace_paths_and_network_are_carried_into_the_plan() {
        let mut spec = AgentSpec::new("agent-a", Trust::Untrusted)
            .with_workspace("/work/aether", "/work/aether/scratch");
        spec.allow_net = true;
        let plan = plan(&spec);
        assert!(plan
            .read_paths
            .iter()
            .any(|path| path == &PathBuf::from("/work/aether")));
        assert_eq!(
            plan.write_paths,
            vec![PathBuf::from("/work/aether/scratch")]
        );
        assert!(plan.allow_net);
    }

    #[test]
    fn timeouts_and_oversized_agents_warn() {
        let mut spec = AgentSpec::new("big", Trust::Untrusted);
        spec.memory_mib = 4096;
        spec.timeout_secs = None;
        let plan = plan(&spec);
        assert!(plan.warnings.iter().any(|line| line.contains("timeout")));
        assert!(plan.warnings.iter().any(|line| line.contains("memory")));
    }
}
