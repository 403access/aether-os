//! Shortcuts surface composition.

use aether_geometry::Rect as AetherRect;
use std::sync::Arc;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::{scene_box, scene_node, scene_text};

impl App {
    pub(crate) fn compose_shortcuts(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill = ctx.panel_fill;
        let panel_border = ctx.panel_border;
        let shortcuts_here = ctx.flags.shortcuts;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if shortcuts_here {
            let rect = self.shortcuts_rect();
            let panel = scene_box!((), None, "overlay", rect, panel_fill, 14.0);
            let header = scene_node!(
                (),
                Some(panel),
                "header",
                AetherRect::new(rect.x, rect.y, rect.width, 58)
            );
            let width = rect.width as i32;
            let height = rect.height as i32;
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
                "command".into(),
                20,
                tokens.accent(255),
                rect.x + 18,
                rect.y + 18,
            ));
            scene_text!(
                (),
                Some(header),
                "title",
                &format!("Keyboard shortcuts · {}", self.key_profile.name()),
                AetherRect::new(
                    rect.x + 50,
                    rect.y + 12,
                    self.text_width(
                        &format!("Keyboard shortcuts · {}", self.key_profile.name()),
                        2
                    ),
                    24
                ),
                2,
                tokens.text
            );
            labels.push((
                format!("Keyboard shortcuts · {}", self.key_profile.name()),
                rect.x + 50,
                rect.y + 12,
                2,
                tokens.text,
            ));
            labels.push((
                self.key_profile.description().into(),
                rect.x + 50,
                rect.y + 35,
                0,
                tokens.dim,
            ));
            let key = self.key_profile.cheat_sheet_chord();
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

            let entries = self.key_profile.entries();
            let half = entries.len().div_ceil(2);
            for (column, slice) in [&entries[..half], &entries[half..]].iter().enumerate() {
                let column_x = rect.x + 18 + column as i32 * 368;
                let mut y = rect.y + 76;
                let mut group = "";
                for entry in *slice {
                    if entry.group != group {
                        group = entry.group;
                        labels.push((group.to_uppercase(), column_x, y, 0, tokens.accent(200)));
                        y += 20;
                    }
                    labels.push((entry.chord.into(), column_x, y + 5, 0, tokens.soft));
                    labels.push((
                        entry.action.into(),
                        column_x + 140,
                        y + 3,
                        0,
                        tokens.text_soft,
                    ));
                    y += 28;
                }
            }
            labels.push((
                "Esc close   ↑↓ navigate overlays   Enter activate".into(),
                rect.x + 18,
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
