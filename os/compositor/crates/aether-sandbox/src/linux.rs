//! Linux enforcement: apply limits to the child between fork and exec.
//!
//! No shell, no helper process and no extra context switch on the hot path. The
//! rules are installed once in `pre_exec`; the child then executes the agent
//! binary directly.

use std::collections::BTreeMap;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};

use landlock::{
    Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI,
};
use seccompiler::{
    apply_filter, BpfProgram, SeccompAction, SeccompFilter, SeccompRule, TargetArch,
};

use crate::cgroup::CgroupLimits;
use crate::{SandboxPlan, Tier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Support {
    pub landlock: bool,
    pub seccomp: bool,
    pub cgroup: bool,
}

pub fn support() -> Support {
    Support {
        landlock: landlock_supported(),
        seccomp: true,
        cgroup: Path::new("/sys/fs/cgroup/cgroup.controllers").exists(),
    }
}

pub fn landlock_supported() -> bool {
    Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|ruleset| ruleset.create())
        .is_ok()
}

/// Spawn an agent with filesystem, syscall, identity and resource limits.
pub fn spawn(plan: &SandboxPlan, program: &Path, args: &[String]) -> io::Result<Child> {
    ensure_plan(plan)?;
    let mut command = base_command(plan, program, args)?;
    command.spawn()
}

/// Spawn under a cgroup v2 root. The caller owns cleanup of the cgroup.
pub fn spawn_in_cgroup(
    plan: &SandboxPlan,
    program: &Path,
    args: &[String],
    cgroup_root: &Path,
) -> io::Result<Child> {
    ensure_plan(plan)?;
    let limits = CgroupLimits::new(plan.agent_id.clone(), plan.memory_mib, plan.cpu_weight);
    let cgroup = limits.materialize(cgroup_root)?;
    let mut command = base_command(plan, program, args)?;
    let child = command.spawn()?;
    if let Err(error) = limits.attach(&cgroup, child.id()) {
        let mut child = child;
        let _ = child.kill();
        return Err(error);
    }
    Ok(child)
}

/// Run and capture stdout/stderr; used by tests and `aether-sandbox run --check`.
pub fn capture(plan: &SandboxPlan, program: &Path, args: &[String]) -> io::Result<Output> {
    ensure_plan(plan)?;
    let mut command = base_command(plan, program, args)?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = command.spawn()?;
    child.wait_with_output()
}

fn ensure_plan(plan: &SandboxPlan) -> io::Result<()> {
    if plan.blocked {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("sandbox plan is blocked: {}", plan.blockers.join("; ")),
        ));
    }
    if plan.tier != Tier::ProcessLandlock {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "only the process-landlock tier is implemented in this crate",
        ));
    }
    Ok(())
}

fn base_command(plan: &SandboxPlan, program: &Path, args: &[String]) -> io::Result<Command> {
    let mut command = Command::new(program);
    command.args(args);
    let child_plan = plan.clone();
    unsafe {
        command.pre_exec(move || apply_pre_exec(&child_plan));
    }
    Ok(command)
}

fn apply_pre_exec(plan: &SandboxPlan) -> io::Result<()> {
    if let Some(uid) = plan.uid {
        if unsafe { libc::geteuid() } != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "dropping to an agent UID requires root/CAP_SETUID",
            ));
        }
        let empty = std::ptr::null::<libc::gid_t>();
        if unsafe { libc::setgroups(0, empty) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::setgid(uid) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::setuid(uid) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    apply_rlimits(plan)?;
    apply_landlock(plan)?;
    apply_seccomp(plan.allow_net)
}

fn apply_rlimits(plan: &SandboxPlan) -> io::Result<()> {
    let memory = plan.memory_mib.saturating_mul(1024 * 1024);
    let memory = libc::rlimit {
        rlim_cur: memory,
        rlim_max: memory,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &memory) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let processes = libc::rlimit {
        rlim_cur: 256,
        rlim_max: 256,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &processes) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if let Some(seconds) = plan.timeout_secs {
        let cpu = libc::rlimit {
            rlim_cur: seconds.saturating_add(1),
            rlim_max: seconds.saturating_add(1),
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_CPU, &cpu) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn apply_landlock(plan: &SandboxPlan) -> io::Result<()> {
    let mut rules: Vec<Result<PathBeneath<PathFd>, landlock::RulesetError>> = Vec::new();
    for path in &plan.read_paths {
        if let Some(rule) = path_rule(path, false) {
            rules.push(Ok(rule));
        }
    }
    for path in &plan.write_paths {
        if let Some(rule) = path_rule(path, true) {
            rules.push(Ok(rule));
        }
    }
    let result = Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|ruleset| ruleset.create())
        .and_then(|ruleset| ruleset.add_rules(rules))
        .and_then(|ruleset| ruleset.restrict_self());
    result.map_err(|error| io::Error::other(format!("Landlock restriction failed: {error}")))?;
    Ok(())
}

fn path_rule(path: &Path, write: bool) -> Option<PathBeneath<PathFd>> {
    let metadata = std::fs::metadata(path).ok()?;
    let file = PathFd::new(path).ok()?;
    let access: landlock::BitFlags<AccessFs> = if write {
        if metadata.is_dir() {
            AccessFs::from_all(ABI::V1)
        } else {
            AccessFs::ReadFile | AccessFs::WriteFile
        }
    } else if metadata.is_dir() {
        AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::Execute
    } else {
        AccessFs::ReadFile | AccessFs::Execute
    };
    Some(PathBeneath::new(file, access))
}

fn apply_seccomp(allow_net: bool) -> io::Result<()> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    for syscall in denied_syscalls(allow_net) {
        rules.insert(syscall, Vec::new());
    }
    let arch: TargetArch = std::env::consts::ARCH.try_into().map_err(|error| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("seccomp target architecture: {error:?}"),
        )
    })?;
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|error| io::Error::other(error.to_string()))?;
    let program: BpfProgram = filter
        .try_into()
        .map_err(|error: seccompiler::BackendError| io::Error::other(error.to_string()))?;
    apply_filter(&program)
        .map_err(|error| io::Error::other(format!("seccomp filter failed: {error}")))
}

