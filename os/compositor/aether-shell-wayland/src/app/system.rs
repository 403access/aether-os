//! First-party app windows: geometry, state and per-app actions.
//!
//! These are compositor-owned surfaces (no Wayland client), drawn with the
//! same window chrome as client toplevels. Every window uses the demo's
//! `.os-window` box: 16 px from the work-area edge, below the 38 px menu bar,
//! 1248 × 664 at 1280 × 800.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;
use super::system_info::{self, BatteryFacts, BluetoothFacts, NetworkFacts, SystemFacts};

/// Demo window box at 1280 × 800.
pub const APP_WIDTH: u32 = 1248;
pub const APP_HEIGHT: u32 = 664;
pub const TITLEBAR_HEIGHT: i32 = 40;

#[allow(dead_code)] // variants land one commit at a time
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemApp {
    About,
    Network,
    Battery,
    Devices,
    Radio,
    Settings,
    Studio,
    Compare,
}

impl SystemApp {
    #[allow(dead_code)]
    pub const ALL: [Self; 8] = [
        Self::About,
        Self::Network,
        Self::Battery,
        Self::Devices,
        Self::Radio,
        Self::Settings,
        Self::Studio,
        Self::Compare,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::About => "About Aether OS",
            Self::Network => "Network",
            Self::Battery => "Battery",
            Self::Devices => "Devices",
            Self::Radio => "Tide Radio",
            Self::Settings => "Settings",
            Self::Studio => "App Studio",
            Self::Compare => "Data Cloud Compare",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::About => "",
            Self::Network => "Wi-Fi, VPN, and local network",
            Self::Battery => "Power, health, and energy",
            Self::Devices => "Wireless accessories",
            Self::Radio => "Deep Current · live stream",
            Self::Settings => "General",
            Self::Studio => "GUI builder · projects and live system variables",
            Self::Compare => "Two-pane comparison",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::About => "drop",
            Self::Network => "wifi",
            Self::Battery => "battery",
            Self::Devices => "bluetooth",
            Self::Radio => "audio",
            Self::Settings => "gear",
            Self::Studio => "layers",
            Self::Compare => "explorer",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SystemState {
    pub app: Option<SystemApp>,
    /// Selected sidebar row in Settings / App Studio.
    pub selected: usize,
    /// Radio transport state.
    pub playing: bool,
    /// Track index in the Tide Radio queue.
    pub track: usize,
    /// Wi-Fi / Bluetooth switches.
    pub wifi: bool,
    pub vpn: bool,
    pub bluetooth: bool,
    pub discoverable: bool,
    /// Energy mode (0 = low power, 1 = balanced, 2 = high performance).
    pub energy: usize,
    pub charging_optimised: bool,
    /// Top CPU consumers, sampled when the window opens.
    pub consumers: Vec<(String, f32)>,
    /// Cached host facts, refreshed when a window opens.
    pub network: NetworkFacts,
    pub battery: BatteryFacts,
    pub bluetooth_facts: BluetoothFacts,
    pub facts: SystemFacts,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            wifi: true,
            vpn: false,
            bluetooth: true,
            discoverable: false,
            playing: true,
            energy: 1,
            charging_optimised: true,
            ..Default::default()
        }
    }

    pub fn refresh(&mut self) {
        self.network = system_info::network();
        self.battery = system_info::battery();
        self.bluetooth_facts = system_info::bluetooth();
        self.facts = system_info::system();
        self.wifi = self.network.wireless();
        self.bluetooth = self.bluetooth_facts.adapter;
    }
}

impl App {
    pub(crate) fn open_system(&mut self, app: SystemApp) {
        self.shortcuts_open = false;
        self.menu_open = None;
        self.close_task_manager();
        self.close_sandboxes();
        self.close_calendar();
        self.shell.close_audio();
        self.shell.close_themes();
        self.shell.close_remote();
        self.shell.close_explorer();
        self.shell.close_chat();
        self.shell.close_overlay();
        self.system.app = Some(app);
        self.system.refresh();
        self.system.consumers = self.top_consumers();
        println!("aether-shell: system app {}", app.title());
        self.mark_full_damage();
    }

