//! The Themes app surface and its settings geometry.

use aether_geometry::{Point, Rect};

use crate::design;
use crate::theme;

use super::Shell;

/// Theme row group offset: one gap after each section label.
fn theme_row_group(index: usize) -> i32 {
    match index {
        0 => 0,
        1..=5 => 1,
        6..=7 => 2,
        _ => 3,
    }
}

impl Shell {
    /// Active theme settings. Persisted by the backend on every change.
    pub fn theme_settings(&self) -> &theme::ThemeSettings {
        &self.theme
    }

    pub fn theme_tokens(&self) -> theme::ThemeTokens {
        self.theme.tokens()
    }

    pub fn theme_accent(&self) -> [u8; 3] {
        self.theme.accent_rgb()
    }

    pub fn set_theme_settings(&mut self, settings: theme::ThemeSettings) {
        self.theme = settings;
        if let Some(layer) = self.themes.as_mut() {
            layer.set_settings(settings);
        }
    }

    /// Toggle one theme field from the Settings app. Returns whether the
    /// settings changed.
    pub fn toggle_theme_field(&mut self, field: theme::ThemeField) -> bool {
        let mut settings = self.theme;
        let changed = settings.toggle(field);
        if changed {
            self.set_theme_settings(settings);
        }
        changed
    }

    /// Cycle the light/dark/water mode from menu bar chrome (View ▸ Toggle
    /// appearance, tray moon). Returns whether anything changed.
    pub fn toggle_theme_mode(&mut self) -> bool {
        let mut settings = self.theme;
        let changed = settings.toggle(theme::ThemeField::Mode);
        if changed {
            self.set_theme_settings(settings);
        }
        changed
    }

    /// Open the Themes app with the current settings.
    pub fn open_themes(&mut self) -> Option<()> {
        self.close_overlay();
        self.close_explorer();
        self.close_chat();
        self.close_audio();
        self.close_remote();
        self.themes = Some(theme::ThemeLayer::new(self.theme));
        Some(())
    }

    pub fn close_themes(&mut self) {
        self.themes = None;
    }

    pub fn themes_open(&self) -> bool {
        self.themes.is_some()
    }

    pub fn theme_layer(&self) -> Option<&theme::ThemeLayer> {
        self.themes.as_ref()
    }

    pub fn theme_layer_mut(&mut self) -> Option<&mut theme::ThemeLayer> {
        self.themes.as_mut()
    }

    pub fn theme_rect(&self) -> Rect {
        let bounds = self
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport));
        let width = design::THEME_PANEL_WIDTH
            .min(bounds.width.saturating_sub(32))
            .max(1);
        let height = design::THEME_PANEL_HEIGHT
            .min(bounds.height.saturating_sub(32))
            .max(1);
        Rect::new(
            bounds.x + (bounds.width.saturating_sub(width) / 2) as i32,
            bounds.y + (bounds.height.saturating_sub(height) / 2) as i32,
            width,
            height,
        )
    }

    pub fn theme_status_rect(&self) -> Rect {
        let rect = self.theme_rect();
        Rect::new(
            rect.x + 14,
            rect.y + design::THEME_STATUS_TOP,
            rect.width.saturating_sub(28),
            design::THEME_STATUS_HEIGHT,
        )
    }

    pub fn theme_preview_rect(&self) -> Rect {
        let rect = self.theme_rect();
        Rect::new(
            rect.x + 14,
            rect.y + design::THEME_PREVIEW_TOP,
            rect.width.saturating_sub(28),
            design::THEME_PREVIEW_HEIGHT,
        )
    }

    pub fn theme_row_rect(&self, index: usize) -> Option<Rect> {
        if index >= theme::ThemeField::ALL.len() {
            return None;
        }
        let rect = self.theme_rect();
        let group = theme_row_group(index);
        Some(Rect::new(
            rect.x + 14,
            rect.y
                + design::THEME_ROWS_TOP
                + index as i32 * design::THEME_ROW_SPACING
                + group * design::THEME_GROUP_GAP,
            rect.width.saturating_sub(28),
            design::THEME_ROW_HEIGHT,
        ))
    }

    pub fn theme_row_at(&self, point: Point) -> Option<usize> {
        (0..theme::ThemeField::ALL.len()).find(|index| {
            self.theme_row_rect(*index)
                .is_some_and(|rect| rect.contains(point))
        })
    }

    pub fn theme_close_rect(&self) -> Rect {
        let rect = self.theme_rect();
        Rect::new(rect.x + rect.width as i32 - 40, rect.y + 18, 24, 24)
    }

    pub fn theme_close_at(&self, point: Point) -> bool {
        self.theme_close_rect().contains(point)
    }

    /// "Use Water" button inside the status strip.
    pub fn theme_action_rect(&self) -> Rect {
        let rect = self.theme_rect();
        Rect::new(
            rect.x + rect.width as i32 - 104,
            rect.y + design::THEME_STATUS_TOP + 7,
            90,
            24,
        )
    }

    pub fn theme_action_at(&self, point: Point) -> bool {
        !self.theme.is_water() && self.theme_action_rect().contains(point)
    }

    pub fn theme_reset_rect(&self) -> Rect {
        let rect = self.theme_rect();
        Rect::new(
            rect.x + 14,
            rect.y + rect.height as i32 - 38,
            196,
            design::THEME_FOOTER_HEIGHT,
        )
    }

    pub fn theme_reset_at(&self, point: Point) -> bool {
        self.theme_reset_rect().contains(point)
    }

    pub fn theme_move_selection(&mut self, delta: isize) -> bool {
        self.themes
            .as_mut()
            .is_some_and(|layer| layer.move_selection(delta))
    }

    pub fn theme_select(&mut self, index: usize) -> bool {
        self.themes
            .as_mut()
            .is_some_and(|layer| layer.select(index))
    }

    pub fn theme_activate(&mut self) -> Option<theme::ThemeAction> {
        let action = self.themes.as_mut().and_then(|layer| layer.activate());
        self.sync_open_theme();
        action
    }

    pub fn theme_adjust(&mut self, delta: i32) -> Option<theme::ThemeAction> {
        let action = self.themes.as_mut().and_then(|layer| layer.adjust(delta));
        self.sync_open_theme();
        action
    }

    pub fn theme_reset(&mut self) -> theme::ThemeAction {
        let action = self
            .themes
            .as_mut()
            .map(theme::ThemeLayer::reset)
            .unwrap_or(theme::ThemeAction::Reset);
        self.sync_open_theme();
        action
    }

    pub fn theme_use_water(&mut self) -> Option<theme::ThemeAction> {
        let action = self.themes.as_mut().and_then(|layer| layer.use_water());
        self.sync_open_theme();
        action
    }

    fn sync_open_theme(&mut self) {
        if let Some(layer) = self.themes.as_ref() {
            self.theme = *layer.settings();
        }
    }
}
