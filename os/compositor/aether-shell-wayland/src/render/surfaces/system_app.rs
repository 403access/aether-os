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
            SystemApp::Network => {
                let up = self
                    .system
                    .network
                    .interfaces
                    .iter()
                    .any(|interface| interface.state == "up");
                Some((
                    if up { "Connected" } else { "Off" },
                    if up { tokens.green } else { tokens.soft },
                ))
            }
            SystemApp::Battery => Some(("AC power", tokens.text_soft)),
            SystemApp::Devices => Some((
                if self.system.bluetooth {
                    "2 connected"
                } else {
                    "Off"
                },
                tokens.green,
            )),
            SystemApp::Radio => Some(("Live", [0xff, 0x9a, 0xd5, 255])),
            SystemApp::Compare => Some(("Live", tokens.green)),
            SystemApp::Studio => Some(("4 nodes", tokens.text_soft)),
            _ => None,
        };
        self.window_chrome(&mut ui, rect, app, badge, tokens);

        if app == SystemApp::About {
            self.about_body(&mut ui, body, tokens);
        }
        if app == SystemApp::Network {
            self.network_body(&mut ui, body, tokens);
        }
        if app == SystemApp::Battery {
            self.battery_body(&mut ui, body, tokens);
        }
        if app == SystemApp::Devices {
            self.devices_body(&mut ui, body, tokens);
        }
        if app == SystemApp::Radio {
            self.radio_body(&mut ui, body, tokens);
        }
        if app == SystemApp::Settings {
            self.settings_body(&mut ui, body, tokens);
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
        let hover = self.pointer_in(rect);
        let effects = self.window_effects(rect, 14, true, hover);
        ui.effect(effects.shadow);
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
        ui.label(
            app.title(),
            mark_x + 31,
            rect.y + 13,
            0x100 | 10,
            tokens.text,
        );
        if !app.subtitle().is_empty() {
            ui.label(
                app.subtitle(),
                mark_x + 31 + self.text_width(app.title(), 10) as i32 + 8,
                rect.y + 13,
                9,
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
        ui.grip(rect, [1.0, 1.0, 1.0, 0.18]);
        ui.effect(effects.wave);
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

impl App {
    // ------------------------------------------------------------- primitives

    fn card(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 10, CARD_FILL) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) = self.chrome_outline(rect.width, rect.height, 10, tokens.border(42)) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
    }

    fn section(&mut self, ui: &mut Ui, x: i32, y: i32, text: &str, tokens: ThemeTokens) {
        ui.label(text.to_uppercase(), x, y, 6, tokens.dim);
    }

    fn switch(&mut self, ui: &mut Ui, rect: AetherRect, on: bool, tokens: ThemeTokens) {
        let fill = if on {
            tokens.accent(255)
        } else {
            [255, 255, 255, 26]
        };
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 10, fill) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        let knob = if on {
            [255, 255, 255, 255]
        } else {
            [165, 219, 229, 255]
        };
        let x = if on { rect.x + 18 } else { rect.x + 4 };
        if let Some(buffer) = self.chrome_round(12, 12, 6, knob) {
            ui.panel(Some(buffer), x, rect.y + 4);
        }
    }

    fn inline_badge(
        &mut self,
        ui: &mut Ui,
        x: i32,
        y: i32,
        text: &str,
        color: [u8; 4],
        tokens: ThemeTokens,
    ) {
        let width = 8 + 6 + 6 + self.text_width(text, 0) + 8;
        if let Some(buffer) = self.chrome_round(width, 20, 10, [color[0], color[1], color[2], 24]) {
            ui.panel(Some(buffer), x - width as i32, y);
        }
        if let Some(buffer) = self.chrome_round(6, 6, 3, color) {
            ui.panel(Some(buffer), x - width as i32 + 8, y + 7);
        }
        ui.label(text, x - width as i32 + 20, y + 4, 0, tokens.text_soft);
    }

    // --------------------------------------------------------------- network

    fn network_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let x = body.x + 16;
        let width = body.width as i32 - 32;
        let facts = self.system.network.clone();
        let primary = facts.primary().cloned().unwrap_or_default();

        let header = AetherRect::new(x, body.y + 16, width as u32, 64);
        self.card(ui, header, tokens);
        if let Some(buffer) = self.chrome_round(34, 34, 9, tokens.accent(30)) {
            ui.panel(Some(buffer), x + 16, body.y + 31);
        }
        ui.icon(
            if primary.wireless { "wifi" } else { "globe" },
            18,
            tokens.accent(255),
            x + 24,
            body.y + 39,
        );
        let title = if primary.name.is_empty() {
            "No network".to_string()
        } else {
            primary.name.clone()
        };
        ui.label(title, x + 64, body.y + 26, 3, tokens.text);
        let meta = match (primary.name.is_empty(), primary.wireless) {
            (true, _) => "No interfaces detected".to_string(),
            (false, true) => format!("Wi-Fi · {}", primary.state),
            (false, false) => format!("Ethernet · {}", primary.state),
        };
        ui.label(meta, x + 64, body.y + 48, 4, tokens.dim);

        let toggle = AetherRect::new(x, body.y + 92, width as u32, 104);
        self.card(ui, toggle, tokens);
        for (index, (label, detail, value)) in [
            (
                "Wi-Fi",
                "Discover and join nearby networks.",
                self.system.wifi,
            ),
            (
                "VPN",
                "Route traffic through the archive gateway.",
                self.system.vpn,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let row_y = body.y + 102 + index as i32 * 52;
            ui.label(label, x + 16, row_y + 6, 0x100, tokens.text);
            ui.label(detail, x + 16, row_y + 22, 4, tokens.dim);
            let switch = self.system_switch_rect(index);
            self.switch(ui, switch, value, tokens);
            if index == 0 {
                ui.solid(
                    AetherRect::new(x + 1, row_y + 51, width as u32 - 2, 1),
                    [
                        RULE[0] as f32 / 255.0,
                        RULE[1] as f32 / 255.0,
                        RULE[2] as f32 / 255.0,
                        0.12,
                    ],
                );
            }
        }

        let rows = facts.interfaces.len().max(1) as i32;
        let networks_y = body.y + 92 + 104 + 12;
        let networks = AetherRect::new(x, networks_y, width as u32, (34 + rows * 44) as u32);
        self.card(ui, networks, tokens);
        self.section(ui, x + 14, networks_y + 12, "Networks", tokens);
        if facts.interfaces.is_empty() {
            ui.label(
                "No interfaces detected.",
                x + 14,
                networks_y + 40,
                4,
                tokens.dim,
            );
        }
        for (index, interface) in facts.interfaces.iter().enumerate() {
            let row_y = networks_y + 30 + index as i32 * 44;
            ui.icon(
                if interface.wireless { "wifi" } else { "globe" },
                16,
                tokens.text_soft,
                x + 14,
                row_y + 10,
            );
            ui.label(&interface.name, x + 42, row_y + 6, 0x100, tokens.text);
            let detail = format!(
                "{} · {}",
                if interface.wireless {
                    "Wi-Fi"
                } else {
                    "Ethernet"
                },
                interface.state
            );
            ui.label(detail, x + 42, row_y + 22, 4, tokens.dim);
            let (label, color) = if interface.state == "up" {
                ("Connected", tokens.green)
            } else {
                ("Down", tokens.dim)
            };
            self.inline_badge(ui, x + width - 16, row_y + 12, label, color, tokens);
        }

        let details_y = networks_y + 34 + rows * 44 + 12;
        let details = AetherRect::new(x, details_y, width as u32, 160);
        self.card(ui, details, tokens);
        self.section(ui, x + 14, details_y + 12, "Details", tokens);
        let dns = if facts.dns.is_empty() {
            "—".to_string()
        } else {
            facts.dns.join(", ")
        };
        let rows = [
            ("IP address", facts.ip.clone().unwrap_or_else(|| "—".into())),
            (
                "Router",
                facts.gateway.clone().unwrap_or_else(|| "—".into()),
            ),
            ("DNS", dns),
            (
                "MAC address",
                if primary.mac.is_empty() {
                    "—".to_string()
                } else {
                    primary.mac.clone()
                },
            ),
            ("Band", "—".to_string()),
            ("Security", "—".to_string()),
        ];
        for (index, (label, value)) in rows.into_iter().enumerate() {
            let row_y = details_y + 36 + index as i32 * 20;
            ui.label(label, x + 14, row_y, 4, tokens.dim);
            ui.label(value, x + 140, row_y - 1, 4, tokens.text_soft);
        }
    }
}

