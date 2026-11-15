//! `aether-sandbox` CLI: inspect support, print a plan, or run one command in
//! the kernel sandbox.

#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use aether_sandbox::{plan, AgentSpec, Capability, Trust};

#[derive(Clone, Debug)]
struct RunOptions {
    label: String,
    log: Option<PathBuf>,
    record: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            label: String::new(),
            log: None,
            record: true,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aether-sandbox: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some("check") => {
            print_support();
            Ok(())
        }
        Some("plan") => {
            let (spec, _) = parse_spec(args)?;
            println!("{:#?}", plan(&spec));
            Ok(())
        }
        Some("run") => {
            let (mut spec, options) = parse_spec(args)?;
            let (program, command_args) = split_command(args)?;
            if spec.id == "cli-agent" {
                spec.id = format!("cli-{}", std::process::id());
            }
            let plan = plan(&spec);
            if plan.blocked {
                return Err(format!("plan is blocked: {}", plan.blockers.join("; ")).into());
            }
            #[cfg(target_os = "linux")]
            {
                let mut child = aether_sandbox::linux::spawn_with_log(
                    &plan,
                    Path::new(&program),
                    &command_args,
                    options.log.as_deref(),
                )?;
                let registry_dir = aether_sandbox::registry::registry_dir();
                if let Err(error) = aether_sandbox::registry::ensure_dir(&registry_dir) {
                    eprintln!(
                        "aether-sandbox: registry {} unavailable: {error}",
                        registry_dir.display()
                    );
                }
                let socket_path =
                    aether_sandbox::registry::control_path_in(&registry_dir, &spec.id);
                let _ = std::fs::remove_file(&socket_path);
                let control = match std::os::unix::net::UnixListener::bind(&socket_path) {
                    Ok(listener) => {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &socket_path,
                            std::fs::Permissions::from_mode(0o666),
                        );
                        let pid = child.id();
                        std::thread::spawn(move || {
                            use std::io::{BufRead, BufReader, Write};
                            for stream in listener.incoming() {
                                let Ok(stream) = stream else { continue };
                                let Ok(mut writer) = stream.try_clone() else {
                                    continue;
                                };
                                let _ = stream
                                    .set_read_timeout(Some(std::time::Duration::from_secs(2)));
                                let mut line = String::new();
                                let _ = BufReader::new(stream).read_line(&mut line);
                                let result = match line.trim() {
                                    "kill" => aether_sandbox::force_stop(pid),
                                    "term" => aether_sandbox::terminate(pid),
                                    other => {
                                        let _ = writeln!(writer, "unknown request: {other}");
                                        continue;
                                    }
                                };
                                let _ = match result {
                                    Ok(()) => writeln!(writer, "ok"),
                                    Err(error) => writeln!(writer, "error: {error}"),
                                };
                            }
                        });
                        Some(socket_path.clone())
                    }
                    Err(error) => {
                        eprintln!(
                            "aether-sandbox: control socket {} unavailable: {error}",
                            socket_path.display()
                        );
                        None
                    }
                };
                let mut record = aether_sandbox::registry::SandboxRecord::from_plan(
                    &plan,
                    spec.trust,
                    child.id(),
                    &options.label,
                    &program,
                    &command_args,
                    options.log.as_deref(),
                );
                record.control_socket = control.map(|path| path.display().to_string());
                record.capabilities = spec
                    .capabilities
                    .iter()
                    .map(|capability| format!("{}:{}", capability.scope, capability.resource))
                    .collect();
                let recorded = if options.record {
                    match aether_sandbox::registry::write(&record) {
                        Ok(path) => {
                            println!("aether-sandbox: recorded {} at {}", spec.id, path.display());
                            true
                        }
                        Err(error) => {
                            eprintln!("aether-sandbox: cannot record sandbox: {error}");
                            false
                        }
                    }
                } else {
                    false
                };
                let status = child.wait();
                if recorded {
                    let _ = aether_sandbox::registry::remove(&spec.id);
                }
                let _ = std::fs::remove_file(aether_sandbox::registry::control_path_in(
                    &registry_dir,
                    &spec.id,
                ));
                std::process::exit(status?.code().unwrap_or(1));
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (program, command_args, options);
                Err(aether_sandbox::unsupported().into())
            }
        }
        Some(other) => Err(format!("unknown command: {other} (try --help)").into()),
    }
}

