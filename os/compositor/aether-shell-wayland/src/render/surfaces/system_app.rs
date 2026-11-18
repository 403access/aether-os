//! First-party app windows: shared window chrome plus per-app bodies.

use aether_geometry::Rect as AetherRect;
use aether_shell::theme::ThemeTokens;

use crate::app::system::{SystemApp, TITLEBAR_HEIGHT};
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::ui::Ui;

const WINDOW_FILL: [u8; 4] = [7, 39, 51, 235];
const TITLEBAR_FILL: [u8; 4] = [10, 51, 66, 255];
#[allow(dead_code)]
const CARD_FILL: [u8; 4] = [7, 39, 51, 150];
#[allow(dead_code)]
const ROW_FILL: [u8; 4] = [5, 37, 48, 110];
const LIGHTS: [[u8; 4]; 3] = [
    [0xff, 0x8a, 0x92, 255],
    [0xff, 0xd2, 0x7a, 255],
    [0x5e, 0xf2, 0xb0, 255],
];
const RULE: [u8; 4] = [127, 230, 255, 38];

impl App {
    pub(crate) fn compose_system_app(&mut self, ctx: &FrameContext) -> RenderChunk {
        let Some(app) = self.system.app else {
            return RenderChunk::default();
        };
        let tokens = ctx.tokens;
        let rect = self.system_rect();
        let body = self.system_body_rect();
        let mut ui = Ui::default();

        let badge = match app {
            SystemApp::Network => Some((
                if self.system.wifi { "Connected" } else { "Off" },
                tokens.green,
            )),
            SystemApp::Battery => Some(("AC power", tokens.text_soft)),
            SystemApp::Devices => Some((
                if self.system.bluetooth {
                    "2 connected"
                } else {
                    "Off"
                },
                tokens.green,
            )),
            SystemApp::Radio => Some(("Live", tokens.accent(255))),
            SystemApp::Compare => Some(("Live", tokens.green)),
            SystemApp::Studio => Some(("4 nodes", tokens.text_soft)),
            _ => None,
        };
        self.window_chrome(&mut ui, rect, app, badge, tokens);

        if app == SystemApp::About {
            self.about_body(&mut ui, body, tokens);
        }

        RenderChunk {
            chrome: ui.chrome,
            labels: ui.labels,
            icons: Vec::new(),
            panels: ui.panels,
            display_icons: ui.icons,
        }
    }

