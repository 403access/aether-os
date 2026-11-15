//! Keyboard shortcuts cheat-sheet overlay.

use super::super::App;
use aether_geometry::Rect as AetherRect;

impl App {
    pub(crate) fn toggle_shortcuts(&mut self) {
        if self.shortcuts_open {
            self.close_shortcuts();
        } else {
            self.shell.close_overlay();
            self.shell.close_explorer();
            self.shell.close_chat();
            self.shell.close_audio();
            self.shell.close_themes();
            self.shell.close_remote();
            self.close_sandboxes();
            self.shortcuts_open = true;
            println!(
                "aether-shell: shortcuts overlay opened ({} bindings)",
                self.key_profile.entries().len()
            );
            self.mark_full_damage();
        }
    }

    pub(crate) fn close_shortcuts(&mut self) {
        if self.shortcuts_open {
            self.shortcuts_open = false;
            self.mark_full_damage();
        }
    }

    pub(crate) fn shortcuts_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = 760.min(bounds.width.saturating_sub(32)).max(1);
        let height = 560.min(bounds.height.saturating_sub(32)).max(1);
        AetherRect::new(
            bounds.x + (bounds.width.saturating_sub(width) / 2) as i32,
            bounds.y + (bounds.height.saturating_sub(height) / 2) as i32,
            width,
            height,
        )
    }

    pub(crate) fn mark_shortcuts_damage(&mut self) {
        if self.shortcuts_open {
            let rect = self.shortcuts_rect();
            self.mark_damage_rect(rect);
        }
    }
}
