//! Explorer surface composition: sidebar, toolbar, search row, table, footer
//! and status strip — the single-pane file browser from the Water OS demo.

use std::collections::BTreeSet;

use aether_geometry::Rect as AetherRect;
use aether_shell::theme::ThemeTokens;

use crate::app::overlays::menubar::Clock;
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::helpers::{file_kind, truncate};
use crate::render::ui::Ui;

const WINDOW_FILL: [u8; 4] = [7, 39, 51, 235];
const TITLEBAR_FILL: [u8; 4] = [10, 51, 66, 255];
const SIDEBAR_FILL: [u8; 4] = [5, 37, 48, 120];
const RULE: [u8; 4] = [127, 230, 255, 38];
const LIGHTS: [[u8; 4]; 3] = [
    [0xff, 0x8a, 0x92, 255],
    [0xff, 0xd2, 0x7a, 255],
    [0x5e, 0xf2, 0xb0, 255],
];

impl App {
    pub(crate) fn compose_explorer(&mut self, ctx: &FrameContext) -> RenderChunk {
        if !ctx.flags.explorer {
            return RenderChunk::default();
        }
        let tokens = ctx.tokens;
        let rect = self.shell.explorer_rect();
        let body = AetherRect::new(
            rect.x + 1,
            rect.y + 40,
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(41),
        );
        let mut ui = Ui::default();
        self.explorer_window(&mut ui, rect, tokens);
        self.explorer_sidebar(&mut ui, body, tokens);
        self.explorer_main(&mut ui, body, tokens);
        RenderChunk {
            chrome: ui.chrome,
            labels: ui.labels,
            icons: Vec::new(),
            panels: ui.panels,
            display_icons: ui.icons,
        }
    }

