//! `aether-sandbox` CLI: inspect support, print a plan, or run one command in
//! the kernel sandbox.

#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use aether_sandbox::{plan, AgentSpec, Trust};

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
            let spec = parse_spec(args)?;
            println!("{:#?}", plan(&spec));
            Ok(())
        }
        Some("run") => {
            let spec = parse_spec(args)?;
            let (program, command_args) = split_command(args)?;
            let plan = plan(&spec);
            if plan.blocked {
                return Err(format!("plan is blocked: {}", plan.blockers.join("; ")).into());
            }
            #[cfg(target_os = "linux")]
            {
                let mut child =
                    aether_sandbox::linux::spawn(&plan, Path::new(&program), &command_args)?;
                let status = child.wait()?;
                std::process::exit(status.code().unwrap_or(1));
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = program;
                let _ = command_args;
                Err(aether_sandbox::unsupported().into())
            }
        }
        Some(other) => Err(format!("unknown command: {other} (try --help)").into()),
    }
}

fn parse_spec(args: &[String]) -> Result<AgentSpec, Box<dyn std::error::Error>> {
    let mut spec = AgentSpec::new("cli-agent", Trust::Untrusted);
    spec.tier = aether_sandbox::Tier::ProcessLandlock;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
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
            "--" => break,
            other => return Err(format!("unknown option: {other}").into()),
        }
        index += 1;
    }
    Ok(spec)
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
           aether-sandbox plan [--read PATH] [--write PATH] [--net] [--no-uid]\n\
           aether-sandbox run  [options] -- <program> [args...]\n\
         \n\
         Options:\n\
           --read PATH       grant read+execute on a subtree (repeatable)\n\
           --write PATH      grant read/write on a subtree (repeatable)\n\
           --net             keep network syscalls (default: deny)\n\
           --uid N           drop to an explicit agent UID\n\
           --no-uid          do not drop UID (tests/dev only; production must omit)\n\
           --memory-mib N    RLIMIT_AS and cgroup memory.max\n\
           --timeout SECS    RLIMIT_CPU ceiling\n\
         \n\
         Runtime files under /usr, /bin, /lib, /lib64, /etc/ld.so.cache, /dev and\n\
         /proc are always read/execute-only. Everything else is denied by Landlock."
    );
}
