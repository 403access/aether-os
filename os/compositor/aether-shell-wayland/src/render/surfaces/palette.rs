//! Palette surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use aether_shell::{
    PaletteAction, PALETTE_CONTENT_TOP, PALETTE_ROW_HEIGHT, PALETTE_ROW_SPACING,
    PALETTE_VISIBLE_ROWS,
};
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

impl App {
    pub(crate) fn compose_palette(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill = ctx.panel_fill;
        let panel_border = ctx.panel_border;
        let palette_here = ctx.flags.palette;
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut icons: Vec<(String, i32, i32)> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if palette_here {
            if let Some(rect) = self.shell.overlay_rect() {
                let width = rect.width as i32;
                let height = rect.height as i32;
                if let Some(buffer) =
                    self.chrome_panel(rect.width, rect.height, 14, panel_fill, panel_border)
                {
                    panels.push((buffer, rect.x - 18, rect.y - 18));
                }
                // Search header.
                if let Some(buffer) = self.chrome_round_top(
                    rect.width.saturating_sub(2),
                    58.min(rect.height),
                    13,
                    tokens.header,
                ) {
                    panels.push((buffer, rect.x + 1, rect.y + 1));
                }
                display_icons.push((
                    "search".into(),
                    18,
                    tokens.accent(255),
                    rect.x + 18,
                    rect.y + 20,
                ));
                let query = self.shell.palette_query();
                if query.is_empty() {
                    labels.push((
                        "Search workspaces, windows, files…".into(),
                        rect.x + 46,
                        rect.y + 18,
                        1,
                        tokens.dim,
                    ));
                } else {
                    labels.push((
                        format!("{query}_"),
                        rect.x + 46,
                        rect.y + 18,
                        1,
                        tokens.text,
                    ));
                }
                let esc = AetherRect::new(rect.x + width - 54, rect.y + 17, 36, 22);
                if let Some(buffer) = self.chrome_round(36, 22, 6, tokens.base) {
                    panels.push((buffer, esc.x, esc.y));
                }
                labels.push(("esc".into(), esc.x + 10, esc.y + 5, 0, tokens.dim));
                chrome.push((
                    AetherRect::new(rect.x + 1, rect.y + 58, rect.width.saturating_sub(2), 1),
                    tokens.border(22).map(|value| value as f32 / 255.0),
                ));

                let entries = self.shell.palette_entries();
                let selected = self.shell.palette_selected();
                if entries.is_empty() {
                    labels.push((
                        "No matching commands".into(),
                        rect.x + width / 2 - 66,
                        rect.y + 96,
                        1,
                        tokens.soft,
                    ));
                    labels.push((
                        "Try a different query, or a prefix such as > or /".into(),
                        rect.x + width / 2 - 148,
                        rect.y + 118,
                        0,
                        tokens.dim,
                    ));
                }
                let start = self.shell.palette_visible_start();
                for (row, entry) in entries
                    .iter()
                    .skip(start)
                    .take(PALETTE_VISIBLE_ROWS)
                    .enumerate()
                {
                    let index = start + row;
                    let row_y =
                        rect.y + PALETTE_CONTENT_TOP + row as i32 * PALETTE_ROW_SPACING as i32;
                    let row_rect = AetherRect::new(
                        rect.x + 10,
                        row_y,
                        rect.width.saturating_sub(20),
                        PALETTE_ROW_HEIGHT,
                    );
                    let active = index == selected;
                    if active {
                        if let Some(buffer) =
                            self.chrome_round(row_rect.width, row_rect.height, 8, tokens.accent(46))
                        {
                            panels.push((buffer, row_rect.x, row_rect.y));
                        }
                        if let Some(buffer) = self.chrome_round(
                            3,
                            row_rect.height.saturating_sub(16),
                            2,
                            tokens.accent(255),
                        ) {
                            panels.push((buffer, row_rect.x, row_rect.y + 8));
                        }
                    }
                    // Icon tile.
                    let tile = AetherRect::new(row_rect.x + 10, row_rect.y + 7, 26, 26);
                    if let Some(buffer) = self.chrome_round(26, 26, 7, tokens.tile) {
                        panels.push((buffer, tile.x, tile.y));
                    }
                    let icon = match entry.action {
                        PaletteAction::LaunchApp(index) => self
                            .shell
                            .launcher(index)
                            .and_then(|launcher| launcher.icon.clone()),
                        _ => None,
                    };
                    let tag = match entry.action {
                        PaletteAction::SwitchWorkspace(_) => "grid",
                        PaletteAction::FocusWindow(_) => "explorer",
                        PaletteAction::LaunchApp(_) => "terminal",
                        PaletteAction::SpawnTerminal => "terminal",
                        PaletteAction::OpenTaskManager => "chart",
                        PaletteAction::OpenCalendar => "calendar",
                        PaletteAction::OpenExplorer => "folder",
                        PaletteAction::OpenChat => "chat",
                        PaletteAction::OpenAudio => "audio",
                        PaletteAction::OpenThemes => "palette",
                        PaletteAction::OpenShortcuts => "keyboard",
                        PaletteAction::OpenSandboxes => "agents",
                        PaletteAction::OpenFile(_) => "file",
                        PaletteAction::Close => "close",
                    };
                    if let Some(path) = icon {
                        icons.push((path, tile.x + 5, tile.y + 5));
                    } else {
                        display_icons.push((
                            tag.into(),
                            16,
                            if active {
                                tokens.accent(255)
                            } else {
                                tokens.soft
                            },
                            tile.x + 5,
                            tile.y + 5,
                        ));
                    }
                    labels.push((
                        entry.title.clone(),
                        row_rect.x + 48,
                        row_rect.y + 11,
                        1,
                        if active {
                            tokens.text
                        } else {
                            tokens.text_soft
                        },
                    ));
                    labels.push((
                        entry.detail.clone(),
                        row_rect.x + row_rect.width as i32
                            - self.text_width(&entry.detail, 0) as i32
                            - 12,
                        row_rect.y + 12,
                        0,
                        tokens.dim,
                    ));
                }

                // Footer with keycaps.
                let footer_y = rect.y + height - 34;
                let mut caps: Vec<(AetherRect, String, bool)> = Vec::new();
                let mut cx = rect.x + 14;
                for (text, label) in [("↑↓", "navigate"), ("↵", "run"), ("esc", "close")] {
                    let key_width = (text.chars().count() as u32 * 7 + 14).max(22);
                    caps.push((
                        AetherRect::new(cx, footer_y + 6, key_width, 20),
                        text.to_string(),
                        true,
                    ));
                    let label_x = cx + key_width as i32 + 5;
                    labels.push((label.into(), label_x, footer_y + 8, 0, tokens.dim));
                    cx = label_x + self.text_width(label, 0) as i32 + 16;
                }
                for (key_rect, text, is_key) in caps {
                    if is_key {
                        if let Some(buffer) =
                            self.chrome_round(key_rect.width, key_rect.height, 5, tokens.base)
                        {
                            panels.push((buffer, key_rect.x, key_rect.y));
                        }
                        labels.push((
                            text.clone(),
                            key_rect.x
                                + (key_rect.width as i32 - self.text_width(&text, 0) as i32) / 2,
                            key_rect.y + 5,
                            0,
                            tokens.soft,
                        ));
                    }
                }
                labels.push((
                    "> commands   @ agents   / files".into(),
                    rect.x + width - 210,
                    footer_y + 8,
                    0,
                    tokens.dim,
                ));
            }
        }

        RenderChunk {
            chrome,
            labels,
            icons,
            panels,
            display_icons,
        }
    }
}
