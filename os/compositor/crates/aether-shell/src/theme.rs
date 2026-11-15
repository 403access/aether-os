//! Theme and Water settings shared by the shell and the Themes app.
//!
//! The model mirrors the reference explorer's Water theme panel: one theme
//! mode, six switches and four Water selections. Values, labels,
//! status copy and the JSON shape stay in sync with the web app, while the
//! compositor persists them under `$XDG_CONFIG_HOME/aether/themes.json`
//! instead of `localStorage`.
//!
//! This module also derives the chrome palette the renderer uses. Water and
//! Dark keep the canonical cyan-on-deep-sea surfaces; Light follows the
//! design-system light tokens, including the darker light-mode accents.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// File written by [`ThemeSettings::save`].
pub const THEME_FILE: &str = "themes.json";

/// Environment override used by tests and headless sessions.
pub const THEME_CONFIG_ENV: &str = "AETHER_THEME_CONFIG";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
    Water,
}

impl ThemeMode {
    pub const ALL: [Self; 4] = [Self::System, Self::Light, Self::Dark, Self::Water];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Water => "Water",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Water => "water",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str().eq_ignore_ascii_case(value))
    }

    /// Resolve `System` to the session default. Aether's session default is
    /// Water, matching the canonical design language.
    pub fn effective(self) -> Self {
        match self {
            Self::System => Self::Water,
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterTone {
    Lagoon,
    Ocean,
    Deep,
}

impl WaterTone {
    pub const ALL: [Self; 3] = [Self::Lagoon, Self::Ocean, Self::Deep];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lagoon => "Lagoon",
            Self::Ocean => "Ocean",
            Self::Deep => "Deep",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lagoon => "lagoon",
            Self::Ocean => "ocean",
            Self::Deep => "deep",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|tone| tone.as_str().eq_ignore_ascii_case(value))
    }

    /// Accent RGB. Light mode uses the deeper variants from `tokens.css`;
    /// dark and Water use the bright Water tones.
    pub fn rgb(self, light: bool) -> [u8; 3] {
        match (self, light) {
            (Self::Lagoon, false) => [85, 240, 224],
            (Self::Ocean, false) => [56, 214, 255],
            (Self::Deep, false) => [72, 183, 255],
            (Self::Lagoon, true) => [10, 143, 127],
            (Self::Ocean, true) => [6, 122, 153],
            (Self::Deep, true) => [33, 102, 196],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterSpeed {
    Calm,
    Flowing,
    Rapid,
}

impl WaterSpeed {
    pub const ALL: [Self; 3] = [Self::Calm, Self::Flowing, Self::Rapid];

    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::Flowing => "Flowing",
            Self::Rapid => "Rapid",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Flowing => "flowing",
            Self::Rapid => "rapid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|speed| speed.as_str().eq_ignore_ascii_case(value))
    }

    pub fn factor(self) -> f32 {
        match self {
            Self::Calm => 0.65,
            Self::Flowing => 1.0,
            Self::Rapid => 1.6,
        }
    }

    /// Visible wave bands in the themed backdrop and the Themes preview.
    pub fn wave_count(self) -> usize {
        match self {
            Self::Calm => 2,
            Self::Flowing => 3,
            Self::Rapid => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterIntensity {
    Subtle,
    Balanced,
    Lively,
}

impl WaterIntensity {
    pub const ALL: [Self; 3] = [Self::Subtle, Self::Balanced, Self::Lively];

    pub fn label(self) -> &'static str {
        match self {
            Self::Subtle => "Subtle",
            Self::Balanced => "Balanced",
            Self::Lively => "Lively",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Subtle => "subtle",
            Self::Balanced => "balanced",
            Self::Lively => "lively",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|intensity| intensity.as_str().eq_ignore_ascii_case(value))
    }

    /// Ring size/alpha step used by the preview.
    pub fn level(self) -> u32 {
        match self {
            Self::Subtle => 0,
            Self::Balanced => 1,
            Self::Lively => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterBackground {
    Still,
    Sea,
}

impl WaterBackground {
    pub const ALL: [Self; 2] = [Self::Still, Self::Sea];

    pub fn label(self) -> &'static str {
        match self {
            Self::Still => "Still gradient",
            Self::Sea => "Sea and waves",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Still => "still",
            Self::Sea => "sea",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|background| background.as_str().eq_ignore_ascii_case(value))
    }
}

/// One row in the Themes app.
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

/// The complete Themes app state. Matches the reference app's
/// `WaterSettings` plus the theme mode.
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

    /// JSON shape written to `$XDG_CONFIG_HOME/aether/themes.json`. Keys match
    /// the reference app's `explorer-water-settings` object; `mode` is stored
    /// alongside it like `explorer-theme`.
    pub fn to_json(&self) -> String {
        let stored = StoredTheme {
            mode: self.mode.as_str(),
            effects: self.effects,
            drips: self.drips,
            click_ripples: self.click_ripples,
            caustics: self.caustics,
            row_motion: self.row_motion,
            glass: self.glass,
            background: self.background.as_str(),
            tone: self.tone.as_str(),
            speed: self.speed.as_str(),
            intensity: self.intensity.as_str(),
        };
        serde_json::to_string_pretty(&stored).unwrap_or_else(|_| "{}".to_string()) + "\n"
    }

    /// Parse a file. Missing or wrongly typed fields fall back to defaults and
    /// unknown enum values are ignored, matching the reference app's
    /// conservative reader instead of rejecting the whole document.
    pub fn from_json(text: &str) -> Self {
        let mut settings = Self::default();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return settings;
        };
        let Some(object) = value.as_object() else {
            return settings;
        };
        let bool_field = |name: &str| object.get(name).and_then(|value| value.as_bool());
        let str_field = |name: &str| object.get(name).and_then(|value| value.as_str());

        if let Some(value) = str_field("mode").and_then(ThemeMode::parse) {
            settings.mode = value;
        }
        if let Some(value) = bool_field("effects") {
            settings.effects = value;
        }
        if let Some(value) = bool_field("drips") {
            settings.drips = value;
        }
        if let Some(value) = bool_field("clickRipples") {
            settings.click_ripples = value;
        }
        if let Some(value) = bool_field("caustics") {
            settings.caustics = value;
        }
        if let Some(value) = bool_field("rowMotion") {
            settings.row_motion = value;
        }
        if let Some(value) = bool_field("glass") {
            settings.glass = value;
        }
        if let Some(value) = str_field("background").and_then(WaterBackground::parse) {
            settings.background = value;
        }
        if let Some(value) = str_field("tone").and_then(WaterTone::parse) {
            settings.tone = value;
        }
        if let Some(value) = str_field("speed").and_then(WaterSpeed::parse) {
            settings.speed = value;
        }
        if let Some(value) = str_field("intensity").and_then(WaterIntensity::parse) {
            settings.intensity = value;
        }
        settings
    }

    pub fn load() -> Self {
        Self::load_from_path(&config_path())
    }

    pub fn load_from_path(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .map(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<PathBuf> {
        let path = config_path();
        self.save_to_path(&path)?;
        Ok(path)
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        fs::write(&tmp, self.to_json())?;
        match fs::rename(&tmp, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&tmp);
                Err(error)
            }
        }
    }

    /// Chrome palette for the effective theme.
    pub fn tokens(&self) -> ThemeTokens {
        ThemeTokens::for_settings(self)
    }
}

/// Resolve the settings path. `AETHER_THEME_CONFIG` wins so headless sessions
/// and tests can isolate writes.
pub fn config_path() -> PathBuf {
    if let Some(path) = std::env::var_os(THEME_CONFIG_ENV) {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("aether").join(THEME_FILE)
}

fn cycle<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let len = values.len() as i32;
    values[(index as i32 + delta).rem_euclid(len) as usize]
}

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
                green: [67, 214, 138, 255],
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
                green: [67, 214, 138, 255],
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

#[derive(Serialize)]
struct StoredTheme<'a> {
    mode: &'a str,
    effects: bool,
    drips: bool,
    #[serde(rename = "clickRipples")]
    click_ripples: bool,
    caustics: bool,
    #[serde(rename = "rowMotion")]
    row_motion: bool,
    glass: bool,
    background: &'a str,
    tone: &'a str,
    speed: &'a str,
    intensity: &'a str,
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

    #[test]
    fn json_round_trips_with_the_web_key_names() {
        let settings = ThemeSettings {
            mode: ThemeMode::Dark,
            click_ripples: false,
            row_motion: false,
            background: WaterBackground::Sea,
            tone: WaterTone::Deep,
            speed: WaterSpeed::Rapid,
            intensity: WaterIntensity::Lively,
            ..ThemeSettings::default()
        };
        let json = settings.to_json();
        assert!(json.contains("\"clickRipples\": false"));
        assert!(json.contains("\"rowMotion\": false"));
        assert!(json.contains("\"background\": \"sea\""));
        assert_eq!(ThemeSettings::from_json(&json), settings);
    }

    #[test]
    fn partial_and_invalid_json_falls_back_field_by_field() {
        let settings = ThemeSettings::from_json(
            r#"{"tone":"nope","speed":"rapid","effects":false,"glass":null}"#,
        );
        assert_eq!(settings.tone, WaterTone::Ocean);
        assert_eq!(settings.speed, WaterSpeed::Rapid);
        assert!(!settings.effects);
        assert!(settings.glass);
        let wrong_types =
            ThemeSettings::from_json(r#"{"effects":"yes","drips":1,"tone":[],"glass":true}"#);
        assert!(wrong_types.effects);
        assert!(wrong_types.drips);
        assert_eq!(wrong_types.tone, WaterTone::Ocean);
        assert!(wrong_types.glass);
        assert_eq!(
            ThemeSettings::from_json("not json"),
            ThemeSettings::default()
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "aether-theme-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("themes.json");
        let settings = ThemeSettings {
            mode: ThemeMode::Light,
            tone: WaterTone::Lagoon,
            ..ThemeSettings::default()
        };
        settings.save_to_path(&path).expect("save");
        assert_eq!(ThemeSettings::load_from_path(&path), settings);
        assert_eq!(
            ThemeSettings::load_from_path(&dir.join("missing.json")),
            ThemeSettings::default()
        );
        let _ = fs::remove_dir_all(dir);
    }

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
