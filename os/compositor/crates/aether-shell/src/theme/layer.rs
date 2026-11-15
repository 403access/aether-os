//! Interaction state for the open Themes app: selection and actions.

use super::field::ThemeField;
use super::settings::ThemeSettings;

#[derive(Clone, Debug)]
pub struct ThemeLayer {
    settings: ThemeSettings,
    selected: usize,
}

/// A change the backend should persist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeAction {
    Changed,
    Reset,
    ActivateWater,
}

impl ThemeLayer {
    pub fn new(settings: ThemeSettings) -> Self {
        Self {
            settings,
            selected: 0,
        }
    }

    pub fn settings(&self) -> &ThemeSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: ThemeSettings) {
        self.settings = settings;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        let len = ThemeField::ALL.len() as isize;
        let next = (self.selected as isize + delta).clamp(0, len - 1);
        let changed = next != self.selected as isize;
        self.selected = next as usize;
        changed
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= ThemeField::ALL.len() {
            return false;
        }
        let changed = index != self.selected;
        self.selected = index;
        changed
    }

    pub fn activate(&mut self) -> Option<ThemeAction> {
        let field = ThemeField::ALL[self.selected];
        self.settings.toggle(field).then_some(ThemeAction::Changed)
    }

    pub fn adjust(&mut self, delta: i32) -> Option<ThemeAction> {
        let field = ThemeField::ALL[self.selected];
        self.settings
            .cycle(field, delta)
            .then_some(ThemeAction::Changed)
    }

    pub fn reset(&mut self) -> ThemeAction {
        self.settings.reset_water();
        ThemeAction::Reset
    }

    pub fn use_water(&mut self) -> Option<ThemeAction> {
        self.settings
            .use_water()
            .then_some(ThemeAction::ActivateWater)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ThemeMode, WaterTone};

    #[test]
    fn reset_and_use_water_update_the_layer() {
        let mut layer = ThemeLayer::new(ThemeSettings::default());
        layer.settings.tone = WaterTone::Lagoon;
        layer.settings.mode = ThemeMode::Dark;
        layer.settings.glass = false;
        assert_eq!(layer.use_water(), Some(ThemeAction::ActivateWater));
        assert_eq!(layer.settings().mode, ThemeMode::Water);
        assert_eq!(layer.reset(), ThemeAction::Reset);
        assert_eq!(layer.settings(), &ThemeSettings::default());
    }
}
