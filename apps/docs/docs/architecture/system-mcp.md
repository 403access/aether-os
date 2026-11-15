---
id: system-mcp
title: System MCP
sidebar_position: 12
description: How Aether exposes the running system to models through MCP, and how the chat drawer uses it.
---

# System MCP

Aether ships an MCP server, `aether-mcp`, whose tools operate on the running
machine: windows, workspaces, applications, files, processes, audio and
notifications. The built-in chat uses the same server through `aether-chat`, so
"open the audio panel" and "what is using memory?" take the same audited path
whether the prompt comes from the drawer or an external MCP client.

```text
chat drawer ── AETHER_CHAT_SOCKET ── aether-chat ── stdio ── aether-mcp
                                                               │
                       AETHER_CONTROL_SOCKET ──────────────────┤
                       AETHER_INDEX_SOCKET ────────────────────┤
                       /proc, ps, wpctl, notify-send ───────────┘
```

The shell never parses model output. `aether-chat` owns the model loop and the
MCP client; the drawer is a non-blocking view that streams tokens and tool
events. `aether-mcp` is transport-only, which is why the same binary works for
opencode, an agent, or the chat backend.

## Tools

| Tool | Kind | What it does |
| --- | --- | --- |
| `aether_system` | read | Uptime, load, memory, CPU count, process count, compositor status |
| `aether_processes` | read | `ps` sample, sorted by CPU, memory or pid |
| `aether_files_search` | read | Index-daemon query, bounded walk fallback |
| `aether_apps` | read | Discovered XDG applications (macOS applications on a dev host) |
| `aether_windows` | read | Window id, title, app id, workspace, geometry, focus |
| `aether_workspaces` | read | Names and active workspace |
| `aether_audio` | read/write | Route status, default sink volume |
| `aether_launch_app` | action | Launch a discovered application |
| `aether_focus_window` | action | Focus a window and reveal its workspace |
| `aether_close_window` | destructive | Ask a window to close; requires `confirm: true` |
| `aether_switch_workspace` | action | Switch the primary output |
| `aether_open_surface` | action | Open palette, chat, explorer, audio, task manager or shortcuts |
| `aether_notify` | action | Desktop notification |

Tools are annotated (`readOnlyHint`, `destructiveHint`) so clients can ask for
consent before actions. There is no "run command" tool by design; system changes
go through the shell's own action surface.

## Control socket

`aether-mcp` talks to the compositor over `AETHER_CONTROL_SOCKET` (default
`$XDG_RUNTIME_DIR/aether-control.sock`). The shell binds the socket, accepts one
newline-delimited JSON request per connection, executes it on the compositor
thread and returns a JSON-RPC-shaped result:

```json
{"id":1,"method":"windows.list","params":{}}
{"jsonrpc":"2.0","id":1,"result":{"windows":[...]}}
```

Methods: `status`, `windows.list`, `windows.focus`, `windows.close`,
`workspaces.list`, `workspaces.switch`, `apps.list`, `apps.launch`,
`surface.open`, `chat.prompt`.

Requests are queued by a listener thread and drained with a per-frame budget, so
a stalled client cannot block presentation. The socket lives in the user runtime
directory and is therefore reachable only by the session user.

## Chat backend

`aether-chat` connects to `AETHER_CHAT_SOCKET`, reads prompts, and writes events:

```json
{"type":"token","text":"The compositor is connected. "}
{"type":"tool","name":"aether_windows","status":"ok","detail":"3 windows"}
{"type":"done"}
```

It speaks an OpenAI-compatible chat-completions API. Point it at a local endpoint:

```bash
# ~/.config/aether/chat.env
AETHER_CHAT_ENDPOINT=http://127.0.0.1:11434/v1
AETHER_CHAT_MODEL=deepseek-chat
# AETHER_CHAT_API_KEY=...        only for a hosted endpoint
```

