//! Tool catalog and JSON schemas.

use serde_json::{json, Value};

pub fn names() -> &'static [&'static str] {
    &[
        "aether_system",
        "aether_processes",
        "aether_files_search",
        "aether_apps",
        "aether_windows",
        "aether_workspaces",
        "aether_audio",
        "aether_launch_app",
        "aether_focus_window",
        "aether_close_window",
        "aether_switch_workspace",
        "aether_open_surface",
        "aether_notify",
    ]
}

pub fn exists(name: &str) -> bool {
    names().contains(&name)
}

/// MCP tool descriptors. Annotations let clients distinguish reads from
/// actions without parsing descriptions.
pub fn catalog() -> Vec<Value> {
    vec![
        tool(
            "aether_system",
            "Aether system status",
            "Host uptime, load, memory, CPU count, process count and whether the \
             compositor control socket is connected.",
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_processes",
            "List processes",
            "Processes sampled with ps(1). Sort by cpu, memory or pid. Read-only.",
            json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "minimum": 1, "maximum": 500, "default": 20},
                    "sort": {"type": "string", "enum": ["cpu", "memory", "pid"], "default": "cpu"},
                    "query": {"type": "string", "description": "Case-insensitive name or command filter"}
                },
                "additionalProperties": false
            }),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_files_search",
            "Search indexed files",
            "Query the Aether index daemon when it is running, otherwise a bounded \
             filename walk below the configured roots. Read-only.",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 1},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 10}
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_apps",
            "List applications",
            "Installed XDG applications, optionally filtered by name or program.",
            json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "additionalProperties": false
            }),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_windows",
            "List windows",
            "Open windows with id, title, app id, workspace, geometry and focus. \
             Requires the compositor control socket.",
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_workspaces",
            "List workspaces",
            "Workspace names and the active workspace index.",
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            json!({"readOnlyHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_audio",
            "Audio routes and volume",
            "Read the current PipeWire/WirePlumber routes, or set the default sink \
             volume with action=volume. Volume changes are reversible.",
            json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["status", "volume"], "default": "status"},
                    "percent": {"type": "integer", "minimum": 0, "maximum": 150}
                },
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
        tool(
            "aether_launch_app",
            "Launch an application",
            "Launch a discovered application by name or program. The application \
             starts inside the active Aether session.",
            json!({
                "type": "object",
                "properties": {
                    "app": {"type": "string", "description": "Exact application name or program"}
                },
                "required": ["app"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
        tool(
            "aether_focus_window",
            "Focus a window",
            "Focus an open window by id and switch to its workspace when needed.",
            json!({
                "type": "object",
                "properties": {"id": {"type": "integer", "minimum": 1}},
                "required": ["id"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
        tool(
            "aether_close_window",
            "Close a window",
            "Ask a window to close. Destructive: requires confirm=true, and the \
             client may still refuse to exit.",
            json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "minimum": 1},
                    "confirm": {"type": "boolean", "default": false}
                },
                "required": ["id", "confirm"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}),
        ),
        tool(
            "aether_switch_workspace",
            "Switch workspace",
            "Switch the primary output to a zero-based workspace index.",
            json!({
                "type": "object",
                "properties": {"index": {"type": "integer", "minimum": 0}},
                "required": ["index"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
        tool(
            "aether_open_surface",
            "Open a shell surface",
            "Open a first-party shell surface: palette, chat, explorer, audio, \
             task_manager or shortcuts.",
            json!({
                "type": "object",
                "properties": {
                    "surface": {
                        "type": "string",
                        "enum": ["palette", "chat", "explorer", "audio", "task_manager", "shortcuts"]
                    }
                },
                "required": ["surface"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
        tool(
            "aether_notify",
            "Show a notification",
            "Deliver a desktop notification through the session notifier.",
            json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string", "minLength": 1},
                    "body": {"type": "string"}
                },
                "required": ["title"],
                "additionalProperties": false
            }),
            json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}),
        ),
    ]
}

fn tool(name: &str, title: &str, description: &str, schema: Value, annotations: Value) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": schema,
        "annotations": annotations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_has_exactly_one_descriptor() {
        let catalog = catalog();
        assert_eq!(catalog.len(), names().len());
        for name in names() {
            assert_eq!(
                catalog.iter().filter(|tool| tool["name"] == *name).count(),
                1,
                "{name}"
            );
        }
    }

    #[test]
    fn schemas_are_closed_objects() {
        for tool in catalog() {
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        }
    }
}
