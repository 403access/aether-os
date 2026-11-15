//! The complete Themes app state and its value behaviour.

use super::field::ThemeField;
use super::mode::ThemeMode;
use super::tokens::ThemeTokens;
use super::water::{WaterBackground, WaterIntensity, WaterSpeed, WaterTone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeSettings {
    pub mode: ThemeMode,
    pub effects: bool,
    pub drips: bool,
    pub click_ripples: bool,
    pub caustics: bool,
    pub row_motion: bool,
    pub glass: bool,
    pub background: WaterBackground,
    pub tone: WaterTone,
    pub speed: WaterSpeed,
    pub intensity: WaterIntensity,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Water,
            effects: true,
            drips: true,
            click_ripples: true,
            caustics: true,
            row_motion: true,
            glass: true,
            background: WaterBackground::Still,
            tone: WaterTone::Ocean,
            speed: WaterSpeed::Flowing,
            intensity: WaterIntensity::Subtle,
        }
    }
}

impl ThemeSettings {
    pub fn is_water(&self) -> bool {
        self.mode.effective() == ThemeMode::Water
    }

    pub fn accent_rgb(&self) -> [u8; 3] {
        self.tone.rgb(self.mode.effective() == ThemeMode::Light)
    }

    pub fn accent_hex(&self) -> String {
        let [r, g, b] = self.accent_rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    pub fn status_text(&self) -> &'static str {
        if !self.is_water() {
            "Water effects are paused while another theme is active."
        } else if !self.effects {
            "Water effects are off. The theme stays calm and still."
        } else if !self.drips {
            "Water effects are active. Cursor drips are off."
        } else {
            "Water effects are active. Drips follow your cursor."
        }
    }

    pub fn toggle(&mut self, field: ThemeField) -> bool {
        let before = *self;
        match field {
            ThemeField::Effects => self.effects = !self.effects,
            ThemeField::Drips => self.drips = !self.drips,
            ThemeField::ClickRipples => self.click_ripples = !self.click_ripples,
            ThemeField::Caustics => self.caustics = !self.caustics,
            ThemeField::RowMotion => self.row_motion = !self.row_motion,
            ThemeField::Glass => self.glass = !self.glass,
            ThemeField::Mode
            | ThemeField::Speed
            | ThemeField::Intensity
            | ThemeField::Background
            | ThemeField::Tone => {
                self.cycle(field, 1);
            }
        }
        *self != before
    }

    pub fn cycle(&mut self, field: ThemeField, delta: i32) -> bool {
        let before = *self;
        match field {
            ThemeField::Mode => {
                self.mode = cycle(&ThemeMode::ALL, self.mode, delta);
            }
            ThemeField::Speed => {
                self.speed = cycle(&WaterSpeed::ALL, self.speed, delta);
            }
            ThemeField::Intensity => {
                self.intensity = cycle(&WaterIntensity::ALL, self.intensity, delta);
            }
            ThemeField::Background => {
                self.background = cycle(&WaterBackground::ALL, self.background, delta);
            }
            ThemeField::Tone => {
                self.tone = cycle(&WaterTone::ALL, self.tone, delta);
            }
            ThemeField::Effects => self.effects = !self.effects,
            ThemeField::Drips => self.drips = !self.drips,
            ThemeField::ClickRipples => self.click_ripples = !self.click_ripples,
            ThemeField::Caustics => self.caustics = !self.caustics,
            ThemeField::RowMotion => self.row_motion = !self.row_motion,
            ThemeField::Glass => self.glass = !self.glass,
        }
        *self != before
    }

    /// Reset the Water settings but keep the selected theme mode, matching the
    /// reference panel's "Reset water settings" action.
    pub fn reset_water(&mut self) -> bool {
        let before = *self;
        let mode = self.mode;
        *self = Self::default();
        self.mode = mode;
        *self != before
    }

    pub fn use_water(&mut self) -> bool {
        if self.mode == ThemeMode::Water {
            return false;
        }
        self.mode = ThemeMode::Water;
        true
    }

    /// Chrome palette for the effective theme.
    pub fn tokens(&self) -> ThemeTokens {
        ThemeTokens::for_settings(self)
    }
}

fn cycle<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let len = values.len() as i32;
    values[(index as i32 + delta).rem_euclid(len) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_reference_app() {
        let settings = ThemeSettings::default();
        assert_eq!(settings.mode, ThemeMode::Water);
        assert!(settings.effects);
        assert!(settings.drips);
        assert!(settings.click_ripples);
        assert!(settings.caustics);
        assert!(settings.row_motion);
        assert!(settings.glass);
        assert_eq!(settings.background, WaterBackground::Still);
        assert_eq!(settings.tone, WaterTone::Ocean);
        assert_eq!(settings.speed, WaterSpeed::Flowing);
        assert_eq!(settings.intensity, WaterIntensity::Subtle);
    }

    #[test]
    fn status_copy_tracks_mode_and_effects() {
        let mut settings = ThemeSettings::default();
        assert_eq!(
            settings.status_text(),
            "Water effects are active. Drips follow your cursor."
        );
        settings.drips = false;
        assert_eq!(
            settings.status_text(),
            "Water effects are active. Cursor drips are off."
        );
        settings.effects = false;
        assert_eq!(
            settings.status_text(),
            "Water effects are off. The theme stays calm and still."
        );
        settings.mode = ThemeMode::Light;
        assert_eq!(
            settings.status_text(),
            "Water effects are paused while another theme is active."
        );
    }

    #[test]
    fn toggles_and_cycles_stay_in_range() {
        let mut settings = ThemeSettings::default();
        assert!(settings.toggle(ThemeField::Glass));
        assert!(!settings.glass);
        assert!(settings.toggle(ThemeField::Glass));
        assert!(settings.glass);

        settings.mode = ThemeMode::Water;
        assert!(settings.cycle(ThemeField::Mode, 1));
        assert_eq!(settings.mode, ThemeMode::System);
        settings.cycle(ThemeField::Mode, -1);
        assert_eq!(settings.mode, ThemeMode::Water);

        settings.tone = WaterTone::Ocean;
        settings.cycle(ThemeField::Tone, 1);
        assert_eq!(settings.tone, WaterTone::Deep);
        settings.cycle(ThemeField::Tone, 1);
        assert_eq!(settings.tone, WaterTone::Lagoon);
        settings.cycle(ThemeField::Tone, -1);
        assert_eq!(settings.tone, WaterTone::Deep);
    }

    #[test]
    fn reset_keeps_the_selected_theme_mode() {
        let mut settings = ThemeSettings {
            mode: ThemeMode::Light,
            tone: WaterTone::Lagoon,
            glass: false,
            speed: WaterSpeed::Rapid,
            ..ThemeSettings::default()
        };
        assert!(settings.reset_water());
        assert_eq!(settings.mode, ThemeMode::Light);
        assert_eq!(settings.tone, WaterTone::Ocean);
        assert!(settings.glass);
        assert_eq!(settings.speed, WaterSpeed::Flowing);
        assert!(!settings.reset_water());
    }
}
