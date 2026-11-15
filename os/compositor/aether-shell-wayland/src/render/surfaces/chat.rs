//! Chat surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

impl App {
    pub(crate) fn compose_chat(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill_soft = ctx.panel_fill_soft;
        let panel_border_soft = ctx.panel_border_soft;
        let chat_here = ctx.flags.chat;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if chat_here {
            let rect = self.shell.chat_rect();
            let width = rect.width as i32;
            let height = rect.height as i32;
            if let Some(buffer) = self.chrome_panel(
                rect.width,
                rect.height,
                14,
                panel_fill_soft,
                panel_border_soft,
            ) {
                panels.push((buffer, rect.x - 18, rect.y - 18));
            }
            // Header band.
            if let Some(buffer) =
                self.chrome_round_top(rect.width.saturating_sub(2), 58, 13, tokens.header)
            {
                panels.push((buffer, rect.x + 1, rect.y + 1));
            }
            if let Some(buffer) = self.chrome_round(28, 28, 8, tokens.accent(42)) {
                panels.push((buffer, rect.x + 14, rect.y + 15));
            }
            display_icons.push((
                "chat".into(),
                17,
                tokens.accent(255),
                rect.x + 20,
                rect.y + 21,
            ));
            labels.push(("Aether".into(), rect.x + 52, rect.y + 13, 2, tokens.text));
            labels.push((
                "Streaming markdown".into(),
                rect.x + 52,
                rect.y + 35,
                0,
                tokens.dim,
            ));
            // Live status pill.
            let pill_w = 86;
            let pill =
                AetherRect::new(rect.x + width - pill_w - 14, rect.y + 18, pill_w as u32, 22);
            if let Some(buffer) = self.chrome_round(pill_w as u32, 22, 11, tokens.green_a(28)) {
                panels.push((buffer, pill.x, pill.y));
            }
            if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
                panels.push((buffer, pill.x + 10, pill.y + 8));
            }
            labels.push(("live".into(), pill.x + 22, pill.y + 5, 0, tokens.green));
            display_icons.push((
                "close".into(),
                14,
                tokens.dim,
                rect.x + width - 30,
                rect.y + 22,
            ));

            // Assistant message card.
            let bubble = AetherRect::new(
                rect.x + 14,
                rect.y + 70,
                rect.width.saturating_sub(28),
                rect.height.saturating_sub(70 + 76),
            );
            if let Some(buffer) = self.chrome_round(bubble.width, bubble.height, 10, tokens.surface)
            {
                panels.push((buffer, bubble.x, bubble.y));
            }
            if let Some(buffer) = self.chrome_round(3, 18, 2, tokens.accent(160)) {
                panels.push((buffer, bubble.x + 1, bubble.y + 16));
            }
            let mut y = bubble.y + 14;
            let bottom = bubble.y + bubble.height as i32 - 12;
            let text_left = bubble.x + 18;
            let max_width = bubble.width as i32 - 36;
            let chat_lines: Vec<aether_shell::ChatLine> = self.shell.visible_chat_lines().to_vec();
            for line in &chat_lines {
                let row_height = if line.scale >= 2 {
                    26
                } else if line.scale == 0 {
                    14
                } else {
                    18
                };
                if y + row_height > bottom {
                    break;
                }
                if line.code {
                    if let Some(buffer) = self.chrome_round(
                        bubble.width.saturating_sub(20),
                        row_height as u32,
                        5,
                        tokens.tile,
                    ) {
                        panels.push((buffer, bubble.x + 10, y - 2));
                    }
                }
                let mut x = text_left + line.indent;
                for span in &line.spans {
                    let color = match span.kind {
                        aether_shell::ChatSpanKind::Text => tokens.text,
                        aether_shell::ChatSpanKind::Bold => tokens.strong,
                        aether_shell::ChatSpanKind::Code => tokens.green,
                    };
                    let mut scale = if line.code { 1 } else { line.scale };
                    if span.kind == aether_shell::ChatSpanKind::Bold {
                        scale |= 0x100;
                    }
                    let span_width = self.text_width(&span.text, scale & 0xff) as i32;
                    if x + span_width > text_left + max_width {
                        break;
                    }
                    labels.push((span.text.clone(), x, y, scale, color));
                    x += span_width;
                }
                y += row_height;
            }
            let track = AetherRect::new(
                rect.x + width - 10,
                bubble.y + 10,
                4,
                bubble.height.saturating_sub(20),
            );
            if let Some(buffer) = self.chrome_round(4, track.height, 2, tokens.base) {
                panels.push((buffer, track.x, track.y));
            }
            let lines = self.shell.chat_line_count().max(1);
            let visible = self.shell.chat_visible_rows().max(1);
            let thumb = ((visible as f32 / lines as f32) * track.height as f32)
                .clamp(12.0, track.height as f32) as u32;
            let offset =
                ((self.shell.chat_scroll() as f32 / lines as f32) * track.height as f32) as i32;
            if let Some(buffer) = self.chrome_round(4, thumb, 2, tokens.accent(190)) {
                panels.push((
                    buffer,
                    track.x,
                    track.y + offset.min(track.height as i32 - thumb as i32).max(0),
                ));
            }

            // Composer footer.
            let composer_y = rect.y + height - 58;
            let composer =
                AetherRect::new(rect.x + 14, composer_y, rect.width.saturating_sub(28), 38);
            if let Some(buffer) = self.chrome_round(composer.width, 38, 10, tokens.base) {
                panels.push((buffer, composer.x, composer.y));
            }
            labels.push((
                "Message Aether…".into(),
                composer.x + 14,
                composer.y + 10,
                1,
                tokens.dim,
            ));
            if let Some(buffer) = self.chrome_round(26, 26, 8, tokens.accent(44)) {
                panels.push((
                    buffer,
                    composer.x + composer.width as i32 - 34,
                    composer.y + 6,
                ));
            }
            display_icons.push((
                "arrow".into(),
                14,
                tokens.accent(255),
                composer.x + composer.width as i32 - 28,
                composer.y + 12,
            ));
            labels.push((
                "esc close   PgUp/PgDn scroll".into(),
                rect.x + width - 176,
                composer_y - 16,
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