/// Hard-deny syscalls. This is a deny-list, not a language allowlist; the
/// filesystem allowlist is Landlock's job. A deny-list keeps dynamic runtimes
/// working while removing the syscalls that escape the process boundary.
pub fn denied_syscalls(allow_net: bool) -> Vec<i64> {
    let mut syscalls = vec![
        libc::SYS_ptrace,
        libc::SYS_bpf,
        libc::SYS_perf_event_open,
        libc::SYS_userfaultfd,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_unshare,
        libc::SYS_setns,
        libc::SYS_keyctl,
        libc::SYS_add_key,
        libc::SYS_request_key,
        libc::SYS_init_module,
        libc::SYS_finit_module,
        libc::SYS_delete_module,
        libc::SYS_kexec_load,
        libc::SYS_kexec_file_load,
        libc::SYS_open_by_handle_at,
        libc::SYS_name_to_handle_at,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_io_uring_setup,
        libc::SYS_io_uring_enter,
        libc::SYS_io_uring_register,
    ];
    if !allow_net {
        syscalls.extend([
            libc::SYS_socket,
            libc::SYS_socketpair,
            libc::SYS_connect,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_accept,
            libc::SYS_accept4,
            libc::SYS_sendto,
            libc::SYS_recvfrom,
            libc::SYS_sendmsg,
            libc::SYS_recvmsg,
            libc::SYS_getsockname,
            libc::SYS_getpeername,
            libc::SYS_setsockopt,
            libc::SYS_getsockopt,
            libc::SYS_shutdown,
        ]);
    }
    syscalls.sort_unstable();
    syscalls.dedup();
    syscalls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan, AgentSpec, Trust};
    use std::path::PathBuf;

    fn test_plan(read: Option<PathBuf>) -> SandboxPlan {
        let mut spec = AgentSpec::new("sandbox-test", Trust::Untrusted);
        spec.isolate_uid = false;
        spec.timeout_secs = Some(10);
        if let Some(path) = read {
            spec.read_paths.push(path);
        }
        plan(&spec)
    }

    fn skip_if_unsupported() -> bool {
        !support().landlock
    }

    fn which(name: &str) -> PathBuf {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| PathBuf::from(name))
    }

    #[test]
    fn seccomp_denies_network_and_privileged_syscalls() {
        let denied = denied_syscalls(false);
        assert!(denied.contains(&libc::SYS_ptrace));
        assert!(denied.contains(&libc::SYS_socket));
        assert!(!denied.contains(&libc::SYS_read));
        let networked = denied_syscalls(true);
        assert!(!networked.contains(&libc::SYS_socket));
    }

    #[test]
    fn allowed_file_is_readable() {
        if skip_if_unsupported() {
            return;
        }
        let path = std::env::temp_dir().join(format!(
            "aether-sandbox-read-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"inside\n").unwrap();
        let plan = test_plan(Some(path.clone()));
        let output =
            capture(&plan, &which("cat"), &[path.display().to_string()]).expect("sandbox spawn");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"inside\n");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn secret_outside_the_allowlist_is_denied() {
        if skip_if_unsupported() {
            return;
        }
        let secret = std::env::temp_dir().join(format!(
            "aether-sandbox-secret-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&secret, b"top-secret\n").unwrap();
        let plan = test_plan(None);
        let output =
            capture(&plan, &which("cat"), &[secret.display().to_string()]).expect("sandbox spawn");
        assert!(!output.status.success(), "Landlock should deny the secret");
        std::fs::remove_file(secret).ok();
    }

    #[test]
    fn seccomp_denies_unshare() {
        if skip_if_unsupported() {
            return;
        }
        let unshare = which("unshare");
        if !unshare.is_file() {
            return;
        }
        let plan = test_plan(None);
        let output = capture(&plan, &unshare, &[String::from("true")]).expect("sandbox spawn");
        assert!(!output.status.success(), "seccomp should deny unshare");
    }

    #[test]
    fn write_outside_the_scratch_path_is_denied() {
        if skip_if_unsupported() {
            return;
        }
        let plan = test_plan(None);
        let output = capture(
            &plan,
            &which("touch"),
            &[String::from("/tmp/aether-sandbox-denied-write")],
        )
        .expect("sandbox spawn");
        assert!(!output.status.success(), "Landlock should deny the write");
        assert!(!Path::new("/tmp/aether-sandbox-denied-write").exists());
    }
}
