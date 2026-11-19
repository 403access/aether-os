//! Desktop dock: activation, hit-testing and damage.

mod layout;

use aether_geometry::Point as AetherPoint;
use aether_shell::OutputId;

pub(crate) use layout::{hit, layout, Dock, DockAction};

use super::super::App;

impl App {
    pub(crate) fn dock(&self, output: OutputId) -> Dock {
        layout(self.shell.output_bounds(output))
    }

    pub(crate) fn dock_visible(&self, output: OutputId) -> bool {
        // Capture switch: app-only screenshots should not bake the floating
        // dock over the bottom of a window crop.
        if std::env::var_os("AETHER_HIDE_DOCK").is_some() {
            return false;
        }
        let enabled = self.dock_enabled || std::env::var_os("AETHER_SHOW_DOCK").is_some();
        enabled && self.shell.fullscreen_window_on(output).is_none()
    }

    pub(crate) fn dock_action_at(&self, point: AetherPoint) -> Option<DockAction> {
        let output = self.shell.output_at(point)?;
        if !self.dock_visible(output) {
            return None;
        }
        hit(&self.dock(output), point.x, point.y)
    }

    pub(crate) fn activate_dock(&mut self, action: DockAction) {
        match action {
            DockAction::Explorer => self.toggle_explorer(),
            DockAction::Search => self.toggle_overlay(),
            DockAction::Settings => self.open_system(crate::app::system::SystemApp::Settings),
            DockAction::Terminal => {
                self.spawn_terminal();
                println!("aether-shell: dock terminal");
            }
            DockAction::Radio => self.open_system(crate::app::system::SystemApp::Radio),
            DockAction::About => self.open_system(crate::app::system::SystemApp::About),
            DockAction::Trash => println!("aether-shell: dock trash"),
            DockAction::Gallery => println!("aether-shell: dock gallery"),
        }
        self.mark_full_damage();
    }
}
