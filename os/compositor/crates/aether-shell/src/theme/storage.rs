//! JSON shape and on-disk persistence for theme settings.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::mode::ThemeMode;
use super::settings::ThemeSettings;
use super::water::{WaterBackground, WaterIntensity, WaterSpeed, WaterTone};

/// File written by [`ThemeSettings::save`].
pub const THEME_FILE: &str = "themes.json";

/// Environment override used by tests and headless sessions.
pub const THEME_CONFIG_ENV: &str = "AETHER_THEME_CONFIG";

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

impl ThemeSettings {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