    pub fn system_open(&self) -> bool {
        self.system.app.is_some()
    }

    pub(crate) fn close_system(&mut self) {
        if self.system.app.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn system_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = APP_WIDTH.min(bounds.width.saturating_sub(32));
        let height = APP_HEIGHT.min(bounds.height.saturating_sub(54 + 70));
        AetherRect::new(bounds.x + 16, bounds.y + 54, width, height)
    }

    pub(crate) fn system_body_rect(&self) -> AetherRect {
        let rect = self.system_rect();
        AetherRect::new(
            rect.x + 1,
            rect.y + TITLEBAR_HEIGHT,
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(TITLEBAR_HEIGHT as u32 + 1),
        )
    }

    pub(crate) fn system_light_rects(&self) -> [AetherRect; 3] {
        let rect = self.system_rect();
        [
            AetherRect::new(rect.x + 11, rect.y + 14, 11, 11),
            AetherRect::new(rect.x + 28, rect.y + 14, 11, 11),
            AetherRect::new(rect.x + 45, rect.y + 14, 11, 11),
        ]
    }

    pub(crate) fn system_action_at(&self, point: AetherPoint) -> Option<SystemAction> {
        let app = self.system.app?;
        let rect = self.system_rect();
        if self.system_light_rects()[0].contains(point) {
            return Some(SystemAction::Close);
        }
        if !rect.contains(point) {
            return Some(SystemAction::Outside);
        }
        match app {
            SystemApp::About => None,
            SystemApp::Network => {
                if self.system_switch_rect(0).contains(point) {
                    Some(SystemAction::ToggleWifi)
                } else if self.system_switch_rect(1).contains(point) {
                    Some(SystemAction::ToggleVpn)
                } else {
                    None
                }
            }
            SystemApp::Battery => (0..3)
                .find(|index| self.energy_segment_rect(*index).contains(point))
                .map(SystemAction::Energy),
            SystemApp::Devices => {
                if self.system_switch_rect(0).contains(point) {
                    Some(SystemAction::ToggleBluetooth)
                } else if self.system_switch_rect(1).contains(point) {
                    Some(SystemAction::ToggleDiscoverable)
                } else {
                    None
                }
            }
            SystemApp::Radio => {
                if self.radio_button_rect(0).contains(point) {
                    Some(SystemAction::PreviousTrack)
                } else if self.radio_button_rect(1).contains(point) {
                    Some(SystemAction::TogglePlay)
                } else if self.radio_button_rect(2).contains(point) {
                    Some(SystemAction::NextTrack)
                } else {
                    None
                }
            }
            SystemApp::Settings => self.settings_tab_at(point).map(SystemAction::SettingsTab),
            SystemApp::Studio => self.studio_node_at(point).map(SystemAction::StudioNode),
            SystemApp::Compare => None,
        }
    }

    pub(crate) fn activate_system_action(&mut self, action: SystemAction) {
        match action {
            SystemAction::Close => self.close_system(),
            SystemAction::Outside => {}
            SystemAction::ToggleWifi => {
                self.system.wifi = !self.system.wifi;
                println!("aether-shell: wifi {}", self.system.wifi);
            }
            SystemAction::ToggleVpn => {
                self.system.vpn = !self.system.vpn;
            }
            SystemAction::ToggleBluetooth => {
                self.system.bluetooth = !self.system.bluetooth;
            }
            SystemAction::ToggleDiscoverable => {
                self.system.discoverable = !self.system.discoverable;
            }
            SystemAction::Energy(index) => {
                self.system.energy = index;
            }
            SystemAction::TogglePlay => {
                self.system.playing = !self.system.playing;
            }
            SystemAction::PreviousTrack => {
                self.system.track = self.system.track.saturating_sub(1);
            }
            SystemAction::NextTrack => {
                self.system.track = (self.system.track + 1).min(4);
            }
            SystemAction::SettingsTab(index) => {
                self.system.selected = index;
            }
            SystemAction::StudioNode(index) => {
                self.system.selected = index;
            }
        }
        self.mark_full_damage();
    }

