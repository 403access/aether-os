//! `aether-chat`: the model loop behind the chat drawer.
//!
//! It connects to `AETHER_CHAT_SOCKET`, spawns `aether-mcp`, and turns prompts
//! into model turns plus MCP tool calls. Chat events stream back to the shell
//! as newline-delimited JSON. `--stdio` provides the same exchange on stdin and
//! stdout for tests and headless use.

mod agent;
mod mcp;
mod model;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use aether_protocol::chat::parse_prompt_line;
use agent::{Agent, Event};
use mcp::McpClient;
use model::{Model, OpenAiModel, ScriptModel};

struct Options {
    socket: Option<PathBuf>,
    stdio: bool,
    mcp: PathBuf,
    script: Option<PathBuf>,
    endpoint: Option<String>,
    model: String,
    api_key: Option<String>,
    max_rounds: usize,
}

fn main() {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => return,
        Err(error) => {
            eprintln!("aether-chat: {error}");
            std::process::exit(2);
        }
    };
    let mcp = match McpClient::connect(&options.mcp) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("aether-chat: {error}");
            std::process::exit(1);
        }
    };
    let model = build_model(&options);
    let mut agent = Agent::new(model, mcp, options.max_rounds);

    if options.stdio {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if let Some(prompt) = parse_prompt_line(&line) {
                run_prompt(&mut agent, &prompt, &mut stdout);
            }
        }
        return;
    }
    let Some(socket) = options.socket else {
        eprintln!("aether-chat: set --stdio or AETHER_CHAT_SOCKET");
        std::process::exit(2);
    };
    serve_socket(&socket, &mut agent);
}

fn parse_args() -> Result<Option<Options>, String> {
    let mut args = std::env::args().skip(1);
    let mut options = Options {
        socket: std::env::var_os("AETHER_CHAT_SOCKET").map(PathBuf::from),
        stdio: false,
        mcp: mcp_binary(),
        script: std::env::var_os("AETHER_CHAT_SCRIPT").map(PathBuf::from),
        endpoint: std::env::var("AETHER_CHAT_ENDPOINT").ok(),
        model: std::env::var("AETHER_CHAT_MODEL").unwrap_or_else(|_| "deepseek-chat".into()),
        api_key: std::env::var("AETHER_CHAT_API_KEY").ok(),
        max_rounds: std::env::var("AETHER_CHAT_MAX_ROUNDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(6),
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdio" => options.stdio = true,
            "--help" | "-h" => {
                println!(
                    "aether-chat {}\n\n\
                     USAGE:\n  aether-chat [--stdio | --socket PATH] [--mcp PATH]\n\
                     \x20             [--script FILE | --endpoint URL --model NAME --api-key KEY]\n\n\
                     ENV:\n  AETHER_CHAT_SOCKET    shell chat socket\n\
                     AETHER_MCP_BIN        aether-mcp binary\n\
                     AETHER_CHAT_SCRIPT    deterministic scripted model\n\
                     AETHER_CHAT_ENDPOINT  OpenAI-compatible endpoint (http:// or unix:)\n\
                     AETHER_CHAT_MODEL     model name\n\
                     AETHER_CHAT_API_KEY   bearer token\n\
                     AETHER_CHAT_MAX_ROUNDS tool rounds per prompt (default 6)\n",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(None);
            }
            "--version" => {
                println!("aether-chat {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--socket" => options.socket = Some(PathBuf::from(next(&mut args, "--socket")?)),
            "--mcp" => options.mcp = PathBuf::from(next(&mut args, "--mcp")?),
            "--script" => options.script = Some(PathBuf::from(next(&mut args, "--script")?)),
            "--endpoint" => options.endpoint = Some(next(&mut args, "--endpoint")?),
            "--model" => options.model = next(&mut args, "--model")?,
            "--api-key" => options.api_key = Some(next(&mut args, "--api-key")?),
            "--max-rounds" => {
                options.max_rounds = next(&mut args, "--max-rounds")?
                    .parse()
                    .map_err(|error| format!("invalid --max-rounds: {error}"))?
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(Some(options))
}

fn next(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn build_model(options: &Options) -> Box<dyn Model> {
    if let Some(script) = &options.script {
        match ScriptModel::from_file(script) {
            Ok(model) => return Box::new(model),
            Err(error) => {
                eprintln!("aether-chat: {error}");
                std::process::exit(1);
            }
        }
    }
    if let Some(endpoint) = &options.endpoint {
        match OpenAiModel::new(endpoint, &options.model, options.api_key.clone()) {
            Ok(model) => return Box::new(model),
            Err(error) => {
                eprintln!("aether-chat: {error}");
                std::process::exit(1);
            }
        }
    }
    Box::new(UnavailableModel)
}

struct UnavailableModel;

impl Model for UnavailableModel {
    fn complete(&mut self, _messages: &[Value], _tools: &[Value]) -> Result<model::Turn, String> {
        Err(
            "no model configured: set AETHER_CHAT_ENDPOINT (OpenAI-compatible) or \
             AETHER_CHAT_SCRIPT"
                .into(),
        )
    }
}

fn serve_socket(path: &Path, agent: &mut Agent) {
    loop {
        match UnixStream::connect(path) {
            Ok(stream) => {
                eprintln!("aether-chat: connected to {}", path.display());
                let Ok(mut writer) = stream.try_clone() else {
                    continue;
                };
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    if let Some(prompt) = parse_prompt_line(&line) {
                        run_prompt(agent, &prompt, &mut writer);
                    }
                }
                eprintln!("aether-chat: shell disconnected; retrying");
            }
            Err(_) => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

fn run_prompt(agent: &mut Agent, prompt: &str, writer: &mut impl Write) {
    let mut emit = |event: Event| {
        let _ = writeln!(writer, "{}", event.to_line());
        let _ = writer.flush();
    };
    if let Err(error) = agent.prompt(prompt, &mut emit) {
        eprintln!("aether-chat: {error}");
    }
    agent.trim_history();
}

fn mcp_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("AETHER_MCP_BIN") {
        return PathBuf::from(path);
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(directory) = current.parent() {
            let sibling = directory.join("aether-mcp");
            if sibling.exists() {
                return sibling;
            }
            if let Some(parent) = directory.parent() {
                let sibling = parent.join("aether-mcp");
                if sibling.exists() {
                    return sibling;
                }
            }
        }
    }
    PathBuf::from("aether-mcp")
}
