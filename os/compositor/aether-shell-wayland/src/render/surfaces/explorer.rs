//! Explorer surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::helpers::{file_kind, truncate};

impl App {
    pub(crate) fn compose_explorer(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let accent_f = ctx.accent_f;
        let explorer_here = ctx.flags.explorer;
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if explorer_here {
            let bounds = self.shell.explorer_rect();
            let pane = self.shell.explorer_pane_rect();
            let width = bounds.width as i32;
            let height = bounds.height as i32;
            // Window body and title bar (`.os-window--explorer`).
            if let Some(buffer) = self.chrome_round(
                bounds.width.max(1),
                bounds.height.max(1),
                14,
                [7, 39, 51, 235],
            ) {
                panels.push((buffer, bounds.x, bounds.y));
            }
            if let Some(buffer) =
                self.chrome_round_top(bounds.width.max(1), 40, 14, [10, 51, 66, 255])
            {
                panels.push((buffer, bounds.x, bounds.y));
            }
            // Accent rim: one 14 px-radius outline so the window reads as the
            // same rounded panel as `.os-window.is-focused`.
            let accent = tokens.accent(255);
            if let Some(buffer) = self.chrome_outline(
                bounds.width.max(1),
                bounds.height.max(1),
                14,
                [accent[0], accent[1], accent[2], 120],
            ) {
                panels.push((buffer, bounds.x, bounds.y));
            }
            chrome.push((
                AetherRect::new(bounds.x, bounds.y + 39, bounds.width, 1),
                tokens.surface_a(120).map(|value| value as f32 / 255.0),
            ));

            // Traffic lights.
            for (index, color) in [
                [0xff, 0x8a, 0x92, 255],
                [0xff, 0xd2, 0x7a, 255],
                [0x5e, 0xf2, 0xb0, 255],
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(buffer) = self.chrome_round(11, 11, 5, color) {
                    panels.push((buffer, bounds.x + 11 + index as i32 * 17, bounds.y + 14));
                }
            }
            // App mark, title and subtitle.
            let mark_x = bounds.x + 66;
            let mark_y = bounds.y + 8;
            if let Some(buffer) = self.chrome_round(
                23,
                23,
                5,
                [
                    tokens.accent(255)[0],
                    tokens.accent(255)[1],
                    tokens.accent(255)[2],
                    36,
                ],
            ) {
                panels.push((buffer, mark_x, mark_y));
            }
            display_icons.push((
                "folder".into(),
                13,
                tokens.accent(255),
                mark_x + 5,
                mark_y + 5,
            ));
            labels.push((
                "Files".into(),
                mark_x + 31,
                bounds.y + 13,
                0x100,
                tokens.text,
            ));
            labels.push((
                "Indexed system".into(),
                mark_x + 31 + self.text_width("Files", 1) as i32 + 8,
                bounds.y + 13,
                0,
                tokens.dim,
            ));
            // Live badge.
            let badge = "Live".to_string();
            let badge_w = 8 + 6 + 6 + self.text_width(&badge, 0) + 8;
            let badge_x = bounds.x + width - 12 - badge_w as i32;
            if let Some(buffer) = self.chrome_round(badge_w, 22, 11, tokens.green_a(28)) {
                panels.push((buffer, badge_x, bounds.y + 9));
            }
            if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
                panels.push((buffer, badge_x + 8, bounds.y + 17));
            }
            labels.push((badge, badge_x + 20, bounds.y + 14, 0, tokens.green));

            // App bar: brand, then the search affordance.
            let appbar_y = bounds.y + 40;
            if let Some(buffer) =
                self.chrome_round(bounds.width.max(1), 52, 0, tokens.surface_a(236))
            {
                panels.push((buffer, bounds.x, appbar_y));
            }
            if let Some(buffer) = self.chrome_round(28, 28, 7, tokens.accent(38)) {
                panels.push((buffer, bounds.x + 16, appbar_y + 12));
            }
            display_icons.push((
                "explorer".into(),
                18,
                tokens.accent(255),
                bounds.x + 21,
                appbar_y + 17,
            ));
            labels.push(("Files".into(), bounds.x + 54, appbar_y + 9, 3, tokens.text));
            labels.push((
                "All indexed roots".into(),
                bounds.x + 54,
                appbar_y + 30,
                4,
                tokens.dim,
            ));
            let search = AetherRect::new(bounds.x + width - 252, appbar_y + 10, 232, 32);
            if let Some(buffer) = self.chrome_round(232, 32, 8, tokens.base_a(120)) {
                panels.push((buffer, search.x, search.y));
            }
            display_icons.push(("search".into(), 16, tokens.dim, search.x + 10, search.y + 8));
            labels.push((
                "Search files".into(),
                search.x + 32,
                search.y + 9,
                0,
                tokens.dim,
            ));

            // Pane header.
            let pane_icon = pane.x + 16;
            let pane_y = pane.y + 14;
            display_icons.push(("folder".into(), 22, tokens.accent(255), pane_icon, pane_y));
            labels.push((
                "Local index".into(),
                pane.x + 48,
                pane.y + 14,
                3,
                tokens.text,
            ));
            labels.push((
                "All indexed roots".into(),
                pane.x + 48,
                pane.y + 34,
                4,
                tokens.dim,
            ));
            let count = format!("{} files", self.shell.explorer_match_count());
            labels.push((
                count.clone(),
                pane.x + pane.width as i32 - self.text_width(&count, 0) as i32 - 50,
                pane.y + 20,
                0,
                tokens.soft,
            ));
            display_icons.push((
                "close".into(),
                16,
                tokens.dim,
                pane.x + pane.width as i32 - 32,
                pane.y + 20,
            ));
            // Column header.
            let columns_y = pane.y + 56;
            chrome.push((
                AetherRect::new(pane.x + 1, columns_y, pane.width.saturating_sub(2), 30),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            let type_x = pane.x + (pane.width as f32 * 0.62) as i32;
            labels.push(("NAME".into(), pane.x + 16, columns_y + 9, 0, tokens.dim));
            labels.push(("TYPE".into(), type_x, columns_y + 9, 0, tokens.dim));
            labels.push((
                "LOCATION".into(),
                pane.x + pane.width as i32 - self.text_width("LOCATION", 0) as i32 - 16,
                columns_y + 9,
                0,
                tokens.dim,
            ));
            // Rows.
            let rows: Vec<aether_shell::PaletteFile> = self
                .shell
                .visible_explorer_rows()
                .into_iter()
                .cloned()
                .collect();
            let scroll = self.shell.explorer_scroll();
            let selected = self.shell.explorer_selected();
            let row_height = aether_shell::design::EXPLORER_ROW_HEIGHT as i32;
            let row_top = self.shell.explorer_rows_top();
            for (row, file) in rows.iter().enumerate() {
                let index = scroll + row;
                let y = row_top + row as i32 * row_height;
                let row_rect = AetherRect::new(
                    pane.x + 8,
                    y,
                    pane.width.saturating_sub(16),
                    (row_height - 2) as u32,
                );
                if index == selected {
                    if let Some(buffer) =
                        self.chrome_round(row_rect.width, row_rect.height, 7, tokens.accent(56))
                    {
                        panels.push((buffer, row_rect.x, row_rect.y));
                    }
                    chrome.push((
                        AetherRect::new(
                            row_rect.x,
                            row_rect.y + 4,
                            3,
                            row_rect.height.saturating_sub(8),
                        ),
                        accent_f,
                    ));
                }
                let extension = file
                    .name
                    .rsplit_once('.')
                    .map(|(_, extension)| extension.to_uppercase())
                    .unwrap_or_else(|| "FILE".to_string());
                let kind = file_kind(&extension);
                display_icons.push((kind.0.into(), 16, kind.1, row_rect.x + 8, row_rect.y + 7));
                labels.push((
                    file.name.clone(),
                    row_rect.x + 30,
                    row_rect.y + 8,
                    1,
                    if index == selected {
                        tokens.text
                    } else {
                        tokens.text_soft
                    },
                ));
                labels.push((extension, type_x, row_rect.y + 9, 0, tokens.soft));
                let location = truncate(&file.detail, 34);
                labels.push((
                    location,
                    pane.x + pane.width as i32
                        - self.text_width(&truncate(&file.detail, 34), 0) as i32
                        - 16,
                    row_rect.y + 9,
                    0,
                    tokens.dim,
                ));
            }
            // Pane footer.
            let footer_y = pane.y + pane.height as i32 - 30;
            chrome.push((
                AetherRect::new(pane.x + 1, footer_y, pane.width.saturating_sub(2), 29),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            labels.push((
                format!(
                    "{} of {} files",
                    rows.len(),
                    self.shell.explorer_match_count()
                ),
                pane.x + 16,
                footer_y + 9,
                0,
                tokens.dim,
            ));
            let hint = "Enter open   Esc close   PgUp/PgDn page";
            labels.push((
                hint.into(),
                pane.x + pane.width as i32 - self.text_width(hint, 0) as i32 - 16,
                footer_y + 9,
                0,
                tokens.dim,
            ));
            // Status bar.
            let status_y = bounds.y + height - 32;
            // Status strip: square top edge, rounded bottom corners so the
            // window rim reads as one rounded panel.
            if let Some(buffer) =
                self.chrome_round_bottom(bounds.width.max(1), 32, 13, tokens.surface_a(242))
            {
                panels.push((buffer, bounds.x, status_y));
            }
            let selected_file = self.shell.explorer_selected_file();
            if let Some(file) = selected_file {
                labels.push((
                    file.name.clone(),
                    bounds.x + 16,
                    status_y + 12,
                    1,
                    tokens.text,
                ));
                labels.push((
                    truncate(&file.path, 60),
                    bounds.x + 16 + self.text_width(&file.name, 1) as i32 + 10,
                    status_y + 13,
                    0,
                    tokens.dim,
                ));
            } else {
                labels.push((
                    "No selection".into(),
                    bounds.x + 16,
                    status_y + 12,
                    1,
                    tokens.dim,
                ));
            }
            let status = self.status().clone();
            let mut sx = bounds.x + width - 16;
            let badges = [
                ("agents", format!("{} agents", status.agents)),
                ("cpu", format!("{}%", status.cpu_percent)),
                (
                    "memory",
                    format!("{}/{} MB", status.mem_used_mb, status.mem_total_mb),
                ),
                ("audio", status.audio.clone()),
                ("chart", format!("{:.1} ms", status.frame_ms)),
            ];
            for (icon, text) in badges.iter().rev() {
                let tw = self.text_width(text, 0) as i32;
                sx -= tw + 22;
                display_icons.push(((*icon).into(), 14, tokens.dim, sx, status_y + 11));
                labels.push((text.clone(), sx + 18, status_y + 12, 0, tokens.soft));
                sx -= 14;
            }
        }

        RenderChunk {
            chrome,
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}