A Unix-socket endpoint is also accepted: `AETHER_CHAT_ENDPOINT=unix:/run/aether/inference.sock`.
No model weights ship with Aether. Without an endpoint the drawer reports the
missing configuration on the first prompt instead of failing silently.

For deterministic demos and tests, `AETHER_CHAT_SCRIPT` selects a scripted model:

```json
{"turns":[
  {"content":"Checking.","tool_calls":[{"name":"aether_system","arguments":{}}]},
  {"content":"The compositor is {{last_tool}}"}
]}
```

## Client setup

Any MCP client can spawn the server over stdio:

```json
{
  "mcp": {
    "servers": {
      "aether": {
        "type": "local",
        "command": ["aether-mcp"]
      }
    }
  }
}
```

With opencode: `opencode mcp add aether -- aether-mcp`. From a development
checkout, `opencode mcp add aether -- "$PWD/scripts/mcp-serve.sh"` builds the
current tree and keeps build output off the MCP channel. Then a prompt such as
"list the open windows and focus the terminal" becomes an `aether_windows` call
followed by `aether_focus_window`.

## Environment

| Variable | Used by | Meaning |
| --- | --- | --- |
| `AETHER_CONTROL_SOCKET` | shell, MCP | Compositor control socket |
| `AETHER_INDEX_SOCKET` | index, MCP | Index daemon query socket |
| `AETHER_CHAT_SOCKET` | shell, chat | Chat event/prompt socket |
| `AETHER_MCP_BIN` | chat | Path to `aether-mcp` |
| `AETHER_CHAT_ENDPOINT` | chat | OpenAI-compatible endpoint |
| `AETHER_CHAT_MODEL` | chat | Model name |
| `AETHER_CHAT_API_KEY` | chat | Bearer token for hosted inference |
| `AETHER_CHAT_SCRIPT` | chat | Scripted model JSON (tests/demos) |
| `AETHER_CHAT_MAX_ROUNDS` | chat | Tool rounds per prompt (default 6) |
| `AETHER_MCP_SEARCH_ROOTS` | MCP | Fallback file-search roots |
| `AETHER_MCP_AUDIT` | MCP | Audit log path (default `~/.local/state/aether/mcp-audit.jsonl`) |

## Trust boundary

- **Read tools** are unrestricted; they only report state.
- **Actions** are limited to the shell's own surfaces. Arbitrary program
  execution is not exposed.
- **Destructive actions** require `confirm: true`; today that is closing a
  window.
- **Audit:** every call appends one JSON line with the tool, arguments, outcome
  and timestamp; the file rotates once at 1 MiB. Prompt and notification text is
  redacted; only its length is recorded.
- **Socket permissions:** the control and chat sockets inherit the session
  runtime directory (`0700`), so another local user cannot connect.
- **Model trust:** the model is treated as untrusted input. It cannot bypass the
  confirmation check or reach a control method that is not in the table.
- **No self-prompting:** the chat is not exposed as a tool. A model cannot ask
  the built-in chat to run another prompt, which would allow unbounded nesting.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Compositor not running | Read tools still work; window tools return "compositor is not reachable" |
| Control request stalls | Three-second client timeout; shell keeps rendering |
| Chat backend absent | Drawer echoes the prompt and reports that the backend is not connected |
| Model endpoint absent | `error` event with the missing configuration |
| Model loops on tools | Stops after `AETHER_CHAT_MAX_ROUNDS` and reports the limit |
| Audit file unwritable | The tool call still runs; the audit error is ignored |
| Malformed MCP line | JSON-RPC `-32700` parse error with a null id |
| Unknown tool | JSON-RPC `-32602`; a tool's own failure is `isError: true` content |

## Not implemented

- Cancellation of a running prompt or tool call.
- Streaming token passthrough from the model (the backend emits 48-character
  chunks after a complete response).
- Remote MCP transport. The server is stdio-only; `--socket` exists for local
  supervisors, not the network.
