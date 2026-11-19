//! Themes surface composition.

use aether_geometry::Rect as AetherRect;
use std::sync::Arc;

use aether_shell::theme::{ThemeField, ThemeMode};
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::{scene_box, scene_node, scene_text};

impl App {
    pub(crate) fn compose_themes(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill = ctx.panel_fill;
        let panel_border = ctx.panel_border;
        let themes_here = ctx.flags.themes;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if themes_here {
            let rect = self.shell.theme_rect();
            let panel = scene_box!((), None, "overlay", rect, panel_fill, 14.0);
            let header = scene_node!(
                (),
                Some(panel),
                "header",
                AetherRect::new(rect.x, rect.y, rect.width, 58)
            );
            let width = rect.width as i32;
            let settings = *self.shell.theme_settings();
            let selected = self
                .shell
                .theme_layer()
                .map(|layer| layer.selected())
                .unwrap_or(0);
            let accent = settings.accent_rgb();
            let accent4 = |alpha: u8| [accent[0], accent[1], accent[2], alpha];
            let light = tokens.is_light();

            if let Some(buffer) =
                self.chrome_panel(rect.width, rect.height, 14, panel_fill, panel_border)
            {
                panels.push((buffer, rect.x - 18, rect.y - 18));
            }
            if let Some(buffer) =
                self.chrome_round_top(rect.width.saturating_sub(2), 58, 13, tokens.header)
            {
                panels.push((buffer, rect.x + 1, rect.y + 1));
            }
            display_icons.push(("palette".into(), 20, accent4(255), rect.x + 18, rect.y + 18));
            scene_text!(
                (),
                Some(header),
                "title",
                &"Themes",
                AetherRect::new(rect.x + 50, rect.y + 12, self.text_width("Themes", 2), 24),
                2,
                tokens.text
            );
            labels.push(("Themes".into(), rect.x + 50, rect.y + 12, 2, tokens.text));
            labels.push((
                format!(
                    "{} · {} · {}",
                    settings.mode.label(),
                    settings.tone.label(),
                    if settings.effects {
                        "effects on"
                    } else {
                        "effects off"
                    }
                ),
                rect.x + 50,
                rect.y + 35,
                0,
                tokens.dim,
            ));
            let key = "Super+T";
            let key_w = self.text_width(key, 0) as i32 + 22;
            if let Some(buffer) = self.chrome_round(key_w as u32, 22, 6, tokens.base) {
                panels.push((buffer, rect.x + width - key_w - 48, rect.y + 18));
            }
            labels.push((
                key.into(),
                rect.x + width - key_w - 37,
                rect.y + 23,
                0,
                tokens.dim,
            ));
            display_icons.push((
                "close".into(),
                14,
                tokens.dim,
                rect.x + width - 34,
                rect.y + 22,
            ));

            let status = self.shell.theme_status_rect();
            let paused = !settings.is_water() || !settings.effects;
            let status_fill = if paused {
                tokens.surface_a(226)
            } else {
                accent4(24)
            };
            if let Some(buffer) = self.chrome_round(status.width, status.height, 9, status_fill) {
                panels.push((buffer, status.x, status.y));
            }
            if let Some(buffer) =
                self.chrome_round(7, 7, 3, if paused { tokens.dim } else { accent4(255) })
            {
                panels.push((buffer, status.x + 12, status.y + 15));
            }
            labels.push((
                settings.status_text().into(),
                status.x + 26,
                status.y + 10,
                0,
                if paused {
                    tokens.soft
                } else {
                    tokens.text_soft
                },
            ));
            if !settings.is_water() {
                let action = self.shell.theme_action_rect();
                if let Some(buffer) =
                    self.chrome_round(action.width, action.height, 11, accent4(42))
                {
                    panels.push((buffer, action.x, action.y));
                }
                labels.push((
                    "Use Water".into(),
                    action.x + (action.width as i32 - self.text_width("Use Water", 0) as i32) / 2,
                    action.y + 6,
                    0,
                    tokens.text,
                ));
            }

            // Live preview: background, caustic wash, waves and rings all
            // respond to the same values as the reference Water canvas.
            let preview = self.shell.theme_preview_rect();
            let preview_base = if light {
                [232, 246, 250, 255]
            } else if settings.mode.effective() == ThemeMode::Dark {
                [5, 22, 31, 255]
            } else {
                [4, 32, 43, 255]
            };
            if let Some(buffer) = self.chrome_round(preview.width, preview.height, 8, preview_base)
            {
                panels.push((buffer, preview.x, preview.y));
            }
            if settings.effects && settings.caustics {
                for index in 0..3 {
                    let size = 42 + index as u32 * 26;
                    let cx = preview.x + 26 + index * 54;
                    let cy = preview.y + 18 + index * 8;
                    if let Some(buffer) = self.chrome_round(
                        size,
                        size,
                        size / 2,
                        accent4(if light { 14 } else { 18 }),
                    ) {
                        panels.push((buffer, cx - size as i32 / 2, cy - size as i32 / 2));
                    }
                }
            }
            if settings.effects && settings.background == aether_shell::theme::WaterBackground::Sea
            {
                for band in 0..settings.speed.wave_count() {
                    let alpha = (if light { 54 } else { 46 }) - band as u8 * 8;
                    if let Some(buffer) =
                        self.chrome_round(preview.width.saturating_sub(24), 2, 1, accent4(alpha))
                    {
                        panels.push((buffer, preview.x + 12, preview.y + 12 + band as i32 * 10));
                    }
                }
            }
            if settings.effects && settings.drips {
                let level = settings.intensity.level();
                for (index, base_alpha) in [18u8, 34, 56].into_iter().enumerate() {
                    let size = 26 + level * 4 + (2 - index as u32) * 4;
                    let alpha = base_alpha + level as u8 * 4;
                    if let Some(buffer) = self.chrome_round(size, size, size / 2, accent4(alpha)) {
                        panels.push((
                            buffer,
                            preview.x + preview.width as i32 - 62 - size as i32 / 2,
                            preview.y + 26 - size as i32 / 2,
                        ));
                    }
                }
            }
            if settings.effects && settings.click_ripples {
                let size = 38 + settings.intensity.level() * 4;
                if let Some(buffer) =
                    self.chrome_round(size, size, size / 2, accent4(if light { 26 } else { 36 }))
                {
                    panels.push((
                        buffer,
                        preview.x + preview.width as i32 - 152,
                        preview.y + 4,
                    ));
                }
            }
            labels.push((
                "PREVIEW".into(),
                preview.x + 10,
                preview.y + 6,
                0,
                accent4(190),
            ));
            labels.push((
                format!(
                    "{} · {} · {}",
                    settings.background.label(),
                    settings.speed.label(),
                    settings.intensity.label()
                ),
                preview.x + 10,
                preview.y + 33,
                0,
                tokens.text_soft,
            ));
            if !settings.effects {
                labels.push((
                    "Effects off · calm and still".into(),
                    preview.x + preview.width as i32
                        - self.text_width("Effects off · calm and still", 0) as i32
                        - 10,
                    preview.y + 20,
                    0,
                    tokens.dim,
                ));
            }

            let mut group = "";
            for (index, field) in ThemeField::ALL.iter().enumerate() {
                let Some(row) = self.shell.theme_row_rect(index) else {
                    continue;
                };
                if field.group() != group {
                    group = field.group();
                    labels.push((group.to_uppercase(), row.x, row.y - 16, 0, accent4(190)));
                }
                let active = index == selected;
                if active {
                    if let Some(buffer) = self.chrome_round(
                        row.width,
                        row.height,
                        7,
                        accent4(if light { 26 } else { 46 }),
                    ) {
                        panels.push((buffer, row.x, row.y));
                    }
                    if let Some(buffer) =
                        self.chrome_round(3, row.height.saturating_sub(10), 2, accent4(255))
                    {
                        panels.push((buffer, row.x, row.y + 5));
                    }
                }
                labels.push((
                    field.title().into(),
                    row.x + 12,
                    row.y + 4,
                    1,
                    if active { tokens.text } else { tokens.soft },
                ));
                labels.push((field.hint().into(), row.x + 12, row.y + 19, 0, tokens.dim));
                if field.is_switch() {
                    let on = field.is_on(&settings);
                    let track_w = 34u32;
                    let track_h = 18u32;
                    let tx = row.x + row.width as i32 - track_w as i32 - 10;
                    let ty = row.y + (row.height as i32 - track_h as i32) / 2;
                    if let Some(buffer) = self.chrome_round(
                        track_w,
                        track_h,
                        9,
                        if on { accent4(72) } else { tokens.base_a(190) },
                    ) {
                        panels.push((buffer, tx, ty));
                    }
                    if let Some(buffer) =
                        self.chrome_round(12, 12, 6, if on { accent4(255) } else { tokens.dim })
                    {
                        panels.push((buffer, if on { tx + 19 } else { tx + 3 }, ty + 3));
                    }
                } else {
                    let value = field.value(&settings);
                    let text_w = self.text_width(&value, 0) as i32;
                    let pill_w = (text_w + 34).max(78) as u32;
                    let px = row.x + row.width as i32 - pill_w as i32 - 10;
                    let py = row.y + 5;
                    if let Some(buffer) = self.chrome_round(
                        pill_w,
                        22,
                        6,
                        if active { accent4(40) } else { tokens.tile },
                    ) {
                        panels.push((buffer, px, py));
                    }
                    if *field == ThemeField::Tone {
                        if let Some(buffer) = self.chrome_round(8, 8, 4, accent4(255)) {
                            panels.push((buffer, px - 14, py + 7));
                        }
                    }
                    labels.push((
                        value,
                        px + 9,
                        py + 5,
                        0,
                        if active { tokens.text } else { tokens.soft },
                    ));
                    display_icons.push((
                        "chevron".into(),
                        10,
                        if active { accent4(255) } else { tokens.dim },
                        px + pill_w as i32 - 16,
                        py + 6,
                    ));
                }
            }

            let reset = self.shell.theme_reset_rect();
            if let Some(buffer) = self.chrome_round(reset.width, reset.height, 7, tokens.tile) {
                panels.push((buffer, reset.x, reset.y));
            }
            labels.push((
                "Reset water settings".into(),
                reset.x + 12,
                reset.y + 8,
                0,
                tokens.text_soft,
            ));
            let hint = "Saved on this device · ↑↓ select · ←→ change";
            labels.push((
                hint.into(),
                rect.x + width - self.text_width(hint, 0) as i32 - 14,
                reset.y + 8,
                0,
                tokens.dim,
            ));
        }

        RenderChunk {
            chrome: Vec::new(),
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}
