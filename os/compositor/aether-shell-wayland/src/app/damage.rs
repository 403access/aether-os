//! Damage accumulation and per-output clipping.
//!
//! The compositor records logical damage rectangles while handling input and
//! protocol commits, then each presenter asks for the subset that intersects
//! its output. `full_damage` short-circuits the rectangle list and requests a
//! complete clear of the affected output.

use aether_geometry::Rect as AetherRect;
use aether_shell::OutputId;
use smithay::utils::{Logical, Physical, Point as SPoint, Rectangle, Size as SSize};

use super::App;

impl App {
    pub(crate) fn mark_window_damage(&mut self, rect: AetherRect) {
        self.mark_damage_rect(rect);
    }

    pub(crate) fn mark_damage_rect(&mut self, rect: AetherRect) {
        if self.full_damage || rect.is_empty() {
            return;
        }
        self.damage.push(Rectangle::new(
            SPoint::from((rect.x, rect.y)),
            SSize::from((rect.width as i32, rect.height as i32)),
        ));
    }

    /// Damage around a cursor position (old and new positions are both
    /// marked by callers), independent of window content.
    pub fn mark_cursor_damage(&mut self, position: SPoint<f64, Logical>) {
        const RADIUS: i32 = 32;
        let x = position.x.round() as i32;
        let y = position.y.round() as i32;
        self.mark_damage_rect(AetherRect::new(
            x - RADIUS,
            y - RADIUS,
            (RADIUS * 2) as u32,
            (RADIUS * 2) as u32,
        ));
    }

    /// Whether compositor content changed since the last presented frame.
    pub fn needs_frame(&self) -> bool {
        self.full_damage || !self.damage.is_empty()
    }

    /// Damage for one output in local physical coordinates. `None` means the
    /// output is unaffected by the pending damage and must not be composed.
    /// A `true` first element requests a full-output clear.
    pub fn output_damage(
        &self,
        output: Option<OutputId>,
    ) -> Option<(bool, Vec<Rectangle<i32, Physical>>)> {
        if !self.needs_frame() {
            return None;
        }
        let bounds = match output {
            Some(id) => self.shell.output_bounds(id),
            None => AetherRect::from_size(self.shell.viewport()),
        };
        damage_for_bounds(self.full_damage, &self.damage, bounds)
    }

    /// Mark an output change for deferred repaint (new output, mode change).
    pub fn mark_output_damage(&mut self) {
        self.mark_full_damage();
    }

    pub fn mark_full_damage(&mut self) {
        if std::env::var_os("AETHER_DAMAGE_TRACE").is_some() {
            eprintln!(
                "FULL_DAMAGE at\n{}",
                std::backtrace::Backtrace::force_capture()
            );
        }
        self.full_damage = true;
        self.damage.clear();
    }

    /// Drain accumulated damage for presenters that need explicit regions.
    pub fn take_damage(&mut self) -> (bool, Vec<Rectangle<i32, Physical>>) {
        let full = self.full_damage;
        let damage = if full {
            Vec::new()
        } else {
            std::mem::take(&mut self.damage)
        };
        self.full_damage = false;
        (full, damage)
    }
}

/// Clip global damage to one output and translate it into local coordinates.
/// Pure so the presenter's damage contract can be unit-tested without EGL.
pub(crate) fn damage_for_bounds(
    full: bool,
    damage: &[Rectangle<i32, Physical>],
    bounds: AetherRect,
) -> Option<(bool, Vec<Rectangle<i32, Physical>>)> {
    let local_size = SSize::from((bounds.width.max(1) as i32, bounds.height.max(1) as i32));
    if full {
        return Some((true, vec![Rectangle::from_size(local_size)]));
    }
    let clip = Rectangle::new(
        SPoint::from((bounds.x, bounds.y)),
        SSize::from((bounds.width.max(1) as i32, bounds.height.max(1) as i32)),
    );
    let mut local = Vec::new();
    for rect in damage {
        if let Some(hit) = rect.intersection(clip) {
            local.push(Rectangle::new(
                SPoint::from((hit.loc.x - bounds.x, hit.loc.y - bounds.y)),
                hit.size,
            ));
        }
    }
    if local.is_empty() {
        None
    } else {
        Some((false, local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rectangle<i32, Physical> {
        Rectangle::new(SPoint::from((x, y)), SSize::from((width, height)))
    }

    #[test]
    fn damage_is_clipped_and_translated_per_output() {
        let damage = vec![rect(90, 10, 40, 20), rect(400, 10, 10, 10)];
        let bounds = AetherRect::new(100, 0, 100, 100);
        let (full, local) = damage_for_bounds(false, &damage, bounds).unwrap();
        assert!(!full);
        assert_eq!(local, vec![rect(0, 10, 30, 20)]);
    }

    #[test]
    fn unaffected_output_reports_no_damage() {
        let damage = vec![rect(0, 0, 20, 20)];
        let bounds = AetherRect::new(100, 0, 100, 100);
        assert!(damage_for_bounds(false, &damage, bounds).is_none());
    }

    #[test]
    fn full_damage_is_local_to_the_output() {
        let bounds = AetherRect::new(100, 50, 100, 80);
        let (full, local) = damage_for_bounds(true, &[], bounds).unwrap();
        assert!(full);
        assert_eq!(local, vec![rect(0, 0, 100, 80)]);
    }
}
