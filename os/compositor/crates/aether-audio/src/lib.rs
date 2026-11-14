//! PipeWire route model shared by the CLI, the shell overlay and tests.
//!
//! The model deliberately contains no PipeWire types. Adapters live in
//! [`pipewire`] and convert `pw-dump`/`wpctl` output into these values, so the
//! policies and overlay can be tested without audio hardware.

use serde::{Deserialize, Serialize};

pub mod pipewire;

pub use pipewire::{read_fixture, SystemAudio};

/// Scope an agent must hold before opening a capture stream.
pub const AUDIO_CAPTURE_SCOPE: &str = "audio.capture";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteKind {
    Output,
    Input,
}

impl RouteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Input => "Input",
        }
    }

    pub fn from_media_class(class: &str) -> Option<Self> {
        if class.starts_with("Audio/Sink") {
            Some(Self::Output)
        } else if class.starts_with("Audio/Source") {
            Some(Self::Input)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Analog,
    Usb,
    Bluetooth,
    Hdmi,
    Virtual,
    Unknown,
}

impl Transport {
    pub fn label(self) -> &'static str {
        match self {
            Self::Analog => "Analog",
            Self::Usb => "USB",
            Self::Bluetooth => "Bluetooth",
            Self::Hdmi => "HDMI",
            Self::Virtual => "Virtual",
            Self::Unknown => "Unknown",
        }
    }

    /// Resolve from `pw-dump` properties without trusting `node.name` alone.
    pub fn detect(api: Option<&str>, bus: Option<&str>, name: &str) -> Self {
        let api = api.unwrap_or_default().to_ascii_lowercase();
        let bus = bus.unwrap_or_default().to_ascii_lowercase();
        let name = name.to_ascii_lowercase();
        if api.contains("bluez") || bus.contains("bluetooth") || name.contains("bluez") {
            Self::Bluetooth
        } else if api.contains("alsa") && (bus.contains("usb") || name.contains("usb")) {
            Self::Usb
        } else if name.contains("hdmi") || name.contains("displayport") {
            Self::Hdmi
        } else if api.contains("alsa") {
            Self::Analog
        } else if api.contains("jack") || api.contains("pipewire") || name.contains("virtual") {
            Self::Virtual
        } else {
            Self::Unknown
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioRoute {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub kind: RouteKind,
    pub transport: Transport,
    pub profile: String,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub volume: Option<u8>,
    /// Set when a client is pinned to a non-default route.
    #[serde(default)]
    pub pinned: bool,
}

fn default_true() -> bool {
    true
}

impl AudioRoute {
    pub fn label(&self) -> &str {
        if self.description.is_empty() {
            &self.name
        } else {
            &self.description
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatencyPreset {
    /// 1024 frames: media playback, power efficient.
    Media,
    /// 256 frames: voice and calls, < 20 ms added latency.
    Voice,
    /// 128 frames: monitoring, opt-in.
    Monitor,
}

impl LatencyPreset {
    pub const ALL: [Self; 3] = [Self::Media, Self::Voice, Self::Monitor];

    pub fn name(self) -> &'static str {
        match self {
            Self::Media => "media",
            Self::Voice => "voice",
            Self::Monitor => "monitor",
        }
    }

    pub fn quantum(self) -> u32 {
        match self {
            Self::Media => 1024,
            Self::Voice => 256,
            Self::Monitor => 128,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Media => "1024 frames · playback",
            Self::Voice => "256 frames · voice",
            Self::Monitor => "128 frames · monitoring",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Media => Self::Voice,
            Self::Voice => Self::Monitor,
            Self::Monitor => Self::Media,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.name().eq_ignore_ascii_case(value.trim()))
    }

    pub fn from_quantum(quantum: u32) -> Self {
        if quantum <= 128 {
            Self::Monitor
        } else if quantum <= 256 {
            Self::Voice
        } else {
            Self::Media
        }
    }
}

impl Default for LatencyPreset {
    fn default() -> Self {
        Self::Media
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioStream {
    pub index: u32,
    pub sink: u32,
    pub app: String,
    #[serde(default)]
    pub media: Option<String>,
}

/// Parse `pactl --format=json list sink-inputs` into per-app streams.
pub fn parse_pactl_streams(input: &str) -> Result<Vec<AudioStream>, AudioError> {
    let root: serde_json::Value =
        serde_json::from_str(input).map_err(|error| AudioError::Parse(error.to_string()))?;
    let entries = root
        .as_array()
        .ok_or_else(|| AudioError::Parse("pactl root is not an array".into()))?;
    let mut streams = Vec::new();
    for entry in entries {
        let Some(index) = entry.get("index").and_then(|value| value.as_u64()) else {
            continue;
        };
        let Some(sink) = entry.get("sink").and_then(|value| value.as_u64()) else {
            continue;
        };
        let properties = entry.get("properties");
        let app = properties
            .and_then(|value| value.get("application.name"))
            .and_then(|value| value.as_str())
            .or_else(|| entry.get("name").and_then(|value| value.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let media = properties
            .and_then(|value| value.get("media.name"))
            .and_then(|value| value.as_str())
            .map(str::to_string);
        streams.push(AudioStream {
            index: index as u32,
            sink: sink as u32,
            app,
            media,
        });
    }
    Ok(streams)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioSnapshot {
    pub routes: Vec<AudioRoute>,
    pub preset: LatencyPreset,
    #[serde(default)]
    pub xruns: u64,
    #[serde(default)]
    pub source: SnapshotSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotSource {
    #[default]
    Live,
    Fixture,
}

impl AudioSnapshot {
    pub fn from_routes(routes: Vec<AudioRoute>) -> Self {
        let mut snapshot = Self {
            routes,
            preset: LatencyPreset::default(),
            xruns: 0,
            source: SnapshotSource::Live,
        };
        snapshot.ensure_defaults();
        snapshot
    }

    pub fn outputs(&self) -> impl Iterator<Item = &AudioRoute> {
        self.routes
            .iter()
            .filter(|route| route.kind == RouteKind::Output)
    }

    pub fn inputs(&self) -> impl Iterator<Item = &AudioRoute> {
        self.routes
            .iter()
            .filter(|route| route.kind == RouteKind::Input)
    }

    pub fn default_route(&self, kind: RouteKind) -> Option<&AudioRoute> {
        self.routes
            .iter()
            .find(|route| route.kind == kind && route.default)
            .or_else(|| self.routes.iter().find(|route| route.kind == kind))
    }

    pub fn mark_default(&mut self, kind: RouteKind, id: u32) -> bool {
        let exists = self
            .routes
            .iter()
            .any(|route| route.id == id && route.kind == kind);
        if !exists {
            return false;
        }
        for route in self.routes.iter_mut().filter(|route| route.kind == kind) {
            route.default = route.id == id;
        }
        true
    }

    /// Every route kind must have exactly one default when at least one
    /// available route exists.
    pub fn ensure_defaults(&mut self) {
        for kind in [RouteKind::Output, RouteKind::Input] {
            let candidates: Vec<(usize, bool, bool)> = self
                .routes
                .iter()
                .enumerate()
                .filter(|(_, route)| route.kind == kind)
                .map(|(index, route)| (index, route.available, route.default))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            if let Some((index, _, _)) = candidates.iter().find(|(_, _, default)| *default) {
                let selected = self.routes[*index].id;
                for route in self.routes.iter_mut().filter(|route| route.kind == kind) {
                    route.default = route.id == selected;
                }
                continue;
            }
            let selected = candidates
                .iter()
                .find(|(_, available, _)| *available)
                .or_else(|| candidates.first())
                .map(|(index, _, _)| *index);
            let Some(index) = selected else {
                continue;
            };
            let id = self.routes[index].id;
            for route in self.routes.iter_mut().filter(|route| route.kind == kind) {
                route.default = route.id == id;
            }
        }
    }

    pub fn cycle_preset(&mut self) -> LatencyPreset {
        self.preset = self.preset.next();
        self.preset
    }
}

/// Parse `pw-dump` JSON (a JSON array of PipeWire objects) into routes.
pub fn parse_pw_dump(input: &str) -> Result<Vec<AudioRoute>, AudioError> {
    let root: serde_json::Value =
        serde_json::from_str(input).map_err(|error| AudioError::Parse(error.to_string()))?;
    let objects = root
        .as_array()
        .ok_or_else(|| AudioError::Parse("pw-dump root is not an array".into()))?;
    let mut routes = Vec::new();
    for object in objects {
        routes.extend(route_from_object(object));
    }
    Ok(routes)
}

fn route_from_object(object: &serde_json::Value) -> Option<AudioRoute> {
    if object.get("type")?.as_str()? != "PipeWire:Interface:Node" {
        return None;
    }
    let info = object.get("info")?;
    let props = info.get("props").unwrap_or(&serde_json::Value::Null);
    let class = props.get("media.class")?.as_str()?;
    let kind = RouteKind::from_media_class(class)?;
    let id = object.get("id")?.as_u64()? as u32;
    let name = props
        .get("node.name")
        .and_then(|value| value.as_str())
        .unwrap_or("node")
        .to_string();
    let description = props
        .get("node.description")
        .or_else(|| props.get("node.nick"))
        .and_then(|value| value.as_str())
        .unwrap_or(&name)
        .to_string();
    let api = props.get("device.api").and_then(|value| value.as_str());
    let bus = props.get("device.bus").and_then(|value| value.as_str());
    let transport = Transport::detect(api, bus, &name);
    let profile = props
        .get("device.profile.name")
        .or_else(|| props.get("api.bluez5.profile"))
        .or_else(|| props.get("node.profile"))
        .and_then(|value| value.as_str())
        .unwrap_or("default")
        .to_string();
    let codec = props
        .get("api.bluez5.codec")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_uppercase());
    Some(AudioRoute {
        id,
        name,
        description,
        kind,
        transport,
        profile,
        codec,
        default: false,
        available: info
            .get("state")
            .and_then(|value| value.as_str())
            .is_none_or(|state| state != "error"),
        muted: false,
        volume: None,
        pinned: false,
    })
}

/// Parse `wpctl inspect @DEFAULT_AUDIO_SINK@` style output for the numeric id.
pub fn parse_wpctl_id(output: &str) -> Option<u32> {
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("id ") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|value| value.trim_end_matches(',').parse().ok());
        }
    }
    None
}

/// Sum the `ERR` column from `pw-top -b -n 1` output.
///
/// Batch rows start with a state character (`S`/`I`/`E`) and carry numeric
/// columns. Non-numeric headers and notes are ignored.
pub fn parse_pw_top_xruns(output: &str) -> u64 {
    let mut total = 0u64;
    for line in output.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let Some(state) = fields[0].chars().next() else {
            continue;
        };
        if !matches!(state, 'S' | 'I' | 'E' | 'D') {
            continue;
        }
        // Columns: state id quantum rate wait busy w/q b/q err format…
        let Ok(error) = fields[8].parse::<f64>() else {
            continue;
        };
        if error > 0.0 {
            total = total.saturating_add(1);
        }
    }
    total
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioError {
    Parse(String),
    Command(String),
    Unavailable(String),
    NotFound(u32),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(message) => write!(formatter, "audio parse error: {message}"),
            Self::Command(message) => write!(formatter, "audio command failed: {message}"),
            Self::Unavailable(message) => write!(formatter, "audio unavailable: {message}"),
            Self::NotFound(id) => write!(formatter, "audio route {id} not found"),
        }
    }
}

impl std::error::Error for AudioError {}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = r#"[
      {"id": 42, "type": "PipeWire:Interface:Node", "info": {"state": "running", "props": {
        "media.class": "Audio/Sink", "node.name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
        "node.description": "Speakers · Analog", "device.api": "alsa", "device.bus": "pci",
        "device.profile.name": "analog-stereo"}}},
      {"id": 43, "type": "PipeWire:Interface:Node", "info": {"state": "running", "props": {
        "media.class": "Audio/Sink", "node.name": "alsa_output.usb-headset",
        "node.description": "USB Headset", "device.api": "alsa", "device.bus": "usb",
        "device.profile.name": "analog-stereo"}}},
      {"id": 44, "type": "PipeWire:Interface:Node", "info": {"state": "running", "props": {
        "media.class": "Audio/Sink", "node.name": "bluez_output.AC_80_0A",
        "node.description": "AirPods Pro", "device.api": "bluez5", "api.bluez5.profile": "a2dp-sink",
        "api.bluez5.codec": "ldac"}}},
      {"id": 51, "type": "PipeWire:Interface:Node", "info": {"state": "running", "props": {
        "media.class": "Audio/Source", "node.name": "alsa_input.usb-mic",
        "node.description": "USB Microphone", "device.api": "alsa", "device.bus": "usb"}}},
      {"id": 90, "type": "PipeWire:Interface:Client", "info": {"props": {}}}
    ]"#;

    #[test]
    fn parses_sinks_and_sources_with_transport() {
        let routes = parse_pw_dump(DUMP).unwrap();
        assert_eq!(routes.len(), 4);
        assert_eq!(routes[0].kind, RouteKind::Output);
        assert_eq!(routes[0].transport, Transport::Analog);
        assert_eq!(routes[1].transport, Transport::Usb);
        assert_eq!(routes[2].transport, Transport::Bluetooth);
        assert_eq!(routes[2].codec.as_deref(), Some("LDAC"));
        assert_eq!(routes[3].kind, RouteKind::Input);
        assert_eq!(routes[3].transport, Transport::Usb);
    }

    #[test]
    fn defaults_are_stable_and_exactly_one_per_kind() {
        let routes = parse_pw_dump(DUMP).unwrap();
        let mut snapshot = AudioSnapshot::from_routes(routes);
        assert_eq!(snapshot.default_route(RouteKind::Output).unwrap().id, 42);
        assert!(snapshot.mark_default(RouteKind::Output, 44));
        assert_eq!(snapshot.outputs().filter(|route| route.default).count(), 1);
        assert_eq!(snapshot.default_route(RouteKind::Output).unwrap().id, 44);
        assert!(!snapshot.mark_default(RouteKind::Output, 51));
        assert_eq!(snapshot.default_route(RouteKind::Input).unwrap().id, 51);
    }

    #[test]
    fn presets_map_to_quanta_and_cycle() {
        assert_eq!(LatencyPreset::Media.quantum(), 1024);
        assert_eq!(LatencyPreset::Voice.quantum(), 256);
        assert_eq!(LatencyPreset::Monitor.quantum(), 128);
        assert_eq!(LatencyPreset::Voice.next(), LatencyPreset::Monitor);
        assert_eq!(LatencyPreset::from_quantum(64), LatencyPreset::Monitor);
        assert_eq!(LatencyPreset::from_quantum(512), LatencyPreset::Media);
        assert_eq!(LatencyPreset::parse("VOICE"), Some(LatencyPreset::Voice));
    }

    #[test]
    fn parses_wpctl_default_id() {
        let text = "id 44, type PipeWire:Interface:Node\n  * media.class = \"Audio/Sink\"\n";
        assert_eq!(parse_wpctl_id(text), Some(44));
    }

    #[test]
    fn parses_pactl_streams() {
        let text = r#"[
          {"index": 7, "sink": 43, "properties": {"application.name": "Firefox", "media.name": "Video"}},
          {"index": 8, "sink": 42, "properties": {"application.name": "Music"}},
          {"index": 9}
        ]"#;
        let streams = parse_pactl_streams(text).unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].app, "Firefox");
        assert_eq!(streams[0].sink, 43);
        assert_eq!(streams[0].media.as_deref(), Some("Video"));
        assert_eq!(streams[1].app, "Music");
        assert_eq!(streams[1].media, None);
    }

    #[test]
    fn counts_nonzero_xruns() {
        let text = "\
S  29 1024 48000 0.03 0.01 0.001 0.0005 0.000000 S16LE
S  30  256 48000 0.04 0.02 0.001 0.0005 2.000000 S16LE
I  31  256 48000 0.04 0.02 0.001 0.0005 0.000000 S16LE
";
        assert_eq!(parse_pw_top_xruns(text), 1);
    }
}
