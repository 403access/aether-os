//! Theme and Water settings shared by the shell and the Themes app.
//!
//! The model mirrors the reference explorer's Water theme panel: one theme
//! mode, six switches and four Water selections. Values, labels, status copy
//! and the JSON shape stay in sync with the web app, while the compositor
//! persists them under `$XDG_CONFIG_HOME/aether/themes.json` instead of
//! `localStorage`.
//!
//! This module also derives the chrome palette the renderer uses. Water and
//! Dark keep the canonical cyan-on-deep-sea surfaces; Light follows the
//! design-system light tokens, including the darker light-mode accents.
//!
//! Layout:
//!
//! - `mode` — theme mode selection.
//! - `water` — tone, speed, intensity and background enums.
//! - `field` — the rows rendered by the Themes app.
//! - `settings` — settings state and value behaviour.
//! - `storage` — JSON shape and on-disk persistence.
//! - `layer` — selection and actions for the open app.
//! - `tokens` — chrome colours derived from the active theme.

mod field;
mod layer;
mod mode;
mod settings;
mod storage;
mod tokens;
mod water;

pub use field::ThemeField;
pub use layer::{ThemeAction, ThemeLayer};
pub use mode::ThemeMode;
pub use settings::ThemeSettings;
pub use storage::{config_path, THEME_CONFIG_ENV, THEME_FILE};
pub use tokens::ThemeTokens;
pub use water::{WaterBackground, WaterIntensity, WaterSpeed, WaterTone};
