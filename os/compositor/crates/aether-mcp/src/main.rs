//! `aether-mcp`: MCP server for the running Aether system.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use aether_mcp::{tools, Server, SystemBackend};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "aether-mcp {}\n\
             MCP server for Aether OS over stdio.\n\n\
             USAGE:\n  aether-mcp [--socket PATH] [--tools]\n\n\
             ENV:\n  AETHER_CONTROL_SOCKET  shell control socket\n\
             AETHER_INDEX_SOCKET    index daemon socket\n\
             AETHER_MCP_SEARCH_ROOTS colon-separated fallback search roots\n\
             AETHER_MCP_AUDIT       audit log path\n",
            aether_mcp::SERVER_VERSION
        );
        return;
    }
    if args.iter().any(|arg| arg == "--version") {
        println!("aether-mcp {}", aether_mcp::SERVER_VERSION);
        return;
    }
    if args.iter().any(|arg| arg == "--tools") {
        println!(
            "{}",
            serde_json::to_string_pretty(&tools::catalog()).unwrap_or_default()
        );
        return;
    }

    let socket = args
        .windows(2)
        .find(|pair| pair[0] == "--socket")
        .map(|pair| PathBuf::from(&pair[1]));
    match socket {
        Some(path) => serve_socket(&path),
        None => serve_stdio(),
    }
}

fn serve_stdio() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut server = Server::new(Box::new(SystemBackend::from_env()));
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(response) = server.handle_line(&line) {
            if writeln!(stdout, "{response}").is_err() {
                break;
            }
            let _ = stdout.flush();
        }
    }
}

fn serve_socket(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(path);
    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("aether-mcp: cannot bind {}: {error}", path.display());
            std::process::exit(1);
        }
    };
    eprintln!("aether-mcp: listening at {}", path.display());
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let Ok(mut writer) = stream.try_clone() else {
            continue;
        };
        let mut server = Server::new(Box::new(SystemBackend::from_env()));
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            if let Some(response) = server.handle_line(&line) {
                if writeln!(writer, "{response}").is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        }
    }
}