impl App {
    // --------------------------------------------------------------- battery

    fn battery_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let x = body.x + 16;
        let width = body.width as i32 - 32;
        let facts = self.system.battery.clone();

        let header = AetherRect::new(x, body.y + 16, width as u32, 96);
        self.card(ui, header, tokens);
        if facts.present {
            ui.label(
                format!("{}%", facts.capacity),
                x + 16,
                body.y + 30,
                2,
                tokens.text,
            );
            let remaining = match facts.status.as_str() {
                "Charging" => "Charging".to_string(),
                "Full" => "Fully charged".to_string(),
                _ => format!("{} · {:.1} h estimated", facts.status, 4.2),
            };
            ui.label(remaining, x + 16, body.y + 58, 4, tokens.dim);
            let bar = AetherRect::new(x + 16, body.y + 82, width as u32 - 32, 10);
            if let Some(buffer) = self.chrome_round(bar.width, bar.height, 5, [255, 255, 255, 22]) {
                ui.panel(Some(buffer), bar.x, bar.y);
            }
            let fill = (bar.width as f32 * (facts.capacity as f32 / 100.0)).round() as u32;
            if fill > 0 {
                let color = if facts.capacity >= 20 {
                    tokens.green
                } else {
                    tokens.amber
                };
                if let Some(buffer) = self.chrome_round(fill.max(6), bar.height, 5, color) {
                    ui.panel(Some(buffer), bar.x, bar.y);
                }
            }
        } else {
            ui.label("On AC power", x + 16, body.y + 32, 2, tokens.text);
            ui.label(
                "No battery detected on this machine.",
                x + 16,
                body.y + 66,
                4,
                tokens.dim,
            );
        }

