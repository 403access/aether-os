//! Audio routing overlay backed by the system audio service.

use aether_audio::LatencyPreset;
use aether_shell::audio::AudioAction;
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;

impl App {
    pub(crate) fn toggle_audio(&mut self) {
        if self.shell.audio_open() {
            self.shell.close_audio();
            self.mark_full_damage();
            return;
        }
        self.shortcuts_open = false;
        self.close_task_manager();
        self.shell.close_overlay();
        self.shell.close_explorer();
        self.shell.close_chat();
        match self.audio_backend.snapshot() {
            Ok(snapshot) => {
                println!(
                    "aether-shell: audio overlay opened ({} routes, {:?}, preset {})",
                    snapshot.routes.len(),
                    snapshot.source,
                    snapshot.preset.name()
                );
                self.shell.open_audio(snapshot);
            }
            Err(error) => {
                eprintln!("aether-shell: audio unavailable: {error}");
                return;
            }
        }
        self.mark_full_damage();
    }

    pub(crate) fn handle_audio_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let preset: Keysym = keysyms::KEY_p.into();
        if keysym == escape {
            self.shell.close_audio();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.audio_move_selection(-1);
            self.mark_audio_damage();
        } else if keysym == down {
            self.shell.audio_move_selection(1);
            self.mark_audio_damage();
        } else if keysym == enter {
            self.activate_audio();
            self.mark_audio_damage();
        } else if keysym == preset {
            self.cycle_audio_preset();
            self.mark_audio_damage();
        }
    }

    pub(crate) fn activate_audio(&mut self) {
        let label = self
            .shell
            .audio_layer()
            .and_then(|layer| layer.selected_route())
            .map(|route| route.label().to_string())
            .unwrap_or_default();
        let fixture = self
            .shell
            .audio_layer()
            .is_some_and(|layer| layer.is_fixture());
        if let Some(action) = self.shell.audio_activate() {
            match action {
                AudioAction::SetDefault { kind, id } => {
                    if !fixture {
                        if let Err(error) = self.audio_backend.switch(kind, id) {
                            eprintln!("aether-shell: audio switch failed: {error}");
                        }
                    }
                    println!(
                        "aether-shell: audio default {} -> {} ({id})",
                        format!("{kind:?}").to_lowercase(),
                        label
                    );
                }
                AudioAction::SetPreset(preset) => self.apply_audio_preset(preset),
            }
        }
    }

    fn cycle_audio_preset(&mut self) {
        if let Some(AudioAction::SetPreset(preset)) = self.shell.audio_cycle_preset() {
            self.apply_audio_preset(preset);
        }
    }

    pub(crate) fn apply_audio_preset(&self, preset: LatencyPreset) {
        if let Err(error) = self.audio_backend.set_preset(preset) {
            eprintln!("aether-shell: audio preset failed: {error}");
        }
        println!(
            "aether-shell: audio preset -> {} ({})",
            preset.name(),
            preset.quantum()
        );
    }

    pub(crate) fn mark_audio_damage(&mut self) {
        if self.shell.audio_open() {
            let rect = self.shell.audio_rect();
            self.mark_damage_rect(rect);
        }
    }
}