    /// `.os-window` chrome shared by every first-party app: rounded body,
    /// 40 px title bar, traffic lights, app mark, title + subtitle, badge.
    fn window_chrome(
        &mut self,
        ui: &mut Ui,
        rect: AetherRect,
        app: SystemApp,
        badge: Option<(&str, [u8; 4])>,
        tokens: ThemeTokens,
    ) {
        let accent = tokens.accent(255);
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 14, WINDOW_FILL) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) =
            self.chrome_round_top(rect.width, TITLEBAR_HEIGHT as u32, 14, TITLEBAR_FILL)
        {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) = self.chrome_outline(
            rect.width,
            rect.height,
            14,
            [accent[0], accent[1], accent[2], 120],
        ) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        ui.solid(
            AetherRect::new(rect.x + 1, rect.y + TITLEBAR_HEIGHT - 1, rect.width - 2, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );
        for (index, color) in LIGHTS.into_iter().enumerate() {
            if let Some(buffer) = self.chrome_round(11, 11, 5, color) {
                ui.panel(Some(buffer), rect.x + 11 + index as i32 * 17, rect.y + 14);
            }
        }
        let mark_x = rect.x + 66;
        let mark_y = rect.y + (TITLEBAR_HEIGHT - 23) / 2;
        if let Some(buffer) = self.chrome_round(23, 23, 5, [accent[0], accent[1], accent[2], 36]) {
            ui.panel(Some(buffer), mark_x, mark_y);
        }
        ui.icon(app.icon(), 13, accent, mark_x + 5, mark_y + 5);
        ui.label(app.title(), mark_x + 31, rect.y + 13, 0x100, tokens.text);
        if !app.subtitle().is_empty() {
            ui.label(
                app.subtitle(),
                mark_x + 31 + self.text_width(app.title(), 1) as i32 + 8,
                rect.y + 13,
                0,
                tokens.dim,
            );
        }
        if let Some((text, color)) = badge {
            let width = 8 + 6 + 6 + self.text_width(text, 0) + 8;
            let x = rect.x + rect.width as i32 - 12 - width as i32;
            if let Some(buffer) =
                self.chrome_round(width, 22, 11, [color[0], color[1], color[2], 28])
            {
                ui.panel(Some(buffer), x, rect.y + 9);
            }
            if let Some(buffer) = self.chrome_round(6, 6, 3, color) {
                ui.panel(Some(buffer), x + 8, rect.y + 17);
            }
            ui.label(text, x + 20, rect.y + 14, 0, color);
        }
    }

    // ---------------------------------------------------------------- about

    fn about_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let cx = body.x + body.width as i32 / 2;
        let top = body.y + 62;

        if let Some(buffer) = self.chrome_round(56, 56, 14, tokens.accent(48)) {
            ui.panel(Some(buffer), cx - 28, top);
        }
        if let Some(buffer) = self.chrome_outline(56, 56, 14, tokens.accent(90)) {
            ui.panel(Some(buffer), cx - 28, top);
        }
        ui.icon("water", 30, tokens.accent(255), cx - 15, top + 13);

        let title = "Aether OS";
        let version = " 0.1.0-alpha";
        let title_w = self.text_width(title, 2) as i32;
        let version_w = self.text_width(version, 1) as i32;
        let start = cx - (title_w + version_w) / 2;
        ui.label(title, start, top + 84, 2, tokens.text);
        ui.label(version, start + title_w, top + 92, 1, tokens.dim);

        let paragraph = "A single-target Linux workstation OS drawn from the Water\n\
             design system. Rust compositor, zero-copy IPC, local index and\n\
             capability-isolated agents.";
        for (index, line) in paragraph.lines().enumerate() {
            let width = self.text_width(line, 1) as i32;
            ui.label(
                line,
                cx - width / 2,
                top + 126 + index as i32 * 20,
                1,
                tokens.text_soft,
            );
        }

        let chips = [
            ("Ocean current", tokens.accent(255)),
            ("Glass windows", tokens.soft),
            ("Local-first", tokens.soft),
        ];
        // Chips as one centred row.
        let widths: Vec<i32> = chips
            .iter()
            .map(|(label, _)| 12 + 6 + 6 + self.text_width(label, 0) as i32 + 10)
            .collect();
        let total: i32 = widths.iter().sum::<i32>() + 8 * (chips.len() as i32 - 1);
        let mut x = cx - total / 2;
        for ((label, color), width) in chips.iter().zip(widths.iter()) {
            if let Some(buffer) = self.chrome_round(*width as u32, 26, 13, [255, 255, 255, 16]) {
                ui.panel(Some(buffer), x, top + 210);
            }
            if let Some(buffer) = self.chrome_round(6, 6, 3, *color) {
                ui.panel(Some(buffer), x + 12, top + 220);
            }
            ui.label(*label, x + 24, top + 216, 0, tokens.text_soft);
            x += width + 8;
        }

        // Buttons.
        let primary = "Open the documentation";
        let primary_w = 28 + self.text_width(primary, 1) as i32 + 28;
        let secondary = "Keyboard shortcuts";
        let secondary_w = 24 + self.text_width(secondary, 1) as i32 + 24;
        let gap = 10;
        let mut bx = cx - (primary_w + gap + secondary_w) / 2;
        let by = top + 262;
        if let Some(buffer) = self.chrome_round(primary_w as u32, 36, 8, tokens.accent(255)) {
            ui.panel(Some(buffer), bx, by);
        }
        ui.label(primary, bx + 28, by + 11, 1, [3, 25, 34, 255]);
        bx += primary_w + gap;
        if let Some(buffer) = self.chrome_round(secondary_w as u32, 36, 8, [255, 255, 255, 14]) {
            ui.panel(Some(buffer), bx, by);
        }
        if let Some(buffer) = self.chrome_outline(secondary_w as u32, 36, 8, tokens.border(56)) {
            ui.panel(Some(buffer), bx, by);
        }
        ui.label(secondary, bx + 24, by + 11, 1, tokens.text_soft);

        let footer = "Rust compositor · Smithay · no web view";
        let width = self.text_width(footer, 0) as i32;
        ui.label(
            footer,
            cx - width / 2,
            body.y + body.height as i32 - 30,
            0,
            tokens.dim,
        );
    }
}
