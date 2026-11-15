//! Remote-session overlay state: pairing, transport, latency and the two
//! audited toggles (input and clipboard).
//!
//! Rendering and hit-testing live in the backend; this module owns policy so it
//! is testable without a display server and can be driven by `aether-remote`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteState {
    Waiting,
    Streaming,
    Closed,
}

impl RemoteState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for viewer",
            Self::Streaming => "Streaming",
            Self::Closed => "Closed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteAction {
    StopSession,
    ToggleInput,
    ToggleClipboard,
}

impl RemoteAction {
    pub const ALL: [Self; 3] = [Self::StopSession, Self::ToggleInput, Self::ToggleClipboard];

    pub fn label(self) -> &'static str {
        match self {
            Self::StopSession => "Stop session",
            Self::ToggleInput => "Toggle input",
            Self::ToggleClipboard => "Toggle clipboard",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteLayer {
    session_id: String,
    pairing_code: String,
    tier: String,
    codec: String,
    state: RemoteState,
    viewers: u8,
    latency_ms: Option<f32>,
    bitrate_kbps: u32,
    input_enabled: bool,
    clipboard_enabled: bool,
    software: bool,
    selected: usize,
}

impl RemoteLayer {
    pub fn waiting(pairing_code: impl Into<String>) -> Self {
        Self {
            session_id: "—".to_string(),
            pairing_code: pairing_code.into(),
            tier: "WebRTC".to_string(),
            codec: "H.264".to_string(),
            state: RemoteState::Waiting,
            viewers: 0,
            latency_ms: None,
            bitrate_kbps: 0,
            input_enabled: true,
            clipboard_enabled: false,
            software: true,
            selected: 0,
        }
    }

    /// Deterministic fixture used by the preview, the Xvfb smoke and the docs.
    pub fn demo() -> Self {
        Self {
            session_id: "rs-kestrel-7f3a".to_string(),
            pairing_code: "K7QM-2H4P".to_string(),
            tier: "WebRTC".to_string(),
            codec: "H.264".to_string(),
            state: RemoteState::Streaming,
            viewers: 1,
            latency_ms: Some(18.4),
            bitrate_kbps: 12_000,
            input_enabled: true,
            clipboard_enabled: false,
            software: true,
            selected: 0,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn set_session_id(&mut self, session_id: impl Into<String>) {
        self.session_id = session_id.into();
    }

    pub fn pairing_code(&self) -> &str {
        &self.pairing_code
    }

    pub fn tier(&self) -> &str {
        &self.tier
    }

    pub fn codec(&self) -> &str {
        &self.codec
    }

    pub fn state(&self) -> RemoteState {
        self.state
    }

    pub fn set_state(&mut self, state: RemoteState) {
        self.state = state;
    }

    pub fn viewers(&self) -> u8 {
        self.viewers
    }

    pub fn set_viewers(&mut self, viewers: u8) {
        self.viewers = viewers;
    }

    pub fn latency_ms(&self) -> Option<f32> {
        self.latency_ms
    }

    pub fn set_latency_ms(&mut self, latency_ms: Option<f32>) {
        self.latency_ms = latency_ms;
    }

    pub fn bitrate_kbps(&self) -> u32 {
        self.bitrate_kbps
    }

    pub fn set_bitrate_kbps(&mut self, bitrate_kbps: u32) {
        self.bitrate_kbps = bitrate_kbps;
    }

    pub fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    pub fn clipboard_enabled(&self) -> bool {
        self.clipboard_enabled
    }

    pub fn software(&self) -> bool {
        self.software
    }

    pub fn set_software(&mut self, software: bool) {
        self.software = software;
    }

    /// The recording indicator is visible for the whole session, including the
    /// waiting phase. There is no hidden capture.
    pub fn recording(&self) -> bool {
        self.state != RemoteState::Closed
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= RemoteAction::ALL.len() {
            return false;
        }
        self.selected = index;
        true
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        let len = RemoteAction::ALL.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
        true
    }

    pub fn selected_action(&self) -> RemoteAction {
        RemoteAction::ALL[self.selected.min(RemoteAction::ALL.len() - 1)]
    }

    /// Apply the selected action locally and return it so the backend can
    /// publish the change to the host service.
    pub fn activate(&mut self) -> Option<RemoteAction> {
        let action = self.selected_action();
        match action {
            RemoteAction::StopSession => {
                self.state = RemoteState::Closed;
                self.input_enabled = false;
                self.clipboard_enabled = false;
                self.latency_ms = None;
                self.bitrate_kbps = 0;
                Some(action)
            }
            RemoteAction::ToggleInput => {
                self.toggle_input();
                Some(action)
            }
            RemoteAction::ToggleClipboard => {
                self.toggle_clipboard();
                Some(action)
            }
        }
    }

    pub fn toggle_input(&mut self) -> bool {
        self.input_enabled = !self.input_enabled;
        self.input_enabled
    }

    pub fn toggle_clipboard(&mut self) -> bool {
        self.clipboard_enabled = !self.clipboard_enabled;
        self.clipboard_enabled
    }

    pub fn action_label(&self, index: usize) -> Option<String> {
        let action = RemoteAction::ALL.get(index)?;
        Some(match action {
            RemoteAction::ToggleInput => {
                format!("Input · {}", if self.input_enabled { "on" } else { "off" })
            }
            RemoteAction::ToggleClipboard => format!(
                "Clipboard · {}",
                if self.clipboard_enabled { "on" } else { "off" }
            ),
            RemoteAction::StopSession => action.label().to_string(),
        })
    }

    pub fn status_label(&self) -> Option<String> {
        match self.state {
            RemoteState::Closed => None,
            RemoteState::Waiting => Some("remote waiting".to_string()),
            RemoteState::Streaming => {
                let mut label = String::from("remote ");
                if let Some(latency) = self.latency_ms {
                    label.push_str(&format!("{latency:.1} ms · "));
                }
                label.push_str(&format!(
                    "{:.1} Mbps · {}",
                    self.bitrate_kbps as f32 / 1000.0,
                    self.codec
                ));
                if self.software {
                    label.push_str(" · software");
                }
                Some(label)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_waiting_and_records_from_the_start() {
        let mut layer = RemoteLayer::waiting("K7QM-2H4P");
        assert_eq!(layer.state(), RemoteState::Waiting);
        assert!(layer.recording());
        assert_eq!(layer.status_label().as_deref(), Some("remote waiting"));
        layer.set_state(RemoteState::Streaming);
        layer.set_latency_ms(Some(18.42));
        layer.set_bitrate_kbps(12_000);
        let status = layer.status_label().unwrap();
        assert!(status.contains("18.4 ms"));
        assert!(status.contains("12.0 Mbps"));
        assert!(status.contains("software"));
    }

    #[test]
    fn actions_toggle_input_and_clipboard() {
        let mut layer = RemoteLayer::demo();
        assert!(layer.input_enabled());
        assert!(!layer.clipboard_enabled());
        assert_eq!(layer.action_label(1).as_deref(), Some("Input · on"));
        layer.select(1);
        assert_eq!(layer.activate(), Some(RemoteAction::ToggleInput));
        assert!(!layer.input_enabled());
        layer.select(2);
        assert_eq!(layer.activate(), Some(RemoteAction::ToggleClipboard));
        assert!(layer.clipboard_enabled());
        assert_eq!(layer.action_label(2).as_deref(), Some("Clipboard · on"));
        assert!(layer.recording());
    }

    #[test]
    fn stop_closes_and_hides_the_status() {
        let mut layer = RemoteLayer::demo();
        layer.select(0);
        assert_eq!(layer.activate(), Some(RemoteAction::StopSession));
        assert_eq!(layer.state(), RemoteState::Closed);
        assert!(!layer.recording());
        assert!(layer.status_label().is_none());
        assert!(!layer.input_enabled());
    }

    #[test]
    fn selection_wraps_and_rejects_out_of_range() {
        let mut layer = RemoteLayer::demo();
        assert!(layer.move_selection(-1));
        assert_eq!(layer.selected(), 2);
        assert!(layer.move_selection(1));
        assert_eq!(layer.selected(), 0);
        assert!(!layer.select(3));
        assert_eq!(layer.selected(), 0);
    }
}
