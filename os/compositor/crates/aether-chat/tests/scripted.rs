//! Scripted end-to-end test: prompt -> model -> MCP tool -> streamed answer.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_binaries() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .args(["build", "-p", "aether-chat", "-p", "aether-mcp"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo");
        assert!(status.success(), "failed to build chat and MCP binaries");
    });
}

fn binary(name: &str) -> PathBuf {
    build_binaries();
    workspace_root().join("target/debug").join(name)
}

#[test]
fn streams_tool_calls_and_a_final_answer() {
    let script = std::env::temp_dir().join(format!("aether-chat-test-{}.json", std::process::id()));
    std::fs::write(
        &script,
        r#"{"turns":[
            {"content":"Checking.","tool_calls":[{"name":"aether_system","arguments":{}}]},
            {"content":"Tool result: {{last_tool}}"}
        ]}"#,
    )
    .unwrap();

    let mut child = Command::new(binary("aether-chat"))
        .args([
            "--stdio",
            "--mcp",
            binary("aether-mcp").to_str().unwrap(),
            "--script",
            script.to_str().unwrap(),
        ])
        .env("AETHER_CONTROL_SOCKET", "/nonexistent/control.sock")
        .env("AETHER_MCP_AUDIT", "/nonexistent/audit.jsonl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, r#"{{"type":"prompt","prompt":"status?"}}"#).unwrap();
    stdin.flush().unwrap();

    let mut events = Vec::new();
    for line in BufReader::new(child.stdout.as_mut().unwrap()).lines() {
        let Ok(line) = line else { break };
        let value: Value = serde_json::from_str(&line).unwrap();
        let done = value["type"] == "done";
        events.push(value);
        if done {
            break;
        }
    }
    drop(child.stdin.take());
    let _ = child.wait();
    let _ = std::fs::remove_file(&script);

    assert!(events.iter().any(|event| {
        event["type"] == "tool" && event["name"] == "aether_system" && event["status"] == "ok"
    }));
    let text: String = events
        .iter()
        .filter(|event| event["type"] == "token")
        .filter_map(|event| event["text"].as_str())
        .collect();
    assert!(text.contains("Checking."));
    assert!(text.contains("Tool result:"));
    assert_eq!(events.last().unwrap()["type"], "done");
}
