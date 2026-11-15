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
        let panel_fill_soft = ctx.panel_fill_soft;
        let panel_border_soft = ctx.panel_border_soft;
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
            // App bar and status bar.
            if let Some(buffer) =
                self.chrome_round(bounds.width.max(1), 62, 0, tokens.surface_a(236))
            {
                panels.push((buffer, bounds.x, bounds.y));
            }
            if let Some(buffer) =
                self.chrome_round(bounds.width.max(1), 36, 0, tokens.surface_a(242))
            {
                panels.push((buffer, bounds.x, bounds.y + height - 36));
            }
            // Pane card.
            if let Some(buffer) = self.chrome_panel(
                pane.width,
                pane.height,
                12,
                panel_fill_soft,
                panel_border_soft,
            ) {
                panels.push((buffer, pane.x - 18, pane.y - 18));
            }
            // App bar brand.
            display_icons.push((
                "explorer".into(),
                28,
                tokens.accent(255),
                bounds.x + 18,
                bounds.y + 17,
            ));
            labels.push((
                "Aether".into(),
                bounds.x + 56,
                bounds.y + 12,
                2,
                tokens.text,
            ));
            labels.push((
                "Files · indexed system".into(),
                bounds.x + 56,
                bounds.y + 35,
                0,
                tokens.dim,
            ));
            // Center tabs.
            let tab_x = bounds.x + width / 2 - 150;
            let tab_active = AetherRect::new(tab_x, bounds.y + 15, 92, 32);
            if let Some(buffer) = self.chrome_round(92, 32, 8, tokens.accent(38)) {
                panels.push((buffer, tab_active.x, tab_active.y));
            }
            labels.push(("Files".into(), tab_x + 26, bounds.y + 22, 1, tokens.text));
            labels.push(("Palette".into(), tab_x + 120, bounds.y + 22, 1, tokens.soft));
            labels.push(("Chat".into(), tab_x + 215, bounds.y + 22, 1, tokens.soft));
            // Search affordance.
            let search = AetherRect::new(bounds.x + width - 250, bounds.y + 15, 232, 32);
            if let Some(buffer) = self.chrome_round(232, 32, 8, tokens.base) {
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
            labels.push((
                "Super+K".into(),
                search.x + 176,
                search.y + 9,
                0,
                tokens.dim,
            ));
            // Pane header.
            let pane_icon = pane.x + 16;
            let pane_y = pane.y + 14;
            display_icons.push(("folder".into(), 22, tokens.accent(255), pane_icon, pane_y));
            labels.push(("Files".into(), pane.x + 48, pane.y + 14, 3, tokens.text));
            labels.push((
                "All indexed roots".into(),
                pane.x + 48,
                pane.y + 34,
                0,
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
            let status_y = bounds.y + height - 36;
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