fn parse_spec(args: &[String]) -> Result<(AgentSpec, RunOptions), Box<dyn std::error::Error>> {
    let mut spec = AgentSpec::new("cli-agent", Trust::Untrusted);
    spec.tier = aether_sandbox::Tier::ProcessLandlock;
    let mut options = RunOptions::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--id" => {
                index += 1;
                spec.id = args.get(index).ok_or("--id needs a value")?.clone();
            }
            "--label" => {
                index += 1;
                options.label = args.get(index).ok_or("--label needs a value")?.clone();
            }
            "--trust" => {
                index += 1;
                spec.trust = match args.get(index).map(String::as_str) {
                    Some("trusted") => Trust::Trusted,
                    Some("untrusted") => Trust::Untrusted,
                    Some("hostile") => Trust::Hostile,
                    Some(other) => return Err(format!("unknown trust {other:?}").into()),
                    None => return Err("--trust needs a value".into()),
                };
            }
            "--read" => {
                index += 1;
                spec.read_paths
                    .push(PathBuf::from(args.get(index).ok_or("--read needs a path")?));
            }
            "--write" => {
                index += 1;
                spec.write_paths.push(PathBuf::from(
                    args.get(index).ok_or("--write needs a path")?,
                ));
            }
            "--net" => spec.allow_net = true,
            "--uid" => {
                index += 1;
                spec.uid = Some(args.get(index).ok_or("--uid needs a number")?.parse()?);
            }
            "--no-uid" => spec.isolate_uid = false,
            "--memory-mib" => {
                index += 1;
                spec.memory_mib = args
                    .get(index)
                    .ok_or("--memory-mib needs a number")?
                    .parse()?;
            }
            "--timeout" => {
                index += 1;
                spec.timeout_secs =
                    Some(args.get(index).ok_or("--timeout needs seconds")?.parse()?);
            }
            "--capability" => {
                index += 1;
                let value = args.get(index).ok_or("--capability needs scope:resource")?;
                let (scope, resource) = value
                    .split_once(':')
                    .ok_or("--capability needs scope:resource")?;
                spec.capabilities.push(Capability {
                    scope: scope.to_string(),
                    resource: resource.to_string(),
                });
            }
            "--log" => {
                index += 1;
                options.log = Some(PathBuf::from(args.get(index).ok_or("--log needs a path")?));
            }
            "--no-record" => options.record = false,
            "--" => break,
            other => return Err(format!("unknown option: {other}").into()),
        }
        index += 1;
    }
    Ok((spec, options))
}

fn split_command(args: &[String]) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    let separator = args.iter().position(|arg| arg == "--");
    let Some(separator) = separator else {
        return Err("run needs `-- <program> [args...]`".into());
    };
    let program = args.get(separator + 1).ok_or("missing program after --")?;
    Ok((program.clone(), args[separator + 2..].to_vec()))
}

fn print_support() {
    #[cfg(target_os = "linux")]
    {
        let support = aether_sandbox::linux::support();
        println!("landlock: {}", support.landlock);
        println!("seccomp: {}", support.seccomp);
        println!("cgroup-v2: {}", support.cgroup);
    }
    #[cfg(not(target_os = "linux"))]
    println!("{}", aether_sandbox::unsupported());
}

fn print_help() {
    println!(
        "aether-sandbox — kernel-enforced agent process sandbox\n\
         \n\
         Usage:\n\
           aether-sandbox check\n\
           aether-sandbox plan [options]\n\
           aether-sandbox run  [options] -- <program> [args...]\n\
         \n\
         Options:\n\
           --id ID           stable agent id (default: cli-<pid>)\n\
           --label TEXT      display name in the Agent sandboxes surface\n\
           --trust LEVEL     trusted | untrusted | hostile\n\
           --read PATH       grant read+execute on a subtree (repeatable)\n\
           --write PATH      grant read/write on a subtree (repeatable)\n\
           --net             keep network syscalls (default: deny)\n\
           --uid N           drop to an explicit agent UID\n\
           --no-uid          do not drop UID (tests/dev only; production must omit)\n\
           --memory-mib N    RLIMIT_AS and cgroup memory.max\n\
           --timeout SECS    RLIMIT_CPU ceiling\n\
           --capability S:R  record a capability (repeatable)\n\
           --log PATH        append the agent's stdout/stderr to PATH\n\
           --no-record       do not publish a record to the sandbox registry\n\
         \n\
         Runtime files under /usr, /bin, /lib, /lib64, /etc/ld.so.cache, /dev and\n\
         /proc are always read/execute-only. Everything else is denied by Landlock."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_identity_label_and_capability() {
        let (spec, options) = parse_spec(&args(&[
            "run",
            "--id",
            "build",
            "--label",
            "Build agent",
            "--trust",
            "untrusted",
            "--capability",
            "fs.read:/work",
            "--log",
            "/tmp/build.log",
            "--",
            "/bin/true",
        ]))
        .unwrap();
        assert_eq!(spec.id, "build");
        assert_eq!(spec.trust, Trust::Untrusted);
        assert_eq!(spec.capabilities[0].scope, "fs.read");
        assert_eq!(spec.capabilities[0].resource, "/work");
        assert_eq!(options.label, "Build agent");
        assert_eq!(
            options.log.as_deref(),
            Some(std::path::Path::new("/tmp/build.log"))
        );
        assert!(options.record);
    }

    #[test]
    fn no_record_and_bad_capability_are_handled() {
        let (_, options) = parse_spec(&args(&["run", "--no-record", "--", "/bin/true"])).unwrap();
        assert!(!options.record);
        assert!(parse_spec(&args(&["run", "--capability", "broken"])).is_err());
        assert!(parse_spec(&args(&["run", "--trust", "sometimes"])).is_err());
    }
}
