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
use super::tray::TrayId;

/// Demo window box at 1280 × 800.
pub const APP_WIDTH: u32 = 1248;
pub const APP_HEIGHT: u32 = 730;
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
    /// Switches that are view state only (automatic updates, indicators, clock).
    pub settings_toggles: [bool; 7],
    pub settings_scroll: usize,
    /// Open menu-bar tray popover.
    pub tray: Option<TrayId>,
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
            settings_toggles: [true, true, false, false, false, true, false],
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
        self.close_tray();
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

    /// True when the pointer is inside `rect` (chrome hover state).
    pub(crate) fn pointer_in(&self, rect: AetherRect) -> bool {
        let point = AetherPoint::new(
            self.pointer_pos.x.round() as i32,
            self.pointer_pos.y.round() as i32,
        );
        rect.contains(point)
    }

    pub(crate) fn system_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = APP_WIDTH.min(bounds.width.saturating_sub(32));
        let height = APP_HEIGHT.min(bounds.height.saturating_sub(54 + 16));
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
            SystemApp::Settings => self
                .settings_switch_at(point)
                .map(SystemAction::SettingsSwitch)
                .or_else(|| self.settings_tab_at(point).map(SystemAction::SettingsTab)),
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
                self.system.settings_scroll = 0;
            }
            SystemAction::SettingsSwitch(action) => self.toggle_settings_action(action),
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

    /// Advance the tide rim while any animated surface is visible. The winit
    /// event loop polls continuously but only presents when damage exists, so
    /// this caps the animation at ~12 fps instead of redrawing every frame.
    pub fn tick_waves(&mut self) {
        if !self.shell.theme_settings().effects {
            return;
        }
        let animated = self.system_open()
            || self.calendar_open()
            || self.shell.explorer_open()
            || self.task_manager_open()
            || self.shell.overlay().is_some()
            || self.shell.focused_window().is_some();
        if !animated {
            return;
        }
        let now = std::time::Instant::now();
        if self
            .wave_last
            .is_some_and(|last| now.duration_since(last).as_millis() < 83)
        {
            return;
        }
        self.wave_last = Some(now);
        if self.system_open() {
            self.mark_system_damage();
        }
        if self.calendar_open() {
            self.mark_calendar_damage();
        }
        if self.shell.explorer_open() {
            self.mark_damage_rect(self.shell.explorer_rect());
        }
        if self.task_manager_open() {
            self.mark_task_damage();
        }
        if self.shell.overlay().is_some() {
            self.mark_overlay_damage();
        }
        if let Some(id) = self.shell.focused_window() {
            if let Some(rect) = self.window_rect(id) {
                self.mark_damage_rect(rect);
            }
        }
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
        // Controls sit under the track text, which starts after the 96 px art.
        let text_x = body.x + 16 + 8 + 96 + 20;
        let row_y = body.y + body.height as i32 / 2 - 60 + 152;
        AetherRect::new(text_x + index as i32 * 56, row_y, 44, 40)
    }

    /// Centred settings pane: the demo caps it at 640 px inside the content.
    pub(crate) fn settings_pane_rect(&self) -> AetherRect {
        let content = self.settings_content_rect();
        let width = content.width.min(640);
        AetherRect::new(
            content.x + (content.width as i32 - width as i32) / 2,
            content.y,
            width,
            content.height,
        )
    }

    /// Settings sidebar geometry, shared by rendering and hit-testing.
    pub(crate) fn settings_nav(&self) -> SettingsNav {
        let body = self.system_body_rect();
        let sidebar = body.width.min(238) as i32;
        let mut y = body.y + 103;
        let mut tabs = vec![AetherRect::new(0, 0, 0, 0); SETTINGS_TABS.len()];
        let mut groups = Vec::new();
        for (label, indices) in SETTINGS_GROUPS {
            let label_rect = AetherRect::new(body.x + 16, y, (sidebar - 24) as u32, 15);
            y += 18;
            let first = tabs.len();
            for index in indices {
                tabs[*index] = AetherRect::new(body.x + 8, y, (sidebar - 16) as u32, 30);
                y += 30;
            }
            groups.push((label, label_rect, first));
            y += 16;
        }
        SettingsNav {
            search: AetherRect::new(body.x + 10, body.y + 11, 217, 30),
            profile: AetherRect::new(body.x + 10, body.y + 49, 217, 48),
            footer: AetherRect::new(
                body.x,
                body.y + body.height as i32 - 35,
                sidebar.saturating_sub(1) as u32,
                35,
            ),
            divider: AetherRect::new(
                body.x + sidebar,
                body.y + 12,
                1,
                body.height.saturating_sub(24),
            ),
            groups,
            tabs,
        }
    }

    pub(crate) fn settings_tab_at(&self, point: AetherPoint) -> Option<usize> {
        self.settings_nav()
            .tabs
            .iter()
            .position(|rect| rect.contains(point))
    }

    /// Real account shown in the Settings profile card.
    pub(crate) fn settings_user(&self) -> (String, String) {
        let raw = std::env::var("AETHER_USER_NAME")
            .ok()
            .or_else(|| std::env::var("USER").ok())
            .or_else(|| std::env::var("LOGNAME").ok())
            .unwrap_or_else(|| "aether".into());
        let name = raw
            .split(['_', '-'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let name = if name.is_empty() {
            "Aether".into()
        } else {
            name
        };
        let initials: String = name
            .split_whitespace()
            .filter_map(|part| part.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        (name, initials)
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

/// Rects for the Settings sidebar, derived once per interaction/frame.
pub(crate) struct SettingsNav {
    pub search: AetherRect,
    pub profile: AetherRect,
    pub footer: AetherRect,
    pub divider: AetherRect,
    /// (label, label rect, first tab index)
    pub groups: Vec<(&'static str, AetherRect, usize)>,
    pub tabs: Vec<AetherRect>,
}

/// Sidebar groups, matching `.os-settings-nav__group` in the demo.
pub(crate) const SETTINGS_GROUPS: [(&str, &[usize]); 3] = [
    ("System", &[0, 1, 2, 3, 4, 5]),
    ("Connectivity", &[6, 7, 8]),
    ("Power & privacy", &[9, 10, 11, 12]),
];

/// One-line description under each pane title, mirroring the demo.
pub(crate) const SETTINGS_SUBTITLES: [&str; 13] = [
    "System version, updates, language, and startup.",
    "Pick a theme, then shape the current and the calm.",
    "Resolution, brightness, colour, and arrangement.",
    "Window layout, the dock, and what the desktop shows.",
    "Choose the sources, groups, and conditions the command bar uses.",
    "Vision, motion, hearing, and keyboard comfort.",
    "Wi-Fi, VPN, and the local archive route.",
    "Wireless accessories and nearby devices.",
    "Output, volume, and alert sounds.",
    "Charge, health, and energy mode.",
    "Alerts, previews, and focus.",
    "Encryption, firewall, location, and app access.",
    "What fills the archive volume and how to reclaim space.",
];

pub const SETTINGS_TABS: [&str; 13] = [
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
    "Notifications",
    "Privacy & Security",
    "Storage",
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
    SettingsSwitch(SettingsAction),
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
        assert_eq!(SETTINGS_TABS.len(), 13);
        assert_eq!(SETTINGS_TABS[0], "General");
        assert_eq!(SETTINGS_TABS[10], "Notifications");
        assert_eq!(SETTINGS_TABS[12], "Storage");
    }
}

fn start_section(sections: &mut Vec<SettingsSection>, title: &'static str) -> usize {
    sections.push(SettingsSection {
        title,
        rows: Vec::new(),
    });
    sections.len() - 1
}

/// One row in a Settings section: label left, value or switch right.
#[derive(Clone, Debug)]
pub struct SettingsRow {
    pub label: &'static str,
    pub value: String,
    pub switch: Option<(bool, SettingsAction)>,
    /// Optional second line, mirroring `.os-settings-row__text small`.
    pub description: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub title: &'static str,
    pub rows: Vec<SettingsRow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    Theme(aether_shell::theme::ThemeField),
    State(usize),
    /// Show or hide the desktop dock (off by default, like the demo).
    Dock,
}

impl App {
    /// Data for the selected Settings tab. Shared by the renderer and hit
    /// testing so switches cannot drift from what is drawn.
    pub(crate) fn settings_sections(&self) -> Vec<SettingsSection> {
        let tab = self.system.selected.min(SETTINGS_TABS.len() - 1);
        let facts = &self.system.facts;
        let theme = *self.shell.theme_settings();
        let output = self.shell.primary_output();
        let mut sections: Vec<SettingsSection> = Vec::new();
        let push = |sections: &mut Vec<SettingsSection>,
                    index: usize,
                    label: &'static str,
                    value: String| {
            sections[index].rows.push(SettingsRow {
                label,
                value,
                switch: None,
                description: None,
            });
        };
        let push_desc = |sections: &mut Vec<SettingsSection>,
                         index: usize,
                         label: &'static str,
                         value: String,
                         description: &'static str| {
            sections[index].rows.push(SettingsRow {
                label,
                value,
                switch: None,
                description: Some(description),
            });
        };
        let toggle = |sections: &mut Vec<SettingsSection>,
                      index: usize,
                      label: &'static str,
                      value: &'static str,
                      on: bool,
                      action: SettingsAction| {
            sections[index].rows.push(SettingsRow {
                label,
                value: value.to_string(),
                switch: Some((on, action)),
                description: None,
            });
        };
        let toggle_desc = |sections: &mut Vec<SettingsSection>,
                           index: usize,
                           label: &'static str,
                           value: &'static str,
                           on: bool,
                           action: SettingsAction,
                           description: &'static str| {
            sections[index].rows.push(SettingsRow {
                label,
                value: value.to_string(),
                switch: Some((on, action)),
                description: Some(description),
            });
        };

        match tab {
            0 => {
                let about = start_section(&mut sections, "About");
                push_desc(
                    &mut sections,
                    about,
                    "Aether OS",
                    "0.1.0-alpha".into(),
                    "Concept build · Water design system",
                );
                push(
                    &mut sections,
                    about,
                    "Host",
                    if facts.hostname.is_empty() {
                        "—".into()
                    } else {
                        facts.hostname.clone()
                    },
                );
                push(&mut sections, about, "Kernel", facts.kernel.clone());
                push(&mut sections, about, "Processor", facts.cpu.clone());
                push_desc(
                    &mut sections,
                    about,
                    "Memory",
                    if facts.memory_gb > 0.0 {
                        format!("{:.1} GB", facts.memory_gb)
                    } else {
                        "—".into()
                    },
                    "Unified memory",
                );
                let update = start_section(&mut sections, "Software update");
                toggle_desc(
                    &mut sections,
                    update,
                    "Automatic updates",
                    "Check on session start",
                    self.system.settings_toggles[0],
                    SettingsAction::State(0),
                    "Download and install security updates overnight.",
                );
                push(&mut sections, update, "Last checked", "Today".into());
                let region = start_section(&mut sections, "Language & Region");
                push_desc(
                    &mut sections,
                    region,
                    "Language",
                    "English (UK)".into(),
                    "Used across the desktop and apps.",
                );
                push_desc(
                    &mut sections,
                    region,
                    "Region",
                    "Europe".into(),
                    "Formats dates, numbers, and currency.",
                );
            }
            1 => {
                let appearance = start_section(&mut sections, "Appearance");
                push(
                    &mut sections,
                    appearance,
                    "Theme",
                    theme.mode.label().to_string(),
                );
                toggle(
                    &mut sections,
                    appearance,
                    "Reduce motion",
                    "Use the same theme with calm motion",
                    !theme.effects,
                    SettingsAction::Theme(aether_shell::theme::ThemeField::Effects),
                );
                let effects = start_section(&mut sections, "Water effects");
                toggle_desc(
                    &mut sections,
                    effects,
                    "Effects",
                    "Ripples and shimmer",
                    theme.effects,
                    SettingsAction::Theme(aether_shell::theme::ThemeField::Effects),
                    "Master switch for ripples, shimmer, and window waves.",
                );
                toggle_desc(
                    &mut sections,
                    effects,
                    "Cursor drips",
                    "Soft rings follow the pointer",
                    theme.drips,
                    SettingsAction::Theme(aether_shell::theme::ThemeField::Drips),
                    "Small, soft rings trail your pointer.",
                );
                toggle_desc(
                    &mut sections,
                    effects,
                    "Caustic shimmer",
                    "Light patterns behind the app",
                    theme.caustics,
                    SettingsAction::Theme(aether_shell::theme::ThemeField::Caustics),
                    "Slow light patterns drift behind the app.",
                );
                toggle(
                    &mut sections,
                    effects,
                    "Glass surfaces",
                    "Frosted panels over the sea",
                    theme.glass,
                    SettingsAction::Theme(aether_shell::theme::ThemeField::Glass),
                );
                let look = start_section(&mut sections, "Look");
                push(
                    &mut sections,
                    look,
                    "Water tone",
                    theme.tone.label().to_string(),
                );
                push(
                    &mut sections,
                    look,
                    "Flow speed",
                    theme.speed.label().to_string(),
                );
            }
            2 => {
                let display = start_section(&mut sections, "Display");
                push(
                    &mut sections,
                    display,
                    "Output",
                    output
                        .as_ref()
                        .map(|output| output.name.clone())
                        .unwrap_or_else(|| "—".into()),
                );
                push(
                    &mut sections,
                    display,
                    "Resolution",
                    output
                        .as_ref()
                        .map(|output| format!("{} × {}", output.rect.width, output.rect.height))
                        .unwrap_or_else(|| "—".into()),
                );
                push(
                    &mut sections,
                    display,
                    "Refresh rate",
                    output
                        .as_ref()
                        .map(|output| format!("{} Hz", output.refresh_hz))
                        .unwrap_or_else(|| "—".into()),
                );
                push(
                    &mut sections,
                    display,
                    "Scale",
                    output
                        .as_ref()
                        .map(|output| format!("{:.2}×", output.scale))
                        .unwrap_or_else(|| "—".into()),
                );
            }
            3 => {
                let dock = start_section(&mut sections, "Desktop & Dock");
                push(&mut sections, dock, "Position", "Bottom".into());
                push_desc(
                    &mut sections,
                    dock,
                    "Size",
                    "439 × 62".into(),
                    "Scale the dock icons.",
                );
                toggle_desc(
                    &mut sections,
                    dock,
                    "Show dock",
                    "Applications along the bottom edge",
                    self.dock_enabled,
                    SettingsAction::Dock,
                    "Display the application dock on the desktop.",
                );
                toggle(
                    &mut sections,
                    dock,
                    "Running indicators",
                    "Dot under open apps",
                    self.system.settings_toggles[1],
                    SettingsAction::State(1),
                );
                push(
                    &mut sections,
                    dock,
                    "Workspaces",
                    "Keyboard and palette".into(),
                );
                let bar = start_section(&mut sections, "Menu bar");
                toggle_desc(
                    &mut sections,
                    bar,
                    "24-hour clock",
                    "Show 17:47 instead of 05:47 PM",
                    self.system.settings_toggles[2],
                    SettingsAction::State(2),
                    "Show 18:40 instead of 6:40 PM.",
                );
                push(&mut sections, bar, "Meters", "CPU · RAM · storage".into());
            }
            4 => {
                let palette = start_section(&mut sections, "Command palette");
                push(&mut sections, palette, "Shortcut", "Super+Space".into());
                push(&mut sections, palette, "Prefix · commands", ">".into());
                push(&mut sections, palette, "Prefix · agents", "@".into());
                push(&mut sections, palette, "Prefix · files", "/".into());
                let search = start_section(&mut sections, "Search");
                push(&mut sections, search, "Provider", "aether-index".into());
                push(
                    &mut sections,
                    search,
                    "Ranking",
                    "BM25F · fuzzy · prefix".into(),
                );
            }
            5 => {
                let motion = start_section(&mut sections, "Motion");
                toggle_desc(
                    &mut sections,
                    motion,
                    "Reduce motion",
                    "Disable ambient and interface motion",
                    self.system.settings_toggles[3],
                    SettingsAction::State(3),
                    "Turn off window waves and animated transitions.",
                );
                let vision = start_section(&mut sections, "Vision");
                push(&mut sections, vision, "Text size", "Comfortable".into());
                push(&mut sections, vision, "Focus ring", "Accent".into());
                push(
                    &mut sections,
                    vision,
                    "Contrast",
                    "Design-system defaults".into(),
                );
            }
            6 => {
                let network = start_section(&mut sections, "Network");
                if let Some(primary) = self.system.network.primary() {
                    push(
                        &mut sections,
                        network,
                        "Interface",
                        format!(
                            "{} · {}",
                            primary.name,
                            if primary.wireless {
                                "Wi-Fi"
                            } else {
                                "Ethernet"
                            }
                        ),
                    );
                    push(&mut sections, network, "MAC address", primary.mac.clone());
                } else {
                    push(&mut sections, network, "Interface", "—".into());
                }
                push(
                    &mut sections,
                    network,
                    "IP address",
                    self.system.network.ip.clone().unwrap_or_else(|| "—".into()),
                );
                push(
                    &mut sections,
                    network,
                    "Router",
                    self.system
                        .network
                        .gateway
                        .clone()
                        .unwrap_or_else(|| "—".into()),
                );
                push(
                    &mut sections,
                    network,
                    "DNS",
                    if self.system.network.dns.is_empty() {
                        "—".into()
                    } else {
                        self.system.network.dns.join(", ")
                    },
                );
            }
            7 => {
                let bluetooth = start_section(&mut sections, "Bluetooth");
                push(
                    &mut sections,
                    bluetooth,
                    "Adapter",
                    if self.system.bluetooth_facts.adapter {
                        "Present".into()
                    } else {
                        "Not detected".into()
                    },
                );
                toggle(
                    &mut sections,
                    bluetooth,
                    "Discoverable",
                    "Let nearby devices find this machine",
                    self.system.discoverable,
                    SettingsAction::State(4),
                );
                push(&mut sections, bluetooth, "Paired devices", "0".into());
            }
            8 => {
                let sound = start_section(&mut sections, "Sound");
                push(&mut sections, sound, "Output", self.status().audio.clone());
                push(
                    &mut sections,
                    sound,
                    "Presets",
                    "Studio · Hall · Deep".into(),
                );
                push(&mut sections, sound, "Inputs", "Microphone gate".into());
            }
            9 => {
                let battery = start_section(&mut sections, "Battery");
                push(
                    &mut sections,
                    battery,
                    "Charge",
                    if self.system.battery.present {
                        format!("{}%", self.system.battery.capacity)
                    } else {
                        "AC power".into()
                    },
                );
                push(
                    &mut sections,
                    battery,
                    "Condition",
                    if self.system.battery.health.is_empty() {
                        "—".into()
                    } else {
                        self.system.battery.health.clone()
                    },
                );
                push(
                    &mut sections,
                    battery,
                    "Energy mode",
                    ["Low power", "Balanced", "High performance"][self.system.energy.min(2)].into(),
                );
            }
            10 => {
                let notifications = start_section(&mut sections, "Notifications");
                toggle(
                    &mut sections,
                    notifications,
                    "Allow notifications",
                    "Show banners and alerts from apps",
                    self.system.settings_toggles[5],
                    SettingsAction::State(5),
                );
                push(
                    &mut sections,
                    notifications,
                    "Show previews",
                    "Always".into(),
                );
                toggle(
                    &mut sections,
                    notifications,
                    "Do Not Disturb",
                    "Silence all notifications until turned off",
                    self.system.settings_toggles[6],
                    SettingsAction::State(6),
                );
                let apps = start_section(&mut sections, "Application notifications");
                push(
                    &mut sections,
                    apps,
                    "Data Cloud Compare",
                    "Alerts on".into(),
                );
                push(&mut sections, apps, "Tide Radio", "Playback changes".into());
                push(&mut sections, apps, "Calendar", "Event invites".into());
            }
            11 => {
                let security = start_section(&mut sections, "Privacy & Security");
                push(
                    &mut sections,
                    security,
                    "Agent isolation",
                    "Landlock · seccomp".into(),
                );
                push(&mut sections, security, "Telemetry", "Off".into());
                push(
                    &mut sections,
                    security,
                    "Audit log",
                    "~/.local/share/aether".into(),
                );
            }
            12 => {
                let storage = start_section(&mut sections, "Storage");
                push(&mut sections, storage, "Root filesystem", "XFS".into());
                push(
                    &mut sections,
                    storage,
                    "Index",
                    "aether-index · inotify".into(),
                );
                push(&mut sections, storage, "Free space", "—".into());
            }
            _ => {}
        }
        sections
    }

    pub(crate) fn toggle_settings_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::Theme(field) => {
                if self.shell.toggle_theme_field(field) {
                    self.apply_theme_action(Some(aether_shell::theme::ThemeAction::Changed));
                }
            }
            SettingsAction::State(index) => {
                if index < self.system.settings_toggles.len() {
                    self.system.settings_toggles[index] = !self.system.settings_toggles[index];
                }
            }
            SettingsAction::Dock => {
                self.dock_enabled = !self.dock_enabled;
                if !self.dock_enabled {
                    self.dock_hover = None;
                }
            }
        }
        self.mark_full_damage();
    }

    /// Switch under the pointer in the selected Settings tab.
    pub(crate) fn settings_switch_at(&self, point: AetherPoint) -> Option<SettingsAction> {
        let content = self.settings_pane_rect();
        let mut y = content.y + 61;
        for section in self.settings_sections() {
            y += 30;
            for row in &section.rows {
                let height = if row.description.is_some() { 48 } else { 34 };
                let offset = if row.description.is_some() { 14 } else { 7 };
                let switch =
                    AetherRect::new(content.x + content.width as i32 - 50, y + offset, 34, 20);
                if row.switch.is_some() && switch.contains(point) {
                    return row.switch.map(|(_, action)| action);
                }
                y += height;
            }
            y += 12;
        }
        None
    }

    pub(crate) fn settings_content_rect(&self) -> AetherRect {
        let body = self.system_body_rect();
        AetherRect::new(
            body.x + 238,
            body.y + 16,
            body.width.saturating_sub(238),
            body.height.saturating_sub(32),
        )
    }
}
