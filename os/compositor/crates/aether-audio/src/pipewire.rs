//! System adapter for PipeWire/WirePlumber.
//!
//! Everything here is process-based on purpose: the shell can be built and
//! tested without linking libpipewire, and the policy remains inspectable with
//! `aether-audio list`. Tests use [`crate::parse_pw_dump`] and a fixture file
//! instead of spawning audio daemons.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    parse_pactl_streams, parse_pw_dump, parse_pw_top_xruns, parse_wpctl_id, AudioError, AudioRoute,
    AudioSnapshot, AudioStream, LatencyPreset, RouteKind, SnapshotSource,
};

#[derive(Clone, Debug)]
pub struct SystemAudio {
    fixture: Option<PathBuf>,
    state_path: PathBuf,
}

impl SystemAudio {
    pub fn new() -> Self {
        Self {
            fixture: std::env::var_os("AETHER_AUDIO_FIXTURE").map(PathBuf::from),
            state_path: state_path(),
        }
    }

    pub fn with_fixture(path: impl Into<PathBuf>) -> Self {
        Self {
            fixture: Some(path.into()),
            state_path: std::env::temp_dir().join("aether-audio-preset"),
        }
    }

    pub fn snapshot(&self) -> Result<AudioSnapshot, AudioError> {
        if let Some(path) = &self.fixture {
            return read_fixture(path);
        }
        let dump = command_stdout("pw-dump", &[])?;
        let mut routes = parse_pw_dump(&dump)?;
        apply_system_defaults(&mut routes)?;
        let preset = self.read_preset().unwrap_or_default();
        let xruns = command_stdout("pw-top", &["-b", "-n", "1"])
            .map(|output| parse_pw_top_xruns(&output))
            .unwrap_or(0);
        let mut snapshot = AudioSnapshot {
            routes,
            preset,
            xruns,
            source: SnapshotSource::Live,
        };
        snapshot.ensure_defaults();
        Ok(snapshot)
    }

    pub fn switch(&self, kind: RouteKind, id: u32) -> Result<(), AudioError> {
        if self.fixture.is_some() {
            return Ok(());
        }
        // wpctl selects the default sink/source from the node's media class.
        let _ = kind;
        run_command("wpctl", &["set-default", &id.to_string()])
    }

    pub fn set_preset(&self, preset: LatencyPreset) -> Result<(), AudioError> {
        if self.fixture.is_none() {
            run_command(
                "pw-metadata",
                &[
                    "-n",
                    "settings",
                    "0",
                    "clock.force-quantum",
                    &preset.quantum().to_string(),
                ],
            )?;
        }
        self.write_preset(preset)
    }

    pub fn streams(&self) -> Result<Vec<AudioStream>, AudioError> {
        if self.fixture.is_some() {
            return Ok(Vec::new());
        }
        command_stdout("pactl", &["--format=json", "list", "sink-inputs"])
            .and_then(|output| parse_pactl_streams(&output))
    }

    /// Move a PulseAudio-compatible playback stream to another sink id/name.
    pub fn move_stream(&self, index: u32, sink: &str) -> Result<(), AudioError> {
        if self.fixture.is_some() {
            return Ok(());
        }
        run_command("pactl", &["move-sink-input", &index.to_string(), sink])
    }

    pub fn xruns(&self) -> Result<u64, AudioError> {
        if self.fixture.is_some() {
            return Ok(self.snapshot()?.xruns);
        }
        command_stdout("pw-top", &["-b", "-n", "1"]).map(|output| parse_pw_top_xruns(&output))
    }

    fn read_preset(&self) -> Option<LatencyPreset> {
        let value = std::fs::read_to_string(&self.state_path).ok()?;
        LatencyPreset::parse(&value)
    }

    fn write_preset(&self, preset: LatencyPreset) -> Result<(), AudioError> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AudioError::Command(error.to_string()))?;
        }
        std::fs::write(&self.state_path, preset.name())
            .map_err(|error| AudioError::Command(error.to_string()))
    }
}

impl Default for SystemAudio {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_fixture(path: &Path) -> Result<AudioSnapshot, AudioError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| AudioError::Unavailable(format!("{}: {error}", path.display())))?;
    let mut snapshot: AudioSnapshot =
        serde_json::from_str(&text).map_err(|error| AudioError::Parse(error.to_string()))?;
    snapshot.source = SnapshotSource::Fixture;
    snapshot.ensure_defaults();
    Ok(snapshot)
}

fn state_path() -> PathBuf {
    if let Some(base) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(base).join("aether/audio/preset");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/state/aether/audio/preset");
    }
    std::env::temp_dir().join("aether-audio-preset")
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String, AudioError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| AudioError::Unavailable(format!("{program}: {error}")))?;
    if !output.status.success() {
        return Err(AudioError::Command(format!(
            "{program} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_command(program: &str, args: &[&str]) -> Result<(), AudioError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| AudioError::Unavailable(format!("{program}: {error}")))?;
    if !output.status.success() {
        return Err(AudioError::Command(format!(
            "{program} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn apply_system_defaults(routes: &mut [AudioRoute]) -> Result<(), AudioError> {
    if let Ok(output) = command_stdout("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"]) {
        if let Some(id) = parse_wpctl_id(&output) {
            for route in routes.iter_mut() {
                route.default = route.kind == RouteKind::Output && route.id == id;
            }
        }
    }
    if let Ok(output) = command_stdout("wpctl", &["inspect", "@DEFAULT_AUDIO_SOURCE@"]) {
        if let Some(id) = parse_wpctl_id(&output) {
            for route in routes.iter_mut() {
                if route.kind == RouteKind::Input && route.id == id {
                    route.default = true;
                }
            }
        }
    }
    Ok(())
}