        // Energy mode.
        let energy = AetherRect::new(x, body.y + 124, width as u32, 76);
        self.card(ui, energy, tokens);
        self.section(ui, x + 16, body.y + 136, "Energy mode", tokens);
        let labels = ["Low power", "Balanced", "High performance"];
        for (index, label) in labels.iter().enumerate() {
            let rect = self.energy_segment_rect(index);
            let active = self.system.energy == index;
            let fill = if active {
                tokens.accent(255)
            } else {
                [255, 255, 255, 12]
            };
            if let Some(buffer) = self.chrome_round(rect.width, rect.height, 7, fill) {
                ui.panel(Some(buffer), rect.x, rect.y);
            }
            let text_width = self.text_width(label, 4) as i32;
            ui.label(
                *label,
                rect.x + (rect.width as i32 - text_width) / 2,
                rect.y + 9,
                4,
                if active {
                    [3, 25, 34, 255]
                } else {
                    tokens.text_soft
                },
            );
        }

        // Toggles.
        let toggle = AetherRect::new(x, body.y + 212, width as u32, 104);
        self.card(ui, toggle, tokens);
        for (index, (label, detail, value)) in [
            (
                "Low power mode",
                "Reduce background activity when unplugged.",
                self.system.energy == 0,
            ),
            (
                "Optimised charging",
                "Finish charging closer to when you unplug.",
                self.system.charging_optimised,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let row_y = body.y + 222 + index as i32 * 52;
            ui.label(label, x + 16, row_y + 6, 0x100, tokens.text);
            ui.label(detail, x + 16, row_y + 22, 4, tokens.dim);
            let rect = AetherRect::new(x + width - 50, row_y + 16, 34, 20);
            self.switch(ui, rect, value, tokens);
            if index == 0 {
                ui.solid(
                    AetherRect::new(x + 1, row_y + 51, width as u32 - 2, 1),
                    [
                        RULE[0] as f32 / 255.0,
                        RULE[1] as f32 / 255.0,
                        RULE[2] as f32 / 255.0,
                        0.12,
                    ],
                );
            }
        }

        // Battery health.
        let health_y = body.y + 328;
        let health = AetherRect::new(x, health_y, width as u32, 138);
        self.card(ui, health, tokens);
        self.section(ui, x + 16, health_y + 12, "Battery health", tokens);
        let health_rows = [
            ("Condition", facts.health.clone()),
            (
                "Maximum capacity",
                if facts.present {
                    format!("{}%", facts.capacity)
                } else {
                    "—".into()
                },
            ),
            (
                "Cycle count",
                facts
                    .cycles
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "Temperature",
                facts
                    .temperature
                    .map(|value| format!("{value:.1} °C"))
                    .unwrap_or_else(|| "—".into()),
            ),
            ("Charging", facts.status.clone()),
        ];
        for (index, (label, value)) in health_rows.into_iter().enumerate() {
            let row_y = health_y + 38 + index as i32 * 21;
            ui.label(label, x + 16, row_y, 4, tokens.dim);
            ui.label(value, x + 170, row_y - 1, 4, tokens.text_soft);
        }

        // Largest consumers.
        let consumers_y = health_y + 150;
        let rows = self.system.consumers.len().max(1) as i32;
        let consumers = AetherRect::new(x, consumers_y, width as u32, (34 + rows * 26) as u32);
        self.card(ui, consumers, tokens);
        self.section(ui, x + 16, consumers_y + 12, "Largest consumers", tokens);
        if self.system.consumers.is_empty() {
            ui.label(
                "No process samples.",
                x + 16,
                consumers_y + 40,
                4,
                tokens.dim,
            );
        }
        for (index, (name, cpu)) in self.system.consumers.clone().into_iter().enumerate() {
            let row_y = consumers_y + 34 + index as i32 * 26;
            ui.label(name, x + 16, row_y + 2, 4, tokens.text_soft);
            let bar_x = x + 190;
            let bar = AetherRect::new(bar_x, row_y + 6, (width as u32) - 250, 6);
            if let Some(buffer) = self.chrome_round(bar.width, bar.height, 3, [255, 255, 255, 20]) {
                ui.panel(Some(buffer), bar.x, bar.y);
            }
            let fill = (bar.width as f32 * (cpu.clamp(0.0, 100.0) / 100.0)).round() as u32;
            if fill > 0 {
                if let Some(buffer) =
                    self.chrome_round(fill.max(3), bar.height, 3, tokens.accent(255))
                {
                    ui.panel(Some(buffer), bar.x, bar.y);
                }
            }
            let value = format!("{cpu:.1}%");
            let value_width = self.text_width(&value, 4) as i32;
            ui.label(
                value,
                x + width - 16 - value_width,
                row_y + 2,
                4,
                tokens.text_soft,
            );
        }
    }
}

