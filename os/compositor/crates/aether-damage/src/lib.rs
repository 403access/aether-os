//! Damage regions.
//!
//! A region is a set of non-overlapping rectangles, merged aggressively. The
//! compositor only needs "what changed"; over-damage is safe, under-damage is a
//! visual bug. When merging would produce too many rectangles, the region
//! collapses to its bounding box.

#![forbid(unsafe_code)]

use aether_geometry::Rect;

/// Above this many rectangles, a full bounding-box redraw is cheaper.
pub const MAX_RECTS: usize = 32;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Region {
    rects: Vec<Rect>,
}

impl Region {
    pub fn new() -> Self {
        Self { rects: Vec::new() }
    }

    pub fn from_rect(rect: Rect) -> Self {
        let mut region = Self::new();
        region.add(rect);
        region
    }

    /// Add a rectangle, merging with every intersecting or edge-adjacent
    /// rectangle. Empty rectangles are ignored.
    pub fn add(&mut self, rect: Rect) {
        if rect.is_empty() {
            return;
        }

        let mut merged = rect;
        let mut index = 0;
        while index < self.rects.len() {
            let other = self.rects[index];
            if other.intersects(merged)
                || other.shares_vertical_edge(merged)
                || other.shares_horizontal_edge(merged)
            {
                merged = merged.union(other);
                self.rects.remove(index);
                index = 0;
                continue;
            }
            index += 1;
        }

        self.rects.push(merged);

        if self.rects.len() > MAX_RECTS {
            let bounds = self.bounds().unwrap_or(merged);
            self.rects.clear();
            self.rects.push(bounds);
        }
    }

    pub fn union_with(&mut self, other: &Region) {
        for rect in &other.rects {
            self.add(*rect);
        }
    }

    pub fn clipped(&self, clip: Rect) -> Region {
        let mut result = Region::new();
        for rect in &self.rects {
            if let Some(clipped) = rect.clamp_to(clip) {
                result.add(clipped);
            }
        }
        result
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut iter = self.rects.iter().copied();
        let first = iter.next()?;
        Some(iter.fold(first, |acc, rect| acc.union(rect)))
    }

    /// Sum of the rectangle areas. Rectangles are non-overlapping by
    /// construction, so this is the true damaged area.
    pub fn area(&self) -> u64 {
        self.rects.iter().map(|rect| rect.area()).sum()
    }

    /// True when redrawing the whole viewport is cheaper than the region.
    pub fn is_mostly_full(&self, viewport: Rect, threshold: f64) -> bool {
        let total = viewport.area();
        if total == 0 {
            return false;
        }
        self.area() as f64 / total as f64 >= threshold
    }

    pub fn clear(&mut self) {
        self.rects.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rects.len()
    }

    pub fn rects(&self) -> &[Rect] {
        &self.rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_region_ignores_empty_rects() {
        let mut region = Region::new();
        region.add(Rect::new(0, 0, 0, 10));
        assert!(region.is_empty());
        assert_eq!(region.area(), 0);
    }

    #[test]
    fn disjoint_rects_are_kept_separate() {
        let mut region = Region::new();
        region.add(Rect::new(0, 0, 10, 10));
        region.add(Rect::new(100, 100, 10, 10));
        assert_eq!(region.len(), 2);
        assert_eq!(region.area(), 200);
    }

    #[test]
    fn intersecting_rects_merge_to_union() {
        let mut region = Region::new();
        region.add(Rect::new(0, 0, 20, 20));
        region.add(Rect::new(10, 10, 20, 20));
        assert_eq!(region.len(), 1);
        assert_eq!(region.rects()[0], Rect::new(0, 0, 30, 30));
    }

    #[test]
    fn adjacent_rects_merge_transitively() {
        let mut region = Region::new();
        for index in 0..10 {
            region.add(Rect::new(0, index * 10, 100, 10));
        }
        assert_eq!(region.len(), 1);
        assert_eq!(region.rects()[0], Rect::new(0, 0, 100, 100));
    }

    #[test]
    fn too_many_rects_collapse_to_bounds() {
        let mut region = Region::new();
        for row in 0..8 {
            for column in 0..8 {
                region.add(Rect::new(column * 40, row * 40, 10, 10));
            }
        }
        // The collapse keeps the region bounded; it may then accumulate a few
        // more rectangles before hitting the cap again.
        assert!(region.len() <= MAX_RECTS);
        assert_eq!(region.bounds(), Some(Rect::new(0, 0, 290, 290)));
        assert!(region.area() > 64 * 100);
    }

    #[test]
    fn clipping_removes_offscreen_parts() {
        let region = Region::from_rect(Rect::new(-10, -10, 40, 40));
        let clipped = region.clipped(Rect::new(0, 0, 100, 100));
        assert_eq!(clipped.rects(), &[Rect::new(0, 0, 30, 30)]);
        assert!(region.clipped(Rect::new(50, 50, 10, 10)).is_empty());
    }

    #[test]
    fn mostly_full_threshold() {
        let viewport = Rect::new(0, 0, 100, 100);
        let mut region = Region::new();
        region.add(Rect::new(0, 0, 50, 100));
        assert!(region.is_mostly_full(viewport, 0.5));
        assert!(!region.is_mostly_full(viewport, 0.6));
        assert!(!Region::new().is_mostly_full(Rect::new(0, 0, 0, 0), 0.0));
    }
}
