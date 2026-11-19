//! Menu-bar tray popovers: Wi-Fi, Bluetooth, battery, sound and a calendar.
//!
//! Sizes mirror the demo's `.os-tray` panels (301 × 329, 301 × 253, 287 × 271,
//! 293 × 256 and 308 × 351). The popovers are anchored to their menu-bar
//! button and closed by clicking the button again or anywhere outside.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};

use super::overlays::menubar::{TrayAction, MENUBAR_HEIGHT};
use super::system::SystemApp;
use super::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayId {
    Network,
    Devices,
    Battery,
    Sound,
    Calendar,
}

impl TrayId {
    pub fn action(self) -> TrayAction {
        match self {
            Self::Network => TrayAction::Network,
            Self::Devices => TrayAction::Devices,
            Self::Battery => TrayAction::Battery,
            Self::Sound => TrayAction::Sound,
            Self::Calendar => TrayAction::Clock,
        }
    }

    pub fn size(self) -> (u32, u32) {
        match self {
            Self::Network => (300, 328),
            Self::Devices => (300, 253),
            Self::Battery => (286, 270),
            Self::Sound => (292, 256),
            Self::Calendar => (308, 351),
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Network => "Wi-Fi",
            Self::Devices => "Bluetooth",
            Self::Battery => "Battery",
            Self::Sound => "Sound",
            Self::Calendar => "Calendar",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Network => "wifi",
            Self::Devices => "bluetooth",
            Self::Battery => "battery",
            Self::Sound => "audio",
            Self::Calendar => "calendar",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayActionKind {
    Close,
    ToggleWifi,
    ToggleBluetooth,
    Energy(usize),
    OpenApp,
    VolumeUp,
    VolumeDown,
    Nothing,
}

impl App {
    pub(crate) fn tray_open(&self) -> Option<TrayId> {
        self.system.tray
    }

    pub(crate) fn close_tray(&mut self) {
        if self.system.tray.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn toggle_tray(&mut self, tray: TrayId) {
        let opening = self.system.tray != Some(tray);
        self.system.tray = opening.then_some(tray);
        if opening {
            self.system.refresh();
        }
        self.mark_full_damage();
    }

    /// Popover rect for a tray, anchored under its menu-bar button.
    pub(crate) fn tray_rect(&self, tray: TrayId) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let button = self
            .menubar(self.shell.primary_output().map(|o| o.id).unwrap_or(1))
            .tray
            .iter()
            .find(|(action, _)| *action == tray.action())
            .map(|(_, rect)| *rect)
            .unwrap_or(AetherRect::new(
                bounds.x + 16,
                MENUBAR_HEIGHT as i32,
                40,
                24,
            ));
        let (width, height) = tray.size();
        let x = (button.right() as i32 - width as i32)
            .max(bounds.x + 8)
            .min(bounds.x + bounds.width as i32 - width as i32 - 8);
        AetherRect::new(x, bounds.y + MENUBAR_HEIGHT as i32 + 4, width, height)
    }

    pub(crate) fn tray_action_at(&self, point: AetherPoint) -> Option<TrayActionKind> {
        let tray = self.system.tray?;
        let rect = self.tray_rect(tray);
        if !rect.contains(point) {
            return Some(TrayActionKind::Close);
        }
        let footer = AetherRect::new(rect.x + 8, rect.bottom() as i32 - 38, rect.width - 16, 30);
        if footer.contains(point) {
            return Some(TrayActionKind::OpenApp);
        }
        match tray {
            TrayId::Network => {
                if self.tray_switch_rect().contains(point) {
                    Some(TrayActionKind::ToggleWifi)
                } else {
                    Some(TrayActionKind::Nothing)
                }
            }
            TrayId::Devices => {
                if self.tray_switch_rect().contains(point) {
                    Some(TrayActionKind::ToggleBluetooth)
                } else {
                    Some(TrayActionKind::Nothing)
                }
            }
            TrayId::Battery => (0..3)
                .find(|index| self.tray_energy_rect(*index).contains(point))
                .map(TrayActionKind::Energy)
                .or(Some(TrayActionKind::Nothing)),
            TrayId::Sound => {
                if self.tray_volume_rect().contains(point) {
                    let half = self.tray_volume_rect();
                    if point.x > half.x + half.width as i32 / 2 {
                        Some(TrayActionKind::VolumeUp)
                    } else {
                        Some(TrayActionKind::VolumeDown)
                    }
                } else {
                    Some(TrayActionKind::Nothing)
                }
            }
            TrayId::Calendar => Some(TrayActionKind::Nothing),
        }
    }

    pub(crate) fn activate_tray_action(&mut self, action: TrayActionKind) {
        let Some(tray) = self.system.tray else {
            return;
        };
        match action {
            TrayActionKind::Close => {
                self.close_tray();
                return;
            }
            TrayActionKind::ToggleWifi => {
                self.system.wifi = !self.system.wifi;
            }
            TrayActionKind::ToggleBluetooth => {
                self.system.bluetooth = !self.system.bluetooth;
            }
            TrayActionKind::Energy(index) => {
                self.system.energy = index;
            }
            TrayActionKind::VolumeUp | TrayActionKind::VolumeDown => {
                println!("aether-shell: tray volume");
            }
            TrayActionKind::OpenApp => {
                self.close_tray();
                match tray {
                    TrayId::Network => self.open_system(SystemApp::Network),
                    TrayId::Devices => self.open_system(SystemApp::Devices),
                    TrayId::Battery => self.open_system(SystemApp::Battery),
                    TrayId::Sound => self.toggle_audio(),
                    TrayId::Calendar => self.open_calendar(),
                }
                return;
            }
            TrayActionKind::Nothing => {}
        }
        self.mark_full_damage();
    }

    pub(crate) fn tray_switch_rect(&self) -> AetherRect {
        let rect = self
            .system
            .tray
            .map(|tray| self.tray_rect(tray))
            .unwrap_or_default();
        AetherRect::new(rect.x + rect.width as i32 - 50, rect.y + 52, 34, 20)
    }

    pub(crate) fn tray_energy_rect(&self, index: usize) -> AetherRect {
        let rect = self
            .system
            .tray
            .map(|tray| self.tray_rect(tray))
            .unwrap_or_default();
        let width = (rect.width as i32 - 24) / 3;
        AetherRect::new(
            rect.x + 12 + index as i32 * width,
            rect.y + 150,
            (width - 4) as u32,
            26,
        )
    }

    pub(crate) fn tray_volume_rect(&self) -> AetherRect {
        let rect = self
            .system
            .tray
            .map(|tray| self.tray_rect(tray))
            .unwrap_or_default();
        AetherRect::new(rect.x + 12, rect.y + 176, rect.width - 24, 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_sizes_match_the_demo() {
        assert_eq!(TrayId::Network.size(), (300, 328));
        assert_eq!(TrayId::Devices.size(), (300, 253));
        assert_eq!(TrayId::Battery.size(), (286, 270));
        assert_eq!(TrayId::Sound.size(), (292, 256));
        assert_eq!(TrayId::Calendar.size(), (308, 351));
    }
}