impl App {
    // --------------------------------------------------------------- devices

    fn devices_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let x = body.x + 16;
        let width = body.width as i32 - 32;
        let facts = self.system.bluetooth_facts.clone();
        let connected = facts.devices.len();

        let header = AetherRect::new(x, body.y + 16, width as u32, 64);
        self.card(ui, header, tokens);
        if let Some(buffer) = self.chrome_round(34, 34, 9, tokens.accent(30)) {
            ui.panel(Some(buffer), x + 16, body.y + 31);
        }
        ui.icon("bluetooth", 18, tokens.accent(255), x + 24, body.y + 39);
        ui.label("Bluetooth", x + 64, body.y + 26, 3, tokens.text);
        let meta = if !facts.adapter {
            "No adapter detected".to_string()
        } else if !self.system.bluetooth {
            "Off".to_string()
        } else {
            format!(
                "On · {connected} accessory{} connected",
                if connected == 1 { "" } else { "ies" }
            )
        };
        ui.label(meta, x + 64, body.y + 48, 4, tokens.dim);

        let toggle = AetherRect::new(x, body.y + 92, width as u32, 104);
        self.card(ui, toggle, tokens);
        for (index, (label, detail, value)) in [
            (
                "Bluetooth",
                "Find and connect wireless accessories.",
                self.system.bluetooth,
            ),
            (
                "Discoverable",
                "Let nearby devices find this machine.",
                self.system.discoverable,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let row_y = body.y + 102 + index as i32 * 52;
            ui.label(label, x + 16, row_y + 6, 0x100, tokens.text);
            ui.label(detail, x + 16, row_y + 22, 4, tokens.dim);
            let switch = self.system_switch_rect(index);
            self.switch(ui, switch, value, tokens);
            if index == 0 {
                ui.solid(
                    AetherRect::new(x + 1, row_y + 51, width as u32 - 2, 1),
                    [
                        RULE[0] as f32 / 255.0,
                        RULE[1] as f32 / 255.0,
                        RULE[2] as f32 / 255.0,
                        0.12,
                    ],
                );
            }
        }

        let rows = facts.devices.len().max(1) as i32;
        let list_y = body.y + 208;
        let list = AetherRect::new(x, list_y, width as u32, (34 + rows * 52) as u32);
        self.card(ui, list, tokens);
        self.section(ui, x + 16, list_y + 12, "Paired devices", tokens);
        if facts.devices.is_empty() {
            ui.label(
                if facts.adapter {
                    "No paired devices."
                } else {
                    "No Bluetooth adapter detected."
                },
                x + 16,
                list_y + 40,
                4,
                tokens.dim,
            );
        }
        for (index, device) in facts.devices.iter().enumerate() {
            let row_y = list_y + 32 + index as i32 * 52;
            if let Some(buffer) = self.chrome_round(30, 30, 8, [255, 255, 255, 12]) {
                ui.panel(Some(buffer), x + 16, row_y + 6);
            }
            ui.icon("bluetooth", 16, tokens.text_soft, x + 23, row_y + 13);
            ui.label(device, x + 56, row_y + 6, 0x100, tokens.text);
            ui.label("Connected", x + 56, row_y + 24, 4, tokens.dim);
            self.inline_badge(
                ui,
                x + width - 16,
                row_y + 14,
                "Connected",
                tokens.green,
                tokens,
            );
        }
    }
}

