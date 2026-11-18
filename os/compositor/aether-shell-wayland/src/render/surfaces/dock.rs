//! Dock composition: glass strip, coloured tiles and the running indicator.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

/// `.os-dock` glass fill and its stronger rim.
const DOCK_FILL: [u8; 4] = [7, 39, 51, 145];
const DOCK_RIM: [u8; 4] = [127, 230, 255, 76];
const TILE_FILL: [u8; 4] = [7, 39, 51, 189];
const SEPARATOR: [u8; 4] = [127, 230, 255, 76];

impl App {
    pub(crate) fn compose_dock(&mut self, ctx: &FrameContext) -> RenderChunk {
        let chrome_output = ctx.chrome_output;
        if !self.dock_visible(chrome_output) {
            return RenderChunk::default();
        }
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();

        let dock = self.dock(chrome_output);
        if let Some(buffer) =
            self.chrome_round(dock.rect.width, dock.rect.height, 17, DOCK_FILL)
        {
            panels.push((buffer, dock.rect.x, dock.rect.y));
        }
        // Rim: four 1 px edges so the strip reads as glass without a fill swap.
        let rim = [
            DOCK_RIM[0] as f32 / 255.0,
            DOCK_RIM[1] as f32 / 255.0,
            DOCK_RIM[2] as f32 / 255.0,
            DOCK_RIM[3] as f32 / 255.0,
        ];
        let rect = dock.rect;
        chrome.push((AetherRect::new(rect.x + 8, rect.y, rect.width.saturating_sub(16), 1), rim));
        chrome.push((
            AetherRect::new(
                rect.x + 8,
                rect.bottom() as i32 - 1,
                rect.width.saturating_sub(16),
                1,
            ),
            rim,
        ));
        chrome.push((AetherRect::new(rect.x, rect.y + 12, 1, rect.height.saturating_sub(24)), rim));
        chrome.push((
            AetherRect::new(
                rect.right() as i32 - 1,
                rect.y + 12,
                1,
                rect.height.saturating_sub(24),
            ),
            rim,
        ));

        // Separator between the applications and the utility group.
        if let Some(buffer) =
            self.chrome_round(dock.separator.width, dock.separator.height, 0, SEPARATOR)
        {
            panels.push((buffer, dock.separator.x, dock.separator.y));
        }

        for (action, item) in &dock.items {
            let tile_x = item.x + 4;
            let tile_y = item.y + 4;
            let [r, g, b] = action.color();
            // `linear-gradient(160deg, color-mix(tile 34%), transparent)` over
            // the surface fill, flattened to one translucent tile.
            if let Some(buffer) = self.chrome_round(38, 38, 10, TILE_FILL) {
                panels.push((buffer, tile_x, tile_y));
            }
            if let Some(buffer) = self.chrome_round(38, 38, 10, [r, g, b, 46]) {
                panels.push((buffer, tile_x, tile_y));
            }
            display_icons.push((
                action.icon().into(),
                19,
                [r, g, b, 255],
                tile_x + 9,
                tile_y + 9,
            ));
            // Running indicator: explorer, terminal and audio mirror open state.
            let running = match action {
                crate::app::overlays::dock::DockAction::Explorer => self.shell.explorer_open(),
                crate::app::overlays::dock::DockAction::Terminal => {
                    self.shell.focused_window().is_some()
                }
                crate::app::overlays::dock::DockAction::Settings => self.shell.themes_open(),
                crate::app::overlays::dock::DockAction::Search => self.shell.overlay().is_some(),
                crate::app::overlays::dock::DockAction::Radio => self.shell.audio_open(),
                _ => false,
            };
            if running {
                if let Some(buffer) = self.chrome_round(4, 4, 2, ctx.tokens.text_soft) {
                    panels.push((buffer, item.x + 21, item.y + 40));
                }
            }
        }

        RenderChunk {
            chrome,
            labels: Vec::new(),
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}