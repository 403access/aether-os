//! Chrome tokens derived from the active theme mode and Water tone.

use super::mode::ThemeMode;
use super::settings::ThemeSettings;

/// Semantic colors for compositor chrome. `Copy` so the renderer can take one
/// snapshot at the top of a frame.
#[derive(Clone, Copy, Debug)]
pub struct ThemeTokens {
    pub text: [u8; 4],
    pub text_soft: [u8; 4],
    pub soft: [u8; 4],
    pub dim: [u8; 4],
    pub strong: [u8; 4],
    pub base: [u8; 4],
    pub header: [u8; 4],
    pub panel: [u8; 4],
    pub panel_soft: [u8; 4],
    pub surface: [u8; 4],
    pub tile: [u8; 4],
    pub green: [u8; 4],
    pub amber: [u8; 4],
    pub red: [u8; 4],
    pub red_soft: [u8; 4],
    pub red_bg: [u8; 4],
    accent_rgb: [u8; 3],
    border_rgb: [u8; 3],
    light: bool,
}

impl ThemeTokens {
    pub fn for_settings(settings: &ThemeSettings) -> Self {
        let mode = settings.mode.effective();
        let light = mode == ThemeMode::Light;
        let accent_rgb = settings.accent_rgb();
        if light {
            Self {
                text: [6, 46, 57, 255],
                text_soft: [15, 69, 83, 255],
                soft: [65, 107, 118, 255],
                dim: [109, 148, 158, 255],
                strong: [3, 28, 36, 255],
                base: [238, 248, 251, 255],
                header: [226, 242, 246, 255],
                panel: [255, 255, 255, 250],
                panel_soft: [246, 252, 253, 246],
                surface: [237, 247, 250, 255],
                tile: [255, 255, 255, 210],
                green: [10, 122, 85, 255],
                amber: [138, 92, 0, 255],
                red: [199, 52, 66, 255],
                red_soft: [120, 20, 28, 255],
                red_bg: [255, 235, 236, 245],
                accent_rgb,
                border_rgb: accent_rgb,
                light,
            }
        } else if mode == ThemeMode::Dark {
            Self {
                text: [233, 251, 255, 255],
                text_soft: [179, 235, 250, 255],
                soft: [165, 219, 229, 255],
                dim: [107, 169, 182, 255],
                strong: [255, 255, 255, 255],
                base: [3, 17, 24, 255],
                header: [8, 34, 44, 255],
                panel: [6, 28, 37, 250],
                panel_soft: [6, 28, 37, 246],
                surface: [4, 26, 34, 255],
                tile: [2, 17, 23, 210],
                green: [94, 242, 176, 255],
                amber: [232, 197, 71, 255],
                red: [255, 90, 90, 255],
                red_soft: [255, 215, 215, 255],
                red_bg: [60, 18, 24, 245],
                accent_rgb,
                border_rgb: [127, 230, 255],
                light,
            }
        } else {
            Self {
                text: [233, 251, 255, 255],
                text_soft: [179, 235, 250, 255],
                soft: [165, 219, 229, 255],
                dim: [107, 169, 182, 255],
                strong: [255, 255, 255, 255],
                base: [3, 25, 34, 255],
                header: [9, 49, 63, 255],
                panel: [7, 39, 51, 250],
                panel_soft: [7, 39, 51, 246],
                surface: [5, 37, 48, 255],
                tile: [3, 25, 34, 210],
                green: [94, 242, 176, 255],
                amber: [232, 197, 71, 255],
                red: [255, 90, 90, 255],
                red_soft: [255, 215, 215, 255],
                red_bg: [60, 18, 24, 245],
                accent_rgb,
                border_rgb: [127, 230, 255],
                light,
            }
        }
    }

    pub fn accent(&self, alpha: u8) -> [u8; 4] {
        with_alpha(self.accent_rgb, alpha)
    }

    pub fn border(&self, alpha: u8) -> [u8; 4] {
        with_alpha(self.border_rgb, alpha)
    }

    pub fn base_a(&self, alpha: u8) -> [u8; 4] {
        with_alpha([self.base[0], self.base[1], self.base[2]], alpha)
    }

    pub fn header_a(&self, alpha: u8) -> [u8; 4] {
        with_alpha([self.header[0], self.header[1], self.header[2]], alpha)
    }

    pub fn surface_a(&self, alpha: u8) -> [u8; 4] {
        with_alpha([self.surface[0], self.surface[1], self.surface[2]], alpha)
    }

    pub fn panel_soft_a(&self, alpha: u8) -> [u8; 4] {
        with_alpha(
            [self.panel_soft[0], self.panel_soft[1], self.panel_soft[2]],
            alpha,
        )
    }

    pub fn green_a(&self, alpha: u8) -> [u8; 4] {
        with_alpha([self.green[0], self.green[1], self.green[2]], alpha)
    }

    pub fn panel_opaque(&self) -> [u8; 4] {
        with_alpha([self.panel[0], self.panel[1], self.panel[2]], 255)
    }

    pub fn panel_soft_opaque(&self) -> [u8; 4] {
        with_alpha(
            [self.panel_soft[0], self.panel_soft[1], self.panel_soft[2]],
            255,
        )
    }

    pub fn is_light(&self) -> bool {
        self.light
    }
}

fn with_alpha(rgb: [u8; 3], alpha: u8) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], alpha]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::water::WaterTone;

    #[test]
    fn light_mode_uses_the_design_system_light_accents() {
        let settings = ThemeSettings {
            mode: ThemeMode::Light,
            ..ThemeSettings::default()
        };
        assert_eq!(settings.accent_rgb(), [6, 122, 153]);
        assert_eq!(settings.accent_hex(), "#067a99");
        let lagoon = ThemeSettings {
            tone: WaterTone::Lagoon,
            ..settings
        };
        assert_eq!(lagoon.accent_rgb(), [10, 143, 127]);
        let tokens = lagoon.tokens();
        assert!(tokens.is_light());
        assert_eq!(tokens.text, [6, 46, 57, 255]);
        assert_eq!(tokens.accent(40), [10, 143, 127, 40]);
    }

    #[test]
    fn system_resolves_to_water() {
        let settings = ThemeSettings {
            mode: ThemeMode::System,
            ..ThemeSettings::default()
        };
        assert_eq!(settings.mode.effective(), ThemeMode::Water);
        let tokens = settings.tokens();
        assert!(!tokens.is_light());
        assert_eq!(tokens.base, [3, 25, 34, 255]);
    }
}