impl App {
    // ----------------------------------------------------------------- radio

    fn radio_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        let tint = [0xff, 0x9a, 0xd5];
        const TRACKS: [(&str, &str, &str); 5] = [
            ("Deep Current", "Tide Ensemble", "04:32"),
            ("Harbour Lights", "Tide Ensemble", "03:58"),
            ("Sandbar", "Low Tide", "05:12"),
            ("Lighthouse", "Tide Ensemble", "04:06"),
            ("Old Pier", "Low Tide", "03:21"),
        ];
        let track = self.system.track.min(TRACKS.len() - 1);
        let (title, artist, duration) = TRACKS[track];
        let x = body.x + 24;
        let center_y = body.y + body.height as i32 / 2;

        let art = AetherRect::new(x, center_y - 60, 96, 96);
        if let Some(buffer) = self.chrome_round(96, 96, 14, [255, 255, 255, 10]) {
            ui.panel(Some(buffer), art.x, art.y);
        }
        if let Some(buffer) = self.chrome_outline(96, 96, 14, [tint[0], tint[1], tint[2], 90]) {
            ui.panel(Some(buffer), art.x, art.y);
        }
        for (size, alpha) in [(64u32, 26u8), (40, 46), (16, 220)] {
            let offset = (96 - size as i32) / 2;
            if let Some(buffer) =
                self.chrome_round(size, size, size / 2, [tint[0], tint[1], tint[2], alpha])
            {
                ui.panel(Some(buffer), art.x + offset, art.y + offset);
            }
        }

        let text_x = art.x + 96 + 20;
        ui.label("AETHER OS SESSIONS", text_x, center_y - 44, 6, tokens.dim);
        ui.label(title, text_x, center_y - 26, 3, tokens.text);
        ui.label(
            format!("{artist} · {duration}"),
            text_x,
            center_y + 2,
            4,
            tokens.dim,
        );

        let progress = AetherRect::new(text_x, center_y + 30, (body.width as i32 - 320) as u32, 4);
        if let Some(buffer) =
            self.chrome_round(progress.width, progress.height, 2, [255, 255, 255, 24])
        {
            ui.panel(Some(buffer), progress.x, progress.y);
        }
        let fill = (progress.width as f32 * 0.38) as u32;
        if let Some(buffer) = self.chrome_round(
            fill.max(4),
            progress.height,
            2,
            [tint[0], tint[1], tint[2], 255],
        ) {
            ui.panel(Some(buffer), progress.x, progress.y);
        }