    pub(crate) fn handle_system_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        if keysym == escape {
            self.close_system();
            return;
        }
        let space: Keysym = keysyms::KEY_space.into();
        if self.system.app == Some(SystemApp::Radio) && keysym == space {
            self.system.playing = !self.system.playing;
            self.mark_full_damage();
        }
    }

    /// Top CPU consumers from one /proc sample, used by Battery and Settings.
    fn top_consumers(&mut self) -> Vec<(String, f32)> {
        let snapshot = self.task_monitor.sample();
        let mut rows: Vec<(String, f32)> = snapshot
            .processes
            .iter()
            .map(|process| (process.name.clone(), process.cpu_percent))
            .collect();
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        rows.truncate(4);
        rows
    }

    pub(crate) fn mark_system_damage(&mut self) {
        if self.system_open() {
            self.mark_damage_rect(self.system_rect());
        }
    }

    /// Switch geometry shared by Network and Devices: 34 × 20 capsule in the
    /// card below the 64 px header (rows are 52 px apart).
    pub(crate) fn system_switch_rect(&self, index: usize) -> AetherRect {
        let body = self.system_body_rect();
        let card_y = body.y + 16 + 64 + 12;
        AetherRect::new(
            body.x + body.width as i32 - 50,
            card_y + 10 + index as i32 * 52 + 16,
            34,
            20,
        )
    }

    pub(crate) fn energy_segment_rect(&self, index: usize) -> AetherRect {
        let body = self.system_body_rect();
        let card_x = body.x + 16;
        let card_width = body.width as i32 - 32;
        let segment = (card_width - 16) / 3;
        AetherRect::new(
            card_x + 8 + index as i32 * segment,
            body.y + 124 + 34,
            (segment - 4) as u32,
            30,
        )
    }

    pub(crate) fn radio_button_rect(&self, index: usize) -> AetherRect {
        let body = self.system_body_rect();
        AetherRect::new(
            body.x + body.width as i32 / 2 - 66 + index as i32 * 44,
            body.y + 330,
            40,
            40,
        )
    }

    pub(crate) fn settings_tab_at(&self, point: AetherPoint) -> Option<usize> {
        let body = self.system_body_rect();
        let x = body.x + 16;
        (0..SETTINGS_TABS.len()).find(|index| {
            AetherRect::new(x, body.y + 16 + *index as i32 * 34, 200, 30).contains(point)
        })
    }

    pub(crate) fn studio_node_at(&self, point: AetherPoint) -> Option<usize> {
        let body = self.system_body_rect();
        (0..4).find(|index| {
            AetherRect::new(
                body.x + 40 + (*index as i32 % 2) * 180,
                body.y + 60 + (*index as i32 / 2) * 120,
                150,
                84,
            )
            .contains(point)
        })
    }
}

pub const SETTINGS_TABS: [&str; 14] = [
    "General",
    "Appearance",
    "Displays",
    "Desktop & Dock",
    "Search & Commands",
    "Accessibility",
    "Network",
    "Bluetooth",
    "Sound",
    "Battery",
    "Privacy & Security",
    "Screen Time",
    "Storage",
    "About",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemAction {
    Close,
    Outside,
    ToggleWifi,
    ToggleVpn,
    ToggleBluetooth,
    ToggleDiscoverable,
    Energy(usize),
    TogglePlay,
    PreviousTrack,
    NextTrack,
    SettingsTab(usize),
    StudioNode(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_has_a_route() {
        assert_eq!(SystemApp::ALL.len(), 8);
        for app in SystemApp::ALL {
            assert!(!app.title().is_empty());
            assert!(!app.icon().is_empty());
        }
    }

    #[test]
    fn settings_tabs_match_the_demo() {
        assert_eq!(SETTINGS_TABS.len(), 14);
        assert_eq!(SETTINGS_TABS[0], "General");
        assert_eq!(SETTINGS_TABS[13], "About");
    }
}
