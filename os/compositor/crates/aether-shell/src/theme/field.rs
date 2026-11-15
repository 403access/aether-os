//! One settings row in the Themes app: label, help copy and value.

use super::settings::ThemeSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeField {
    Mode,
    Effects,
    Drips,
    ClickRipples,
    Caustics,
    RowMotion,
    Speed,
    Intensity,
    Background,
    Tone,
    Glass,
}

impl ThemeField {
    pub const ALL: [Self; 11] = [
        Self::Mode,
        Self::Effects,
        Self::Drips,
        Self::ClickRipples,
        Self::Caustics,
        Self::RowMotion,
        Self::Speed,
        Self::Intensity,
        Self::Background,
        Self::Tone,
        Self::Glass,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Mode => "Theme",
            Self::Effects => "Water effects",
            Self::Drips => "Drips follow the cursor",
            Self::ClickRipples => "Ripples on click",
            Self::Caustics => "Caustic shimmer",
            Self::RowMotion => "Animated row highlights",
            Self::Speed => "Flow speed",
            Self::Intensity => "Drip energy",
            Self::Background => "Background",
            Self::Tone => "Water tone",
            Self::Glass => "Glass surfaces",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Mode => "Light, dark, the living sea, or follow the system.",
            Self::Effects => "Master switch for ripples and shimmer.",
            Self::Drips => "Small, soft rings trail your pointer.",
            Self::ClickRipples => "Clicking sends one larger ring outward.",
            Self::Caustics => "Slow light patterns drift behind the app.",
            Self::RowMotion => "Hover and selection colours flow between states.",
            Self::Speed => "How quickly the water moves.",
            Self::Intensity => "How lively cursor drips feel.",
            Self::Background => "A still gradient or a living sea.",
            Self::Tone => "Choose the palette of the sea.",
            Self::Glass => "Frosted panes over the moving water.",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Self::Mode => "Theme",
            Self::Effects | Self::Drips | Self::ClickRipples | Self::Caustics | Self::RowMotion => {
                "Effects"
            }
            Self::Speed | Self::Intensity => "Current",
            Self::Background | Self::Tone | Self::Glass => "Look",
        }
    }

    pub fn is_switch(self) -> bool {
        matches!(
            self,
            Self::Effects
                | Self::Drips
                | Self::ClickRipples
                | Self::Caustics
                | Self::RowMotion
                | Self::Glass
        )
    }

    pub fn value(self, settings: &ThemeSettings) -> String {
        match self {
            Self::Mode => settings.mode.label().to_string(),
            Self::Effects => switch_label(settings.effects),
            Self::Drips => switch_label(settings.drips),
            Self::ClickRipples => switch_label(settings.click_ripples),
            Self::Caustics => switch_label(settings.caustics),
            Self::RowMotion => switch_label(settings.row_motion),
            Self::Speed => settings.speed.label().to_string(),
            Self::Intensity => settings.intensity.label().to_string(),
            Self::Background => settings.background.label().to_string(),
            Self::Tone => settings.tone.label().to_string(),
            Self::Glass => switch_label(settings.glass),
        }
    }

    pub fn is_on(self, settings: &ThemeSettings) -> bool {
        match self {
            Self::Effects => settings.effects,
            Self::Drips => settings.drips,
            Self::ClickRipples => settings.click_ripples,
            Self::Caustics => settings.caustics,
            Self::RowMotion => settings.row_motion,
            Self::Glass => settings.glass,
            _ => false,
        }
    }
}

fn switch_label(on: bool) -> String {
    if on { "On" } else { "Off" }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_copy_match_the_reference_panel() {
        assert_eq!(ThemeField::ALL.len(), 11);
        assert_eq!(ThemeField::Effects.title(), "Water effects");
        assert_eq!(ThemeField::Drips.title(), "Drips follow the cursor");
        assert_eq!(ThemeField::ClickRipples.title(), "Ripples on click");
        assert_eq!(ThemeField::RowMotion.title(), "Animated row highlights");
        assert_eq!(
            ThemeField::Background.hint(),
            "A still gradient or a living sea."
        );
        assert_eq!(ThemeField::Glass.group(), "Look");
    }
}