        for (index, glyph) in ["prev", "pause", "next"].into_iter().enumerate() {
            let rect = self.radio_button_rect(index);
            let is_play = index == 1;
            if is_play {
                if let Some(buffer) =
                    self.chrome_round(40, 40, 20, [tint[0], tint[1], tint[2], 255])
                {
                    ui.panel(Some(buffer), rect.x + 2, rect.y);
                }
                ui.icon(
                    if self.system.playing { "pause" } else { "play" },
                    16,
                    [3, 25, 34, 255],
                    rect.x + 14,
                    rect.y + 12,
                );
            } else {
                ui.icon(glyph, 16, tokens.text_soft, rect.x + 14, rect.y + 12);
            }
        }
        let elapsed = "1:43 / 4:32";
        let width = self.text_width(elapsed, 4) as i32;
        ui.label(
            elapsed,
            body.x + body.width as i32 - 24 - width,
            center_y + 62,
            4,
            tokens.dim,
        );
    }
}

impl App {
    // -------------------------------------------------------------- settings

    fn settings_body(&mut self, ui: &mut Ui, body: AetherRect, tokens: ThemeTokens) {
        // Sidebar: search affordance and 13 tabs.
        let sidebar_width = 220;
        ui.solid(
            AetherRect::new(
                body.x + sidebar_width,
                body.y + 12,
                1,
                body.height.saturating_sub(24),
            ),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );
        let search = AetherRect::new(body.x + 12, body.y + 14, 196, 30);
        if let Some(buffer) = self.chrome_round(search.width, search.height, 7, [255, 255, 255, 10])
        {
            ui.panel(Some(buffer), search.x, search.y);
        }
        ui.icon("search", 13, tokens.dim, search.x + 9, search.y + 8);
        ui.label(
            "Search settings",
            search.x + 28,
            search.y + 8,
            4,
            tokens.dim,
        );

        let tabs = crate::app::system::SETTINGS_TABS;
        let icons = [
            "gear",
            "drop",
            "chart",
            "grid",
            "search",
            "agents",
            "wifi",
            "bluetooth",
            "audio",
            "battery",
            "shield",
            "chart",
            "layers",
            "drop",
        ];
        for (index, label) in tabs.iter().enumerate() {
            let rect = AetherRect::new(body.x + 12, body.y + 56 + index as i32 * 30, 196, 28);
            let active = index == self.system.selected;
            if active {
                if let Some(buffer) =
                    self.chrome_round(rect.width, rect.height, 6, tokens.accent(38))
                {
                    ui.panel(Some(buffer), rect.x, rect.y);
                }
            }
            ui.icon(
                icons[index],
                14,
                if active {
                    tokens.accent(255)
                } else {
                    tokens.dim
                },
                rect.x + 8,
                rect.y + 7,
            );
            ui.label(
                *label,
                rect.x + 30,
                rect.y + 7,
                4,
                if active {
                    tokens.text
                } else {
                    tokens.text_soft
                },
            );
        }

        // Content: title + sections.
        let content = self.settings_content_rect();
        ui.label(
            tabs[self.system.selected.min(tabs.len() - 1)],
            content.x,
            content.y + 2,
            3,
            tokens.text,
        );
        let mut y = content.y + 44;
        for section in self.settings_sections() {
            let height = 36 + section.rows.len() as i32 * 34;
            let card = AetherRect::new(content.x, y, content.width, height as u32);
            self.card(ui, card, tokens);
            self.section(ui, content.x + 14, y + 12, section.title, tokens);
            for (index, row) in section.rows.iter().enumerate() {
                let row_y = y + 36 + index as i32 * 34;
                if index > 0 {
                    ui.solid(
                        AetherRect::new(content.x + 1, row_y, content.width.saturating_sub(2), 1),
                        [
                            RULE[0] as f32 / 255.0,
                            RULE[1] as f32 / 255.0,
                            RULE[2] as f32 / 255.0,
                            0.1,
                        ],
                    );
                }
                ui.label(row.label, content.x + 14, row_y + 9, 0x100, tokens.text);
                match row.switch {
                    Some((on, _)) => {
                        let switch = AetherRect::new(
                            content.x + content.width as i32 - 50,
                            row_y + 7,
                            34,
                            20,
                        );
                        self.switch(ui, switch, on, tokens);
                    }
                    None => {
                        let width = self.text_width(&row.value, 4) as i32;
                        ui.label(
                            &row.value,
                            content.x + content.width as i32 - 14 - width,
                            row_y + 9,
                            4,
                            tokens.text_soft,
                        );
                    }
                }
            }
            y += height + 12;
        }
    }
}
