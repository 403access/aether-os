//! Frame-plan production from the scene and pacer.

use crate::frame::{FrameItem, FramePlan};

use super::Shell;

impl Shell {
    pub fn next_deadline(&self) -> Option<u64> {
        self.pacer.deadline()
    }

    /// Produce a frame if the pacer is due and something changed. The caller
    /// hands the plan to a backend (Smithay, software preview, remote encoder)
    /// and clears it by presenting.
    pub fn frame(&mut self, now_ns: u64) -> Option<FramePlan> {
        if !self.pacer.should_present(now_ns) {
            return None;
        }

        let damage = self.scene.commit().clone();
        if damage.is_empty() {
            return None;
        }
        // The frame is the presentation point: the damage is now owned by the
        // plan, so the scene starts the next frame clean.
        self.scene.clear_damage();
        self.pacer.on_present(now_ns);

        let mut items = Vec::new();
        for surface_id in self.scene.draw_order() {
            let Some(surface) = self.scene.get(surface_id) else {
                continue;
            };
            let window = self
                .windows
                .iter()
                .find_map(|(id, window)| (window.surface == surface_id).then_some(*id));
            let (title, focused) = match window {
                Some(id) => (
                    self.windows
                        .get(&id)
                        .map(|window| window.spec.title.clone())
                        .unwrap_or_default(),
                    self.focus == Some(id),
                ),
                None if Some(surface_id) == self.overlay => ("Command Palette".to_string(), false),
                None => ("surface".to_string(), false),
            };
            items.push(FrameItem {
                surface: surface_id,
                window,
                rect: surface.rect,
                layer: surface.layer,
                title,
                focused,
            });
        }

        Some(FramePlan {
            viewport: self.viewport,
            damage,
            items,
            focus: self.focus,
            workspace: self.active,
            presented_ns: now_ns,
        })
    }
}
