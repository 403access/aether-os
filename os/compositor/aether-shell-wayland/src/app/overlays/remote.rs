//! Remote-session overlay: status, pairing code and audited toggles.

use aether_shell::remote::{RemoteAction, RemoteLayer, RemoteState};
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_remote(&mut self) {
        if self.shell.remote_open() {
            self.shell.close_remote();
            self.mark_full_damage();
            return;
        }
        self.shortcuts_open = false;
        self.close_task_manager();
        self.shell.close_overlay();
        self.shell.close_explorer();
        self.shell.close_chat();
        self.shell.close_audio();
        let layer = read_layer();
        println!(
            "aether-shell: remote overlay opened ({}, {})",
            layer.session_id(),
            layer.state().label()
        );
        self.shell.open_remote(layer);
        self.mark_full_damage();
    }

    pub(crate) fn handle_remote_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let input: Keysym = keysyms::KEY_i.into();
        let clipboard: Keysym = keysyms::KEY_c.into();
        let stop: Keysym = keysyms::KEY_s.into();
        if keysym == escape {
            self.shell.close_remote();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.remote_move_selection(-1);
            self.mark_remote_damage();
        } else if keysym == down {
            self.shell.remote_move_selection(1);
            self.mark_remote_damage();
        } else if keysym == enter {
            self.activate_remote();
            self.mark_remote_damage();
        } else if keysym == input {
            if let Some(layer) = self.shell.remote_layer_mut() {
                let enabled = layer.toggle_input();
                println!("aether-shell: remote input -> {enabled}");
            }
            self.mark_remote_damage();
        } else if keysym == clipboard {
            if let Some(layer) = self.shell.remote_layer_mut() {
                let enabled = layer.toggle_clipboard();
                println!("aether-shell: remote clipboard -> {enabled}");
            }
            self.mark_remote_damage();
        } else if keysym == stop {
            self.shell.remote_select(0);
            self.activate_remote();
            self.mark_remote_damage();
        }
    }

    pub(crate) fn activate_remote(&mut self) {
        let Some(action) = self.shell.remote_activate() else {
            return;
        };
        match action {
            RemoteAction::StopSession => println!("aether-shell: remote session stopped"),
            RemoteAction::ToggleInput => {
                let enabled = self
                    .shell
                    .remote_layer()
                    .is_some_and(|layer| layer.input_enabled());
                println!("aether-shell: remote input -> {enabled}");
            }
            RemoteAction::ToggleClipboard => {
                let enabled = self
                    .shell
                    .remote_layer()
                    .is_some_and(|layer| layer.clipboard_enabled());
                println!("aether-shell: remote clipboard -> {enabled}");
            }
        }
    }

    pub(crate) fn mark_remote_damage(&mut self) {
        if self.shell.remote_open() {
            let rect = self.shell.remote_rect();
            self.mark_damage_rect(rect);
        }
    }
}

/// The host service publishes a small JSON status document. The Xvfb smoke and
/// the docs set `AETHER_REMOTE_FIXTURE` to a deterministic layer instead.
fn read_layer() -> RemoteLayer {
    if std::env::var_os("AETHER_REMOTE_FIXTURE").is_some() {
        return RemoteLayer::demo();
    }
    if let Some(layer) = read_status_file() {
        return layer;
    }
    RemoteLayer::waiting("----")
}

fn read_status_file() -> Option<RemoteLayer> {
    let path = std::env::var("AETHER_REMOTE_STATUS").ok().or_else(|| {
        std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .map(|dir| format!("{dir}/aether/remote-session.json"))
    })?;
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut layer = RemoteLayer::waiting(
        value
            .get("pairing_code")
            .and_then(|value| value.as_str())
            .unwrap_or("----"),
    );
    if let Some(session) = value.get("session_id").and_then(|value| value.as_str()) {
        layer.set_session_id(session);
    }
    if let Some(state) = value.get("state").and_then(|value| value.as_str()) {
        layer.set_state(match state {
            "streaming" => RemoteState::Streaming,
            "closed" => RemoteState::Closed,
            _ => RemoteState::Waiting,
        });
    }
    if let Some(latency) = value.get("latency_ms").and_then(|value| value.as_f64()) {
        layer.set_latency_ms(Some(latency as f32));
    }
    if let Some(bitrate) = value.get("bitrate_kbps").and_then(|value| value.as_u64()) {
        layer.set_bitrate_kbps(bitrate as u32);
    }
    if let Some(viewers) = value.get("viewers").and_then(|value| value.as_u64()) {
        layer.set_viewers(viewers as u8);
    }
    if let Some(software) = value.get("software").and_then(|value| value.as_bool()) {
        layer.set_software(software);
    }
    Some(layer)
}
