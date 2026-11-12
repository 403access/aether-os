//! Scene graph: surfaces, z-order, damage and hit testing.
//!
//! This is the compositor's model of what is on screen. It is deliberately
//! flat: overlays are surfaces with a higher layer, not a nested widget tree.
//! It has no GPU, Wayland or input dependency, so apps and tests can build a
//! scene without a display.
//!
//! Compose it with `aether-layout` (rectangles), `aether-pacing` (deadlines)
//! and `aether-damage` (what changed).

#![forbid(unsafe_code)]

use aether_damage::Region;
use aether_geometry::{Point, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Layer(pub u8);

impl Layer {
    pub const BACKGROUND: Layer = Layer(0);
    pub const WINDOW: Layer = Layer(10);
    pub const FLOATING: Layer = Layer(50);
    pub const OVERLAY: Layer = Layer(80);
    pub const CHROME: Layer = Layer(100);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId(pub u64);

#[derive(Clone, Debug)]
pub struct Surface {
    pub id: SurfaceId,
    pub rect: Rect,
    pub layer: Layer,
    /// Opaque surfaces let the renderer skip what is behind them.
    pub opaque: bool,
    pub visible: bool,
    z: u64,
    damage: Region,
}

impl Surface {
    pub fn damage(&self) -> &Region {
        &self.damage
    }

    /// Content damage, clipped to the surface. Does not affect the scene until
    /// `Scene::commit`.
    pub fn add_damage(&mut self, rect: Rect) {
        if let Some(clipped) = rect.clamp_to(self.rect) {
            self.damage.add(clipped);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    viewport: Size,
    surfaces: Vec<Surface>,
    damage: Region,
    next_id: u64,
    next_z: u64,
}

impl Scene {
    pub fn new(viewport: Size) -> Self {
        Self {
            viewport,
            surfaces: Vec::new(),
            damage: Region::new(),
            next_id: 1,
            next_z: 1,
        }
    }

    pub fn viewport(&self) -> Size {
        self.viewport
    }

    pub fn add(&mut self, layer: Layer, rect: Rect) -> SurfaceId {
        let id = SurfaceId(self.next_id);
        self.next_id += 1;
        let z = self.next_z;
        self.next_z += 1;

        self.surfaces.push(Surface {
            id,
            rect,
            layer,
            opaque: true,
            visible: true,
            z,
            damage: Region::new(),
        });
        self.damage.add(rect);
        id
    }

    pub fn remove(&mut self, id: SurfaceId) -> bool {
        if let Some(index) = self.surfaces.iter().position(|surface| surface.id == id) {
            let surface = self.surfaces.remove(index);
            if surface.visible {
                self.damage.add(surface.rect);
            }
            return true;
        }
        false
    }

    pub fn get(&self, id: SurfaceId) -> Option<&Surface> {
        self.surfaces.iter().find(|surface| surface.id == id)
    }

    pub fn get_mut(&mut self, id: SurfaceId) -> Option<&mut Surface> {
        self.surfaces.iter_mut().find(|surface| surface.id == id)
    }

    /// Move/resize a surface, damaging both the old and the new rectangle.
    pub fn move_to(&mut self, id: SurfaceId, rect: Rect) -> bool {
        let Some(surface) = self.get_mut(id) else {
            return false;
        };
        if surface.rect == rect {
            return true;
        }
        let old = surface.rect;
        surface.rect = rect;
        surface.damage.clear();
        if surface.visible {
            self.damage.add(old);
            self.damage.add(rect);
        }
        true
    }

    pub fn set_visible(&mut self, id: SurfaceId, visible: bool) -> bool {
        let Some(surface) = self.get_mut(id) else {
            return false;
        };
        if surface.visible == visible {
            return true;
        }
        surface.visible = visible;
        let rect = surface.rect;
        self.damage.add(rect);
        true
    }

    /// Raise within the surface's layer without changing the layer.
    pub fn raise(&mut self, id: SurfaceId) -> bool {
        let z = self.next_z;
        let Some(surface) = self.get_mut(id) else {
            return false;
        };
        surface.z = z;
        let rect = surface.rect;
        self.next_z += 1;
        self.damage.add(rect);
        true
    }

    /// Draw order: layer first, then insertion/raise order. Stable.
    pub fn draw_order(&self) -> Vec<SurfaceId> {
        let mut visible: Vec<&Surface> = self.surfaces.iter().filter(|s| s.visible).collect();
        visible.sort_by_key(|surface| (surface.layer, surface.z));
        visible.into_iter().map(|surface| surface.id).collect()
    }

    /// Topmost visible surface containing the point.
    pub fn hit_test(&self, point: Point) -> Option<SurfaceId> {
        let mut visible: Vec<&Surface> = self.surfaces.iter().filter(|s| s.visible).collect();
        visible.sort_by_key(|surface| (surface.layer, surface.z));
        visible
            .into_iter()
            .rev()
            .find(|surface| surface.rect.contains(point))
            .map(|surface| surface.id)
    }

    pub fn add_damage(&mut self, id: SurfaceId, rect: Rect) -> bool {
        let Some(surface) = self.get_mut(id) else {
            return false;
        };
        surface.add_damage(rect);
        true
    }

    /// Merge surface damage into the scene and clear it.
    pub fn commit(&mut self) -> &Region {
        for surface in &mut self.surfaces {
            if surface.visible && !surface.damage.is_empty() {
                self.damage.union_with(&surface.damage);
                surface.damage.clear();
            }
        }
        &self.damage
    }

    pub fn damage(&self) -> &Region {
        &self.damage
    }

    pub fn clear_damage(&mut self) {
        self.damage.clear();
    }

    pub fn resize_viewport(&mut self, size: Size) {
        let old = self.viewport;
        self.viewport = size;
        self.damage.add(Rect::from_size(old));
        self.damage.add(Rect::from_size(size));
    }

    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    pub fn len(&self) -> usize {
        self.surfaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> Scene {
        Scene::new(Size::new(1920, 1080))
    }

    #[test]
    fn draw_order_respects_layers_and_insertion() {
        let mut scene = scene();
        let window = scene.add(Layer::WINDOW, Rect::new(0, 0, 800, 600));
        let chrome = scene.add(Layer::CHROME, Rect::new(0, 1040, 1920, 40));
        let overlay = scene.add(Layer::OVERLAY, Rect::new(200, 100, 600, 400));
        let second = scene.add(Layer::WINDOW, Rect::new(100, 100, 400, 300));

        assert_eq!(
            scene.draw_order(),
            vec![window, second, overlay, chrome],
            "windows in insertion order, then overlay, then chrome"
        );
    }

    #[test]
    fn raise_changes_order_within_layer() {
        let mut scene = scene();
        let first = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        let second = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        scene.raise(first);
        assert_eq!(scene.draw_order(), vec![second, first]);
    }

    #[test]
    fn hit_test_returns_topmost_surface() {
        let mut scene = scene();
        let back = scene.add(Layer::WINDOW, Rect::new(0, 0, 800, 600));
        let front = scene.add(Layer::OVERLAY, Rect::new(100, 100, 400, 300));

        assert_eq!(scene.hit_test(Point::new(50, 50)), Some(back));
        assert_eq!(scene.hit_test(Point::new(150, 150)), Some(front));
        assert_eq!(scene.hit_test(Point::new(1700, 900)), None);
    }

    #[test]
    fn hidden_surfaces_are_not_drawn_or_hit() {
        let mut scene = scene();
        let surface = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        assert!(scene.set_visible(surface, false));
        assert!(scene.draw_order().is_empty());
        assert_eq!(scene.hit_test(Point::new(10, 10)), None);
        assert!(scene.set_visible(surface, true));
        assert_eq!(scene.hit_test(Point::new(10, 10)), Some(surface));
    }

    #[test]
    fn moving_damages_old_and_new_rects() {
        let mut scene = scene();
        let surface = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        scene.clear_damage();

        assert!(scene.move_to(surface, Rect::new(500, 500, 100, 100)));
        let damage = scene.damage();
        assert_eq!(damage.len(), 2);
        assert_eq!(damage.area(), 20_000);
        assert!(damage.rects().contains(&Rect::new(0, 0, 100, 100)));
        assert!(damage.rects().contains(&Rect::new(500, 500, 100, 100)));
    }

    #[test]
    fn moving_away_and_back_damages_only_original() {
        let mut scene = scene();
        let surface = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        scene.move_to(surface, Rect::new(200, 0, 100, 100));
        scene.move_to(surface, Rect::new(0, 0, 100, 100));
        assert_eq!(scene.damage().area(), 20_000);
    }

    #[test]
    fn commit_merges_content_damage_once() {
        let mut scene = scene();
        let surface = scene.add(Layer::WINDOW, Rect::new(0, 0, 100, 100));
        scene.clear_damage();

        assert!(scene.add_damage(surface, Rect::new(-50, -50, 100, 100)));
        assert_eq!(scene.damage().area(), 0, "not visible until commit");
        let area = scene.commit().area();
        assert_eq!(area, 50 * 50, "clipped to the surface");
        assert!(scene.get(surface).unwrap().damage().is_empty());
        // Scene damage is cleared by the presenter, not by commit.
        assert_eq!(scene.commit().area(), 50 * 50);
        scene.clear_damage();
        assert_eq!(scene.damage().area(), 0);
    }

    #[test]
    fn remove_damages_visible_surface() {
        let mut scene = scene();
        let surface = scene.add(Layer::WINDOW, Rect::new(10, 10, 100, 100));
        scene.clear_damage();
        assert!(scene.remove(surface));
        assert_eq!(scene.damage().area(), 10_000);
        assert!(!scene.remove(surface));
        assert!(scene.is_empty());
    }

    #[test]
    fn resize_damages_both_viewports() {
        let mut scene = scene();
        scene.clear_damage();
        scene.resize_viewport(Size::new(2560, 1440));
        // The new viewport contains the old one, so the union is the new area.
        assert_eq!(scene.damage().area(), 2560 * 1440);
    }

    #[test]
    fn unknown_ids_are_no_ops() {
        let mut scene = scene();
        let ghost = SurfaceId(999);
        assert!(!scene.move_to(ghost, Rect::new(0, 0, 10, 10)));
        assert!(!scene.set_visible(ghost, false));
        assert!(!scene.raise(ghost));
        assert!(!scene.add_damage(ghost, Rect::new(0, 0, 10, 10)));
    }
}
