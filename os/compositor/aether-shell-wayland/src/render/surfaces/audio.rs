//! Audio surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

impl App {
    pub(crate) fn compose_audio(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill = ctx.panel_fill;
        let panel_border = ctx.panel_border;
        let audio_here = ctx.flags.audio;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if audio_here {
            let rect = self.shell.audio_rect();
            let width = rect.width as i32;
            let height = rect.height as i32;
            let routes: Vec<aether_audio::AudioRoute> = self.shell.audio_routes().to_vec();
            let selected = self
                .shell
                .audio_layer()
                .map(|layer| layer.selected())
                .unwrap_or(0);
            let preset = self
                .shell
                .audio_layer()
                .map(|layer| layer.preset())
                .unwrap_or_default();
            let xruns = self
                .shell
                .audio_layer()
                .map(|layer| layer.xruns())
                .unwrap_or(0);
            let outputs = self
                .shell
                .audio_layer()
                .map(|layer| layer.output_count())
                .unwrap_or(0);
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
            display_icons.push((
                "audio".into(),
                20,
                tokens.accent(255),
                rect.x + 18,
                rect.y + 18,
            ));
            labels.push(("Audio".into(), rect.x + 50, rect.y + 12, 2, tokens.text));
            let subtitle = if xruns > 0 {
                format!("PipeWire · 48 kHz · {xruns} xruns")
            } else {
                "PipeWire · 48 kHz · no xruns".to_string()
            };
            labels.push((
                subtitle,
                rect.x + 50,
                rect.y + 35,
                0,
                if xruns > 0 { tokens.amber } else { tokens.dim },
            ));
            let key = "Super+A";
            let key_w = self.text_width(key, 0) as i32 + 22;
            if let Some(buffer) = self.chrome_round(key_w as u32, 22, 6, tokens.base) {
                panels.push((buffer, rect.x + width - key_w - 14, rect.y + 18));
            }
            labels.push((
                key.into(),
                rect.x + width - key_w - 3,
                rect.y + 23,
                0,
                tokens.dim,
            ));
            labels.push(("OUTPUT".into(), rect.x + 18, rect.y + 72, 0, tokens.dim));
            if outputs > 0 {
                if let Some(first_input) = self.shell.audio_route_rect(outputs) {
                    labels.push((
                        "INPUT".into(),
                        rect.x + 18,
                        first_input.y - 22,
                        0,
                        tokens.dim,
                    ));
                }
            }
            let row_rects: Vec<AetherRect> = (0..routes.len())
                .filter_map(|index| self.shell.audio_route_rect(index))
                .collect();
            for (index, route) in routes.iter().enumerate() {
                let Some(row_rect) = row_rects.get(index).copied() else {
                    continue;
                };
                let active = index == selected;
                if active {
                    if let Some(buffer) =
                        self.chrome_round(row_rect.width, row_rect.height, 8, tokens.accent(46))
                    {
                        panels.push((buffer, row_rect.x, row_rect.y));
                    }
                    if let Some(buffer) = self.chrome_round(
                        3,
                        row_rect.height.saturating_sub(14),
                        2,
                        tokens.accent(255),
                    ) {
                        panels.push((buffer, row_rect.x, row_rect.y + 7));
                    }
                }
                let tile = AetherRect::new(row_rect.x + 10, row_rect.y + 7, 28, 26);
                if let Some(buffer) = self.chrome_round(28, 26, 7, tokens.tile) {
                    panels.push((buffer, tile.x, tile.y));
                }
                display_icons.push((
                    if route.kind == aether_audio::RouteKind::Output {
                        "audio".into()
                    } else {
                        "mic".into()
                    },
                    16,
                    if active {
                        tokens.accent(255)
                    } else {
                        tokens.soft
                    },
                    tile.x + 6,
                    tile.y + 5,
                ));
                labels.push((
                    route.label().to_string(),
                    row_rect.x + 48,
                    row_rect.y + 6,
                    1,
                    if active {
                        tokens.text
                    } else {
                        tokens.text_soft
                    },
                ));
                let mut detail = format!("{} · {}", route.transport.label(), route.profile);
                if let Some(codec) = &route.codec {
                    detail.push_str(" · ");
                    detail.push_str(codec);
                }
                labels.push((detail, row_rect.x + 48, row_rect.y + 22, 0, tokens.dim));
                if route.default {
                    let pill_w = 64u32;
                    let pill = AetherRect::new(
                        row_rect.x + row_rect.width as i32 - pill_w as i32 - 10,
                        row_rect.y + 10,
                        pill_w,
                        20,
                    );
                    if let Some(buffer) = self.chrome_round(pill_w, 20, 6, tokens.accent(36)) {
                        panels.push((buffer, pill.x, pill.y));
                    }
                    labels.push((
                        "default".into(),
                        pill.x + 9,
                        pill.y + 5,
                        0,
                        tokens.accent(255),
                    ));
                }
            }
            // Input level meter (deterministic placeholder until live peaks).
            labels.push((
                "INPUT LEVEL".into(),
                rect.x + 18,
                rect.y + height - 118,
                0,
                tokens.dim,
            ));
            for bar in 0..14 {
                let level = ((bar * 37 + 53) % 11 + 3) as u32;
                let bar_x = rect.x + 96 + bar * 11;
                let bar_y = rect.y + height - 122 - level as i32;
                if let Some(buffer) =
                    self.chrome_round(6, level * 2, 2, [56, 214, 255, 90 + (bar as u8 % 5) * 28])
                {
                    panels.push((buffer, bar_x, bar_y));
                }
            }
            labels.push((
                "mic locked".into(),
                rect.x + width - 86,
                rect.y + height - 118,
                0,
                tokens.dim,
            ));
            display_icons.push((
                "mic".into(),
                14,
                tokens.dim,
                rect.x + width - 106,
                rect.y + height - 122,
            ));

            for (index, option) in aether_audio::LatencyPreset::ALL.into_iter().enumerate() {
                let Some(chip) = self.shell.audio_preset_rect(index) else {
                    continue;
                };
                let active = option == preset;
                if let Some(buffer) = self.chrome_round(
                    chip.width,
                    chip.height,
                    8,
                    if active {
                        tokens.accent(46)
                    } else {
                        tokens.tile
                    },
                ) {
                    panels.push((buffer, chip.x, chip.y));
                }
                let mut name = option.name().to_string();
                if let Some(first) = name.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                labels.push((
                    name,
                    chip.x + 12,
                    chip.y + 7,
                    1,
                    if active { tokens.text } else { tokens.soft },
                ));
                labels.push((
                    format!("{} frames", option.quantum()),
                    chip.x + chip.width as i32 - 68,
                    chip.y + 9,
                    0,
                    tokens.dim,
                ));
            }
            labels.push((
                "↑↓ select   Enter switch   P preset   Esc close".into(),
                rect.x + 16,
                rect.y + height - 22,
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
