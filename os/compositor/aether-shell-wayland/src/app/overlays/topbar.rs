//! Top bar layout, hit testing and shortcut activation.

use aether_geometry::Point as AetherPoint;
use aether_shell::OutputId;

mod layout;

pub(crate) use layout::{hit, layout, TopAction, TopBar};

use super::super::App;

impl App {
    pub(crate) fn top_bar(&self, output: OutputId) -> TopBar {
        layout(self.shell.output_bounds(output))
    }

    pub(crate) fn top_bar_visible(&self, output: OutputId) -> bool {
        self.shell.fullscreen_window_on(output).is_none()
            && !self.shell.explorer_open()
            && !self.shell.chat_open()
            && !self.shell.audio_open()
            && !self.shell.themes_open()
            && !self.shell.remote_open()
            && !self.shortcuts_open
            && !self.sandbox_panel_open()
            && self.shell.overlay().is_none()
    }

    pub(crate) fn top_shortcut_at(&self, point: AetherPoint) -> Option<TopAction> {
        let output = self
            .shell
            .output_at(point)
            .or_else(|| self.shell.primary_output().map(|output| output.id))?;
        if !self.top_bar_visible(output) {
            return None;
        }
        hit(&self.top_bar(output), point.x, point.y)
    }

    pub(crate) fn activate_top_shortcut(&mut self, action: TopAction) {
        match action {
            TopAction::Palette => {
                println!("aether-shell: top shortcut palette");
                self.toggle_overlay();
            }
            TopAction::Terminal => {
                self.spawn_terminal();
                println!("aether-shell: top shortcut terminal");
            }
            TopAction::Files => {
                println!("aether-shell: top shortcut files");
                self.toggle_explorer();
            }
            TopAction::Chat => {
                println!("aether-shell: top shortcut chat");
                self.toggle_chat();
            }
            TopAction::Audio => {
                println!("aether-shell: top shortcut audio");
                self.toggle_audio();
            }
            TopAction::Agents => {
                println!("aether-shell: top shortcut agents");
                self.toggle_sandboxes();
            }
            TopAction::Shortcuts => {
                println!("aether-shell: top shortcut keys");
                self.toggle_shortcuts();
            }
        }
    }
}
