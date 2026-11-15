//! End-to-end MCP stdio transport test.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn exchange(child: &mut std::process::Child, request: Value) -> Value {
    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, "{request}").unwrap();
    stdin.flush().unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    serde_json::from_str(line.trim()).unwrap()
}

#[test]
fn speaks_mcp_over_stdio_and_reports_host_state() {
    let audit = std::env::temp_dir().join(format!("aether-mcp-{}.jsonl", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_aether-mcp"))
        .env("AETHER_CONTROL_SOCKET", "/nonexistent/aether-control.sock")
        .env("AETHER_INDEX_SOCKET", "/nonexistent/aether-index.sock")
        .env("AETHER_MCP_AUDIT", &audit)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let init = exchange(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-06-18"},
        }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "aether-mcp");

    let list = exchange(
        &mut child,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let tools = list["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 10);

    let status = exchange(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "aether_system", "arguments": {}},
        }),
    );
    assert_eq!(status["result"]["isError"], false);
    let structured = &status["result"]["structuredContent"];
    assert!(structured["platform"].is_string());
    assert_eq!(structured["compositor"]["connected"], false);

    let unknown = exchange(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": "rm_rf", "arguments": {}},
        }),
    );
    assert_eq!(unknown["error"]["code"], -32602);

    drop(child.stdin.take());
    let _ = child.wait();
    let audit_text = std::fs::read_to_string(&audit).unwrap_or_default();
    assert!(audit_text.contains("aether_system"));
    let _ = std::fs::remove_file(&audit);
}
