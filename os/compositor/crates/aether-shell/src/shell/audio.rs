//! Audio overlay backed by the system audio policy.

use aether_geometry::{Point, Rect};

use crate::audio;
use crate::design;

use super::Shell;

impl Shell {
    /// Open the audio overlay with a policy snapshot.
    pub fn open_audio(&mut self, snapshot: aether_audio::AudioSnapshot) -> Option<()> {
        self.close_overlay();
        self.close_explorer();
        self.close_chat();
        self.close_remote();
        self.close_themes();
        self.audio = Some(audio::AudioLayer::new(snapshot));
        Some(())
    }

    pub fn close_audio(&mut self) {
        self.audio = None;
    }

    pub fn audio_open(&self) -> bool {
        self.audio.is_some()
    }

    pub fn audio_layer(&self) -> Option<&audio::AudioLayer> {
        self.audio.as_ref()
    }

    pub fn audio_layer_mut(&mut self) -> Option<&mut audio::AudioLayer> {
        self.audio.as_mut()
    }

    pub fn audio_routes(&self) -> &[aether_audio::AudioRoute] {
        self.audio
            .as_ref()
            .map(|layer| layer.routes())
            .unwrap_or(&[])
    }

    pub fn audio_rect(&self) -> Rect {
        let bounds = self
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport));
        let width = design::AUDIO_PANEL_WIDTH
            .min(bounds.width.saturating_sub(32))
            .max(1);
        let height = design::AUDIO_PANEL_HEIGHT
            .min(bounds.height.saturating_sub(32))
            .max(1);
        Rect::new(
            bounds.x + (bounds.width.saturating_sub(width) / 2) as i32,
            bounds.y + (bounds.height.saturating_sub(height) / 2) as i32,
            width,
            height,
        )
    }

    pub fn audio_route_rect(&self, index: usize) -> Option<Rect> {
        let layer = self.audio.as_ref()?;
        if index >= layer.routes().len() {
            return None;
        }
        let rect = self.audio_rect();
        let outputs = layer.output_count();
        let spacing = design::AUDIO_ROW_SPACING as i32;
        let y = if index < outputs {
            rect.y + design::AUDIO_ROWS_TOP + index as i32 * spacing
        } else {
            rect.y
                + design::AUDIO_ROWS_TOP
                + outputs as i32 * spacing
                + design::AUDIO_SECTION_GAP
                + (index - outputs) as i32 * spacing
        };
        Some(Rect::new(
            rect.x + 14,
            y,
            rect.width.saturating_sub(28),
            design::AUDIO_ROW_HEIGHT,
        ))
    }

    pub fn audio_route_at(&self, point: Point) -> Option<usize> {
        (0..self.audio_routes().len()).find(|index| {
            self.audio_route_rect(*index)
                .is_some_and(|rect| rect.contains(point))
        })
    }

    pub fn audio_preset_rect(&self, index: usize) -> Option<Rect> {
        if index >= aether_audio::LatencyPreset::ALL.len() {
            return None;
        }
        let rect = self.audio_rect();
        let chip = 204;
        let gap = 6;
        Some(Rect::new(
            rect.x + 14 + index as i32 * (chip + gap),
            rect.y + rect.height as i32 - 74,
            chip as u32,
            36,
        ))
    }

    pub fn audio_preset_at(&self, point: Point) -> Option<aether_audio::LatencyPreset> {
        aether_audio::LatencyPreset::ALL
            .into_iter()
            .enumerate()
            .find(|(index, _)| {
                self.audio_preset_rect(*index)
                    .is_some_and(|rect| rect.contains(point))
            })
            .map(|(_, preset)| preset)
    }

    pub fn audio_move_selection(&mut self, delta: isize) -> bool {
        self.audio
            .as_mut()
            .is_some_and(|layer| layer.move_selection(delta))
    }

    pub fn audio_select(&mut self, index: usize) -> bool {
        self.audio
            .as_mut()
            .is_some_and(|layer| layer.select_index(index))
    }

    pub fn audio_activate(&mut self) -> Option<audio::AudioAction> {
        self.audio.as_mut().and_then(|layer| layer.activate())
    }

    pub fn audio_cycle_preset(&mut self) -> Option<audio::AudioAction> {
        self.audio.as_mut().map(|layer| layer.cycle_preset())
    }

    pub fn audio_set_preset(
        &mut self,
        preset: aether_audio::LatencyPreset,
    ) -> Option<audio::AudioAction> {
        self.audio.as_mut().map(|layer| layer.set_preset(preset))
    }
}
