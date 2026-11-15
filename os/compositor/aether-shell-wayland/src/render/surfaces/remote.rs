//! Remote surface composition.

use std::sync::Arc;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

impl App {
    pub(crate) fn compose_remote(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill = ctx.panel_fill;
        let panel_border = ctx.panel_border;
        let remote_here = ctx.flags.remote;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if remote_here {
            let rect = self.shell.remote_rect();
            let width = rect.width as i32;
            let height = rect.height as i32;
            let layer = self.shell.remote_layer().cloned();
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
                "remote".into(),
                20,
                tokens.accent(255),
                rect.x + 18,
                rect.y + 18,
            ));
            labels.push((
                "Remote display".into(),
                rect.x + 50,
                rect.y + 12,
                2,
                tokens.text,
            ));
            if let Some(layer) = &layer {
                labels.push((
                    format!(
                        "{} · {} · {} viewer{}",
                        layer.state().label(),
                        layer.tier(),
                        layer.viewers(),
                        if layer.viewers() == 1 { "" } else { "s" }
                    ),
                    rect.x + 50,
                    rect.y + 35,
                    0,
                    if layer.state() == aether_shell::remote::RemoteState::Streaming {
                        tokens.dim
                    } else {
                        tokens.amber
                    },
                ));
                if layer.recording() {
                    if let Some(buffer) = self.chrome_round(54, 22, 6, tokens.red_bg) {
                        panels.push((buffer, rect.x + width - 68, rect.y + 18));
                    }
                    if let Some(buffer) = self.chrome_round(7, 7, 4, tokens.red) {
                        panels.push((buffer, rect.x + width - 58, rect.y + 25));
                    }
                    labels.push((
                        "REC".into(),
                        rect.x + width - 46,
                        rect.y + 23,
                        0,
                        tokens.red_soft,
                    ));
                }
                // Pairing block.
                if let Some(buffer) =
                    self.chrome_round(rect.width.saturating_sub(36), 78, 10, tokens.base_a(230))
                {
                    panels.push((buffer, rect.x + 18, rect.y + 72));
                }
                labels.push((
                    "ONE-SHOT PAIRING CODE".into(),
                    rect.x + 32,
                    rect.y + 84,
                    0,
                    tokens.dim,
                ));
                labels.push((
                    layer.pairing_code().into(),
                    rect.x + 32,
                    rect.y + 100,
                    2,
                    tokens.accent(255),
                ));
                labels.push((
                    format!(
                        "session {} · {} · {}",
                        layer.session_id(),
                        layer.codec(),
                        if layer.software() {
                            "software encode"
                        } else {
                            "hardware encode"
                        }
                    ),
                    rect.x + 32,
                    rect.y + 130,
                    0,
                    tokens.soft,
                ));
                // Session facts.
                let facts_y = rect.y + 166;
                let latency = layer
                    .latency_ms()
                    .map(|value| format!("{value:.1} ms p99"))
                    .unwrap_or_else(|| "—".to_string());
                let facts = [
                    ("LATENCY", latency),
                    (
                        "BITRATE",
                        if layer.bitrate_kbps() > 0 {
                            format!("{:.1} Mbps", layer.bitrate_kbps() as f32 / 1000.0)
                        } else {
                            "—".to_string()
                        },
                    ),
                    (
                        "INPUT",
                        if layer.input_enabled() { "on" } else { "off" }.to_string(),
                    ),
                    (
                        "CLIPBOARD",
                        if layer.clipboard_enabled() {
                            "on"
                        } else {
                            "off"
                        }
                        .to_string(),
                    ),
                ];
                let column = (width - 36) / 2;
                for (index, (name, value)) in facts.iter().enumerate() {
                    let x = rect.x + 18 + (index as i32 % 2) * column;
                    let y = facts_y + (index as i32 / 2) * 42;
                    if let Some(buffer) =
                        self.chrome_round((column - 8) as u32, 34, 8, tokens.header_a(210))
                    {
                        panels.push((buffer, x, y));
                    }
                    labels.push(((*name).into(), x + 10, y + 5, 0, tokens.dim));
                    labels.push((value.clone(), x + 10, y + 18, 1, tokens.text));
                }
                // Audited actions.
                let selected = layer.selected();
                for index in 0..aether_shell::remote::RemoteAction::ALL.len() {
                    let Some(row) = self.shell.remote_action_rect(index) else {
                        continue;
                    };
                    let active = index == selected;
                    if let Some(buffer) = self.chrome_round(
                        row.width,
                        row.height,
                        8,
                        if active {
                            tokens.accent(46)
                        } else {
                            tokens.header_a(190)
                        },
                    ) {
                        panels.push((buffer, row.x, row.y));
                    }
                    if active {
                        if let Some(buffer) = self.chrome_round(
                            3,
                            row.height.saturating_sub(14),
                            2,
                            tokens.accent(255),
                        ) {
                            panels.push((buffer, row.x, row.y + 7));
                        }
                    }
                    labels.push((
                        layer
                            .action_label(index)
                            .unwrap_or_else(|| "Action".to_string()),
                        row.x + 14,
                        row.y + 11,
                        1,
                        if active { tokens.text } else { tokens.soft },
                    ));
                    if index == 0 {
                        labels.push((
                            "ends the session".into(),
                            row.x + row.width as i32 - 140,
                            row.y + 12,
                            0,
                            tokens.dim,
                        ));
                    }
                }
                labels.push((
                    "↑↓ select   Enter activate   I input   C clipboard   Esc close".into(),
                    rect.x + 18,
                    rect.y + height - 20,
                    0,
                    tokens.dim,
                ));
            }
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