    fn explorer_window(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let accent = tokens.accent(255);
        let hover = self.pointer_in(rect);
        let effects = self.window_effects(rect, 14, true, hover);
        ui.effect(effects.shadow);
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 14, WINDOW_FILL) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) = self.chrome_round_top(rect.width, 40, 14, TITLEBAR_FILL) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) = self.chrome_outline(
            rect.width,
            rect.height,
            14,
            [
                accent[0],
                accent[1],
                accent[2],
                if hover { 215 } else { 170 },
            ],
        ) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        ui.solid(
            AetherRect::new(rect.x + 14, rect.y + 1, rect.width.saturating_sub(28), 1),
            [1.0, 1.0, 1.0, 0.07],
        );
        for (index, color) in LIGHTS.into_iter().enumerate() {
            if let Some(buffer) = self.chrome_round(11, 11, 5, color) {
                ui.panel(Some(buffer), rect.x + 11 + index as i32 * 17, rect.y + 14);
            }
        }
        if let Some(buffer) = self.chrome_round(23, 23, 5, [accent[0], accent[1], accent[2], 36]) {
            ui.panel(Some(buffer), rect.x + 66, rect.y + 8);
        }
        ui.icon("folder", 13, accent, rect.x + 71, rect.y + 13);
        ui.label(
            "Explorer",
            rect.x + 97,
            rect.y + 13,
            0x100 | 10,
            tokens.text,
        );
        let path = self.explorer_path_label();
        ui.label(
            &path,
            rect.x + 97 + self.text_width("Explorer", 10) as i32 + 8,
            rect.y + 13,
            9,
            tokens.dim,
        );
        let badge = format!("{} files", self.shell.explorer_match_count());
        let badge_w = 8 + 6 + 6 + self.text_width(&badge, 0) + 8;
        let badge_x = rect.x + rect.width as i32 - 12 - badge_w as i32;
        if let Some(buffer) = self.chrome_round(badge_w, 22, 11, tokens.green_a(28)) {
            ui.panel(Some(buffer), badge_x, rect.y + 9);
        }
        if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
            ui.panel(Some(buffer), badge_x + 8, rect.y + 17);
        }
        ui.label(badge, badge_x + 20, rect.y + 14, 0, tokens.green);
        ui.grip(rect, [1.0, 1.0, 1.0, 0.18]);
        ui.effect(effects.wave);
    }

    fn explorer_sidebar(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let width = aether_shell::Shell::SIDEBAR_WIDTH;
        if let Some(buffer) = self.chrome_round(width as u32, body.height, 0, SIDEBAR_FILL) {
            ui.panel(Some(buffer), body.x, body.y);
        }
        ui.solid(
            AetherRect::new(body.x + width, body.y, 1, body.height),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );

        let x = body.x + 14;
        ui.label("FAVOURITES", x, body.y + 14, 6, tokens.dim);
        ui.icon("folder", 14, tokens.accent(255), x, body.y + 38);
        ui.label("All indexed files", x + 22, body.y + 36, 4, tokens.text);
        ui.icon("chart", 14, tokens.soft, x, body.y + 66);
        ui.label("Recent", x + 22, body.y + 64, 4, tokens.text_soft);

        ui.label("LOCATIONS", x, body.y + 104, 6, tokens.dim);
        let mut locations: BTreeSet<String> = BTreeSet::new();
        for file in self.shell.visible_explorer_rows() {
            let detail = file.detail.trim();
            if !detail.is_empty() {
                locations.insert(truncate(detail, 20));
            }
        }
        for (index, location) in locations.iter().take(9).enumerate() {
            let row_y = body.y + 126 + index as i32 * 28;
            ui.icon("folder", 13, tokens.soft, x, row_y + 2);
            ui.label(location, x + 20, row_y, 4, tokens.text_soft);
        }
        if locations.is_empty() {
            ui.label("All roots", x + 20, body.y + 126, 4, tokens.dim);
        }

        // Read-only notice.
        let note_y = body.y + body.height as i32 - 76;
        if let Some(buffer) = self.chrome_round(120, 24, 12, [255, 255, 255, 14]) {
            ui.panel(Some(buffer), x, note_y);
        }
        ui.icon("drop", 11, tokens.soft, x + 9, note_y + 6);
        ui.label(
            "Read-only snapshot",
            x + 26,
            note_y + 6,
            6,
            tokens.text_soft,
        );
        ui.label(
            "Files come from the local index.",
            x,
            note_y + 34,
            6,
            tokens.dim,
        );
    }

    fn explorer_main(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let x0 = body.x + aether_shell::Shell::SIDEBAR_WIDTH;
        let width = body.x + body.width as i32 - x0;
        let accent = tokens.accent(255);

        // Toolbar: navigation + breadcrumb + hidden toggle.
        let toolbar_y = body.y;
        for (index, glyph) in ["chevron-left", "chevron-right", "chevron-up"]
            .into_iter()
            .enumerate()
        {
            ui.icon(
                glyph,
                14,
                if index == 2 { tokens.soft } else { tokens.dim },
                x0 + 16 + index as i32 * 28,
                toolbar_y + 13,
            );
        }
        let path = self.explorer_path_label();
        ui.label(&path, x0 + 106, toolbar_y + 11, 0x100, tokens.text);
        ui.icon("file", 13, tokens.dim, x0 + width - 120, toolbar_y + 13);
        ui.label(
            "Hidden",
            x0 + width - 102,
            toolbar_y + 11,
            4,
            tokens.text_soft,
        );
        if let Some(buffer) = self.chrome_round(30, 16, 8, tokens.accent(48)) {
            ui.panel(Some(buffer), x0 + width - 46, toolbar_y + 11);
        }
        if let Some(buffer) = self.chrome_round(10, 10, 5, tokens.soft) {
            ui.panel(Some(buffer), x0 + width - 40, toolbar_y + 14);
        }
        ui.solid(
            AetherRect::new(x0, toolbar_y + 39, width as u32, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );

        // Search row.
        let search_y = toolbar_y + 40;
        let field = AetherRect::new(x0 + 16, search_y + 6, width as u32 - 210, 28);
        if let Some(buffer) = self.chrome_round(field.width, field.height, 7, [255, 255, 255, 12]) {
            ui.panel(Some(buffer), field.x, field.y);
        }
        ui.icon("search", 13, tokens.dim, field.x + 9, field.y + 7);
        let query = self.shell.explorer_query().to_string();
        ui.label(
            if query.is_empty() {
                "Search this folder…".to_string()
            } else {
                query
            },
            field.x + 28,
            field.y + 6,
            4,
            if self.shell.explorer_query().is_empty() {
                tokens.dim
            } else {
                tokens.text
            },
        );
        let everywhere = AetherRect::new(x0 + width - 178, search_y + 6, 100, 28);
        if let Some(buffer) =
            self.chrome_round(everywhere.width, everywhere.height, 7, [255, 255, 255, 10])
        {
            ui.panel(Some(buffer), everywhere.x, everywhere.y);
        }
        ui.label(
            "Everywhere",
            everywhere.x + 10,
            everywhere.y + 6,
            4,
            tokens.text_soft,
        );
        ui.icon(
            "chevron",
            11,
            tokens.dim,
            everywhere.x + 80,
            everywhere.y + 10,
        );
        let filter = AetherRect::new(x0 + width - 70, search_y + 6, 54, 28);
        if let Some(buffer) = self.chrome_round(filter.width, filter.height, 7, [255, 255, 255, 10])
        {
            ui.panel(Some(buffer), filter.x, filter.y);
        }
        ui.icon("sliders", 12, tokens.text_soft, filter.x + 8, filter.y + 8);
        ui.label("Filter", filter.x + 24, filter.y + 6, 4, tokens.text_soft);
        ui.solid(
            AetherRect::new(x0, search_y + 39, width as u32, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );

        // Column header.
        let columns_y = search_y + 40;
        let type_x = x0 + (width as f32 * 0.52) as i32;
        let size_x = x0 + (width as f32 * 0.72) as i32;
        ui.solid(
            AetherRect::new(x0 + 1, columns_y, width as u32 - 2, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );
        ui.label("Name", x0 + 18, columns_y + 8, 6, tokens.dim);
        ui.label("Type", type_x, columns_y + 8, 6, tokens.dim);
        ui.label("Size", size_x, columns_y + 8, 6, tokens.dim);
        let modified = "Modified";
        let modified_w = self.text_width(modified, 6) as i32;
        ui.label(
            modified,
            x0 + width - 16 - modified_w,
            columns_y + 8,
            6,
            tokens.dim,
        );

        // Rows.
        let pane = self.shell.explorer_pane_rect();
        let rows: Vec<aether_shell::PaletteFile> = self
            .shell
            .visible_explorer_rows()
            .into_iter()
            .cloned()
            .collect();
        let scroll = self.shell.explorer_scroll();
        let selected = self.shell.explorer_selected();
        let row_height = aether_shell::design::EXPLORER_ROW_HEIGHT as i32;
        for (row, file) in rows.iter().enumerate() {
            let index = scroll + row;
            let y = pane.y + row as i32 * row_height;
            let row_rect = AetherRect::new(x0 + 8, y, width as u32 - 16, (row_height - 2) as u32);
            if index == selected {
                if let Some(buffer) =
                    self.chrome_round(row_rect.width, row_rect.height, 7, tokens.accent(56))
                {
                    ui.panel(Some(buffer), row_rect.x, row_rect.y);
                }
                ui.solid(
                    AetherRect::new(row_rect.x, row_rect.y + 4, 3, row_rect.height - 8),
                    [
                        accent[0] as f32 / 255.0,
                        accent[1] as f32 / 255.0,
                        accent[2] as f32 / 255.0,
                        1.0,
                    ],
                );
            }
            let extension = file
                .name
                .rsplit_once('.')
                .map(|(_, extension)| extension.to_uppercase())
                .unwrap_or_else(|| "FILE".to_string());
            let kind = file_kind(&extension);
            ui.icon(kind.0, 16, kind.1, row_rect.x + 8, row_rect.y + 7);
            ui.label(
                file.name.clone(),
                row_rect.x + 30,
                row_rect.y + 8,
                1,
                if index == selected {
                    tokens.text
                } else {
                    tokens.text_soft
                },
            );
            ui.label(extension, type_x, row_rect.y + 9, 4, tokens.soft);
            let (size, modified_secs) = self.file_metadata(&file.path);
            ui.label(format_size(size), size_x, row_rect.y + 9, 4, tokens.dim);
            if let Some(seconds) = modified_secs {
                let date = format_modified(seconds);
                let date_w = self.text_width(&date, 4) as i32;
                ui.label(
                    date,
                    x0 + width - 16 - date_w,
                    row_rect.y + 9,
                    4,
                    tokens.dim,
                );
            }
        }

        // Footer.
        let footer_y = body.y + body.height as i32 - 62;
        ui.solid(
            AetherRect::new(x0, footer_y, width as u32, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );
        ui.label(
            format!(
                "{} items · {} files",
                rows.len(),
                self.shell.explorer_match_count()
            ),
            x0 + 18,
            footer_y + 9,
            6,
            tokens.dim,
        );
        let hint = "Enter open   Esc close   PgUp/PgDn page";
        let hint_w = self.text_width(hint, 6) as i32;
        ui.label(hint, x0 + width - 16 - hint_w, footer_y + 9, 6, tokens.dim);

        // Status strip.
        let status_y = body.y + body.height as i32 - 32;
        if let Some(buffer) = self.chrome_round_bottom(body.width, 32, 13, tokens.surface_a(242)) {
            ui.panel(Some(buffer), body.x, status_y);
        }
        if let Some(file) = self.shell.explorer_selected_file() {
            ui.label(
                file.name.clone(),
                body.x + 16,
                status_y + 10,
                0x100,
                tokens.text,
            );
            ui.label(
                truncate(&file.path, 70),
                body.x + 16 + self.text_width(&file.name, 1) as i32 + 10,
                status_y + 11,
                4,
                tokens.dim,
            );
        } else {
            ui.label(
                "Select an item to see details.",
                body.x + 16,
                status_y + 10,
                4,
                tokens.dim,
            );
        }
        let status = self.status().clone();
        let mut sx = body.x + body.width as i32 - 16;
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
            ui.icon(icon, 13, tokens.dim, sx, status_y + 9);
            ui.label(text.clone(), sx + 17, status_y + 10, 4, tokens.soft);
            sx -= 14;
        }
    }

    fn explorer_path_label(&self) -> String {
        self.shell
            .explorer_selected_file()
            .map(|file| {
                let trimmed = file.detail.trim();
                if trimmed.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{trimmed}")
                }
            })
            .unwrap_or_else(|| "All indexed roots".to_string())
    }

    /// Size and mtime for one indexed path, cached per session.
    fn file_metadata(&mut self, path: &str) -> (u64, Option<u64>) {
        if let Some(entry) = self.file_meta.get(path) {
            return *entry;
        }
        let meta = std::fs::metadata(path).ok();
        let size = meta.as_ref().map(|meta| meta.len()).unwrap_or(0);
        let modified = meta
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_secs());
        let entry = (size, modified);
        self.file_meta.insert(path.to_string(), entry);
        entry
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value >= KB * KB * KB {
        format!("{:.1} GB", value / (KB * KB * KB))
    } else if value >= KB * KB {
        format!("{:.1} MB", value / (KB * KB))
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn format_modified(seconds: u64) -> String {
    let clock = Clock::from_unix(seconds as i64);
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {}, {}",
        MONTHS[(clock.month - 1).min(11) as usize],
        clock.day,
        clock.year
    )
}
