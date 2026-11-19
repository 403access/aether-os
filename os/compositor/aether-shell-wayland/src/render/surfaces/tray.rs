//! Menu-bar tray popovers: compact Wi-Fi, Bluetooth, battery, sound and
//! calendar panels anchored under their menu-bar button.

use aether_geometry::Rect as AetherRect;
use aether_shell::theme::ThemeTokens;

use crate::app::overlays::menubar::Clock;
use crate::app::tray::TrayId;
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::ui::Ui;

const FILL: [u8; 4] = [7, 39, 51, 245];
const RULE: [u8; 4] = [127, 230, 255, 38];

impl App {
    pub(crate) fn compose_tray(&mut self, ctx: &FrameContext) -> RenderChunk {
        let Some(tray) = self.system.tray else {
            return RenderChunk::default();
        };
        let tokens = ctx.tokens;
        let rect = self.tray_rect(tray);
        let mut ui = Ui::default();
        if let Some(buffer) = self.chrome_shadow(rect.width, rect.height, 12, false) {
            ui.panel(Some(buffer), rect.x - 28, rect.y - 28);
        }
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 12, FILL) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }
        if let Some(buffer) = self.chrome_outline(rect.width, rect.height, 12, tokens.border(60)) {
            ui.panel(Some(buffer), rect.x, rect.y);
        }

        let state = match tray {
            TrayId::Network => {
                if self.system.wifi {
                    "Connected"
                } else {
                    "Off"
                }
            }
            TrayId::Devices => {
                if self.system.bluetooth {
                    "2 connected"
                } else {
                    "Off"
                }
            }
            TrayId::Battery => {
                if self.system.battery.present {
                    "On battery"
                } else {
                    "AC power"
                }
            }
            TrayId::Sound => self.status().audio.as_str(),
            TrayId::Calendar => "",
        };
        ui.icon(
            tray.icon(),
            15,
            tokens.accent(255),
            rect.x + 12,
            rect.y + 13,
        );
        ui.label(tray.title(), rect.x + 34, rect.y + 13, 0x100, tokens.text);
        if !state.is_empty() {
            let width = self.text_width(state, 0) as i32;
            ui.label(
                state,
                rect.x + rect.width as i32 - 14 - width,
                rect.y + 13,
                0,
                tokens.dim,
            );
        }
        ui.solid(
            AetherRect::new(rect.x + 1, rect.y + 38, rect.width.saturating_sub(2), 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );

        match tray {
            TrayId::Network => self.tray_network(&mut ui, rect, tokens),
            TrayId::Devices => self.tray_devices(&mut ui, rect, tokens),
            TrayId::Battery => self.tray_battery(&mut ui, rect, tokens),
            TrayId::Sound => self.tray_sound(&mut ui, rect, tokens),
            TrayId::Calendar => self.tray_calendar(&mut ui, rect, tokens),
        }

        // Footer link.
        let footer_y = rect.y + rect.height as i32 - 34;
        ui.solid(
            AetherRect::new(rect.x + 1, footer_y, rect.width.saturating_sub(2), 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        );
        let label = match tray {
            TrayId::Network => "Network Settings…",
            TrayId::Devices => "Bluetooth Settings…",
            TrayId::Battery => "Battery Settings…",
            TrayId::Sound => "Sound Settings…",
            TrayId::Calendar => "Open Calendar…",
        };
        let width = self.text_width(label, 4) as i32;
        ui.label(
            label,
            rect.x + rect.width as i32 - 14 - width,
            footer_y + 10,
            4,
            tokens.accent(255),
        );

        RenderChunk {
            chrome: ui.chrome,
            labels: ui.labels,
            icons: Vec::new(),
            panels: ui.panels,
            display_icons: ui.icons,
        }
    }

    fn tray_switch(&mut self, ui: &mut Ui, rect: AetherRect, on: bool, tokens: ThemeTokens) {
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

    fn tray_network(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let x = rect.x + 12;
        ui.label("Wi-Fi", x, rect.y + 54, 0x100, tokens.text);
        ui.label(
            if self.system.wifi { "On" } else { "Off" },
            x,
            rect.y + 70,
            4,
            tokens.dim,
        );
        let switch = self.tray_switch_rect();
        self.tray_switch(ui, switch, self.system.wifi, tokens);

        ui.label("Connected", x, rect.y + 92, 6, tokens.dim);
        if let Some(primary) = self.system.network.primary() {
            ui.label(&primary.name, x, rect.y + 110, 0x100, tokens.text);
            ui.label(
                format!(
                    "{} · {}",
                    if primary.wireless {
                        "Wi-Fi"
                    } else {
                        "Ethernet"
                    },
                    primary.state
                ),
                x,
                rect.y + 126,
                4,
                tokens.dim,
            );
        } else {
            ui.label("No interfaces", x, rect.y + 110, 4, tokens.dim);
        }

        ui.label("Known networks", x, rect.y + 156, 6, tokens.dim);
        for (index, interface) in self.system.network.interfaces.iter().take(3).enumerate() {
            let row_y = rect.y + 176 + index as i32 * 36;
            ui.icon(
                if interface.wireless { "wifi" } else { "globe" },
                13,
                tokens.text_soft,
                x,
                row_y + 4,
            );
            ui.label(&interface.name, x + 20, row_y, 4, tokens.text_soft);
            let width = self.text_width(&interface.state, 6) as i32;
            ui.label(
                &interface.state,
                rect.x + rect.width as i32 - 14 - width,
                row_y + 2,
                6,
                tokens.dim,
            );
        }
    }

    fn tray_devices(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let x = rect.x + 12;
        ui.label("Bluetooth", x, rect.y + 54, 0x100, tokens.text);
        ui.label(
            if self.system.bluetooth { "On" } else { "Off" },
            x,
            rect.y + 70,
            4,
            tokens.dim,
        );
        let switch = self.tray_switch_rect();
        self.tray_switch(ui, switch, self.system.bluetooth, tokens);
        ui.label("Paired devices", x, rect.y + 100, 6, tokens.dim);
        ui.label(
            if self.system.bluetooth_facts.devices.is_empty() {
                "No paired devices"
            } else {
                "Connected accessories"
            },
            x,
            rect.y + 120,
            4,
            tokens.dim,
        );
    }

    fn tray_battery(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let x = rect.x + 12;
        let capacity = self.system.battery.capacity;
        ui.label(
            if self.system.battery.present {
                format!("{capacity}%")
            } else {
                "AC".to_string()
            },
            x,
            rect.y + 52,
            2,
            tokens.text,
        );
        ui.label(
            if self.system.battery.present {
                "About 4h 12m remaining"
            } else {
                "No battery detected"
            },
            x,
            rect.y + 86,
            4,
            tokens.dim,
        );
        let bar = AetherRect::new(x, rect.y + 110, rect.width - 24, 8);
        if let Some(buffer) = self.chrome_round(bar.width, bar.height, 4, [255, 255, 255, 22]) {
            ui.panel(Some(buffer), bar.x, bar.y);
        }
        let fill = (bar.width as f32 * (capacity as f32 / 100.0)) as u32;
        if fill > 0 {
            if let Some(buffer) = self.chrome_round(fill, bar.height, 4, tokens.green) {
                ui.panel(Some(buffer), bar.x, bar.y);
            }
        }
        ui.label("Energy mode", x, rect.y + 132, 6, tokens.dim);
        for (index, label) in ["Low", "Balanced", "High"].iter().enumerate() {
            let segment = self.tray_energy_rect(index);
            let active = self.system.energy == index;
            let fill = if active {
                tokens.accent(255)
            } else {
                [255, 255, 255, 14]
            };
            if let Some(buffer) = self.chrome_round(segment.width, segment.height, 6, fill) {
                ui.panel(Some(buffer), segment.x, segment.y);
            }
            ui.label(
                *label,
                segment.x + 8,
                segment.y + 7,
                4,
                if active {
                    [3, 25, 34, 255]
                } else {
                    tokens.text_soft
                },
            );
        }
    }

    fn tray_sound(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let x = rect.x + 12;
        ui.label("Output", x, rect.y + 54, 6, tokens.dim);
        ui.label(
            self.status().audio.trim_start_matches("audio ").to_string(),
            x,
            rect.y + 74,
            0x100,
            tokens.text,
        );
        ui.label("Volume", x, rect.y + 100, 6, tokens.dim);
        let bar = self.tray_volume_rect();
        if let Some(buffer) = self.chrome_round(bar.width, bar.height, 8, [255, 255, 255, 20]) {
            ui.panel(Some(buffer), bar.x, bar.y);
        }
        let fill = (bar.width as f32 * 0.62) as u32;
        if let Some(buffer) = self.chrome_round(fill, bar.height, 8, tokens.accent(255)) {
            ui.panel(Some(buffer), bar.x, bar.y);
        }
        ui.icon("audio", 14, tokens.text_soft, x, rect.y + 210);
        ui.label(
            "Harbour speakers",
            x + 22,
            rect.y + 210,
            4,
            tokens.text_soft,
        );
    }

    fn tray_calendar(&mut self, ui: &mut Ui, rect: AetherRect, tokens: ThemeTokens) {
        let clock = Clock::now();
        let view = clock.month_start();
        let x = rect.x + 12;
        ui.label(view.month_label(), x, rect.y + 52, 0x100, tokens.text);
        ui.label("MONTH", x, rect.y + 70, 6, tokens.dim);

        let grid_y = rect.y + 92;
        let cell = (rect.width as i32 - 24) / 7;
        for (index, name) in ["S", "M", "T", "W", "T", "F", "S"].iter().enumerate() {
            ui.label(
                *name,
                x + index as i32 * cell + cell / 2 - 3,
                grid_y,
                6,
                tokens.dim,
            );
        }
        let first = view.first_weekday() as i32;
        let days = view.days_in_month() as i32;
        let start = view.add_days(-first);
        for index in 0..42 {
            let date = start.add_days(index);
            let in_month = index >= first && index < first + days;
            let column = index % 7;
            let row = index / 7;
            let cell_x = x + column * cell;
            let cell_y = grid_y + 18 + row * 26;
            let day = date.day.to_string();
            let width = self.text_width(&day, 4) as i32;
            if date.day_key() == clock.day_key() {
                if let Some(buffer) = self.chrome_round(cell as u32 - 4, 24, 6, tokens.accent(255))
                {
                    ui.panel(Some(buffer), cell_x + 2, cell_y);
                }
                ui.label(
                    day,
                    cell_x + (cell - width) / 2,
                    cell_y + 6,
                    4,
                    [3, 25, 34, 255],
                );
            } else {
                let color = if in_month {
                    tokens.text_soft
                } else {
                    tokens.dim
                };
                ui.label(day, cell_x + (cell - width) / 2, cell_y + 6, 4, color);
            }
        }
    }
}
