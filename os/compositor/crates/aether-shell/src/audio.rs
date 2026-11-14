//! Audio overlay state: route list, selection and latency presets.
//!
//! Rendering and hit-testing live in `aether-shell-wayland`; this module owns
//! the policy so it can be tested without Wayland or PipeWire.

use aether_audio::{AudioRoute, AudioSnapshot, LatencyPreset, RouteKind};

/// Maximum routes shown per kind; extra devices stay available to the CLI.
pub const AUDIO_VISIBLE_OUTPUTS: usize = 4;
pub const AUDIO_VISIBLE_INPUTS: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioAction {
    SetDefault { kind: RouteKind, id: u32 },
    SetPreset(LatencyPreset),
}

#[derive(Clone, Debug)]
pub struct AudioLayer {
    routes: Vec<AudioRoute>,
    selected: usize,
    preset: LatencyPreset,
    xruns: u64,
    fixture: bool,
}

impl AudioLayer {
    pub fn new(snapshot: AudioSnapshot) -> Self {
        let mut routes: Vec<AudioRoute> = snapshot
            .routes
            .iter()
            .filter(|route| route.kind == RouteKind::Output)
            .take(AUDIO_VISIBLE_OUTPUTS)
            .cloned()
            .collect();
        routes.extend(
            snapshot
                .routes
                .iter()
                .filter(|route| route.kind == RouteKind::Input)
                .take(AUDIO_VISIBLE_INPUTS)
                .cloned(),
        );
        let selected = routes
            .iter()
            .position(|route| route.kind == RouteKind::Output && route.default)
            .unwrap_or(0);
        Self {
            routes,
            selected,
            preset: snapshot.preset,
            xruns: snapshot.xruns,
            fixture: snapshot.source == aether_audio::SnapshotSource::Fixture,
        }
    }

    pub fn routes(&self) -> &[AudioRoute] {
        &self.routes
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_route(&self) -> Option<&AudioRoute> {
        self.routes.get(self.selected)
    }

    pub fn preset(&self) -> LatencyPreset {
        self.preset
    }

    pub fn xruns(&self) -> u64 {
        self.xruns
    }

    pub fn is_fixture(&self) -> bool {
        self.fixture
    }

    pub fn output_count(&self) -> usize {
        self.routes
            .iter()
            .filter(|route| route.kind == RouteKind::Output)
            .count()
    }

    pub fn input_count(&self) -> usize {
        self.routes
            .iter()
            .filter(|route| route.kind == RouteKind::Input)
            .count()
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        if index >= self.routes.len() {
            return false;
        }
        self.selected = index;
        true
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if self.routes.is_empty() {
            return false;
        }
        let len = self.routes.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
        true
    }

    /// Mark a route default locally and return the backend action.
    pub fn activate(&mut self) -> Option<AudioAction> {
        let route = self.routes.get(self.selected)?;
        let kind = route.kind;
        let id = route.id;
        for entry in self.routes.iter_mut() {
            if entry.kind == kind {
                entry.default = entry.id == id;
            }
        }
        Some(AudioAction::SetDefault { kind, id })
    }

    pub fn set_preset(&mut self, preset: LatencyPreset) -> AudioAction {
        self.preset = preset;
        AudioAction::SetPreset(preset)
    }

    pub fn cycle_preset(&mut self) -> AudioAction {
        self.set_preset(self.preset.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_audio::Transport;

    fn route(id: u32, kind: RouteKind, transport: Transport, default: bool) -> AudioRoute {
        AudioRoute {
            id,
            name: format!("node-{id}"),
            description: format!("Route {id}"),
            kind,
            transport,
            profile: "default".into(),
            codec: None,
            default,
            available: true,
            muted: false,
            volume: Some(65),
            pinned: false,
        }
    }

    fn snapshot() -> AudioSnapshot {
        AudioSnapshot {
            routes: vec![
                route(1, RouteKind::Output, Transport::Analog, true),
                route(2, RouteKind::Output, Transport::Usb, false),
                route(3, RouteKind::Output, Transport::Bluetooth, false),
                route(4, RouteKind::Input, Transport::Usb, true),
            ],
            preset: LatencyPreset::Media,
            xruns: 2,
            source: aether_audio::SnapshotSource::Fixture,
        }
    }

    #[test]
    fn opens_on_the_default_output() {
        let layer = AudioLayer::new(snapshot());
        assert_eq!(layer.selected(), 0);
        assert_eq!(layer.output_count(), 3);
        assert_eq!(layer.input_count(), 1);
        assert!(layer.is_fixture());
        assert_eq!(layer.xruns(), 2);
    }

    #[test]
    fn activation_switches_default_and_wraps_selection() {
        let mut layer = AudioLayer::new(snapshot());
        assert!(layer.move_selection(1));
        let action = layer.activate().unwrap();
        assert_eq!(
            action,
            AudioAction::SetDefault {
                kind: RouteKind::Output,
                id: 2
            }
        );
        assert!(layer.routes()[1].default);
        assert!(!layer.routes()[0].default);
        // Marking an input must not clear output defaults.
        layer.move_selection(2);
        assert!(layer.activate().is_some());
        assert!(layer.routes()[1].default);
        assert!(layer.routes()[3].default);
        layer.move_selection(1);
        assert_eq!(layer.selected(), 0);
    }

    #[test]
    fn presets_cycle() {
        let mut layer = AudioLayer::new(snapshot());
        assert_eq!(
            layer.cycle_preset(),
            AudioAction::SetPreset(LatencyPreset::Voice)
        );
        assert_eq!(
            layer.cycle_preset(),
            AudioAction::SetPreset(LatencyPreset::Monitor)
        );
    }
}
