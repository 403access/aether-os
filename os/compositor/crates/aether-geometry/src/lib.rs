//! Geometry primitives for the Aether compositor.
//!
//! Coordinates are logical pixels, origin at the top-left of the output.
//! Rectangles are half-open: `x .. x + width`, `y .. y + height`.

#![forbid(unsafe_code)]

use std::ops::{Add, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn translate(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
        }
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Point {
        Point {
            x: self.x.saturating_add(rhs.x),
            y: self.y.saturating_add(rhs.y),
        }
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, rhs: Point) -> Point {
        Point {
            x: self.x.saturating_sub(rhs.x),
            y: self.y.saturating_sub(rhs.y),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn from_size(size: Size) -> Self {
        Self::new(0, 0, size.width, size.height)
    }

    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Exclusive right edge. Computed in i64 so large rects cannot overflow.
    pub const fn right(self) -> i64 {
        self.x as i64 + self.width as i64
    }

    pub const fn bottom(self) -> i64 {
        self.y as i64 + self.height as i64
    }

    pub fn contains(self, point: Point) -> bool {
        let px = point.x as i64;
        let py = point.y as i64;
        px >= self.x as i64 && px < self.right() && py >= self.y as i64 && py < self.bottom()
    }

    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        if x2 <= x1 as i64 || y2 <= y1 as i64 {
            return None;
        }
        Some(Rect::new(
            x1,
            y1,
            (x2 - x1 as i64) as u32,
            (y2 - y1 as i64) as u32,
        ))
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.intersect(other).is_some()
    }

    /// Bounding box of two rectangles.
    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());
        Rect::new(x1, y1, (x2 - x1 as i64) as u32, (y2 - y1 as i64) as u32)
    }

    /// Shrink by `amount` on every side. Negative values expand.
    pub fn inset(self, amount: i32) -> Rect {
        let x = self.x.saturating_add(amount);
        let y = self.y.saturating_add(amount);
        let width = (self.width as i64 - 2 * amount as i64).max(0) as u32;
        let height = (self.height as i64 - 2 * amount as i64).max(0) as u32;
        Rect::new(x, y, width, height)
    }

    pub fn translate(self, dx: i32, dy: i32) -> Rect {
        Rect::new(
            self.x.saturating_add(dx),
            self.y.saturating_add(dy),
            self.width,
            self.height,
        )
    }

    /// Clip to `bounds`; returns None when fully outside.
    pub fn clamp_to(self, bounds: Rect) -> Option<Rect> {
        self.intersect(bounds)
    }

    pub fn shares_vertical_edge(self, other: Rect) -> bool {
        self.x == other.x
            && self.width == other.width
            && (self.bottom() == other.y as i64 || other.bottom() == self.y as i64)
    }

    pub fn shares_horizontal_edge(self, other: Rect) -> bool {
        self.y == other.y
            && self.height == other.height
            && (self.right() == other.x as i64 || other.right() == self.x as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersects_and_contains() {
        let a = Rect::new(0, 0, 100, 50);
        let b = Rect::new(50, 25, 100, 50);
        let overlap = a.intersect(b).expect("overlap");
        assert_eq!(overlap, Rect::new(50, 25, 50, 25));
        assert!(a.contains(Point::new(0, 0)));
        assert!(!a.contains(Point::new(100, 0)), "right edge is exclusive");
        assert!(!a.contains(Point::new(0, 50)));
        assert!(a.intersects(b));
    }

    #[test]
    fn union_and_inset() {
        let a = Rect::new(10, 10, 20, 20);
        let b = Rect::new(100, 40, 30, 10);
        assert_eq!(a.union(b), Rect::new(10, 10, 120, 40));
        assert_eq!(
            Rect::new(0, 0, 100, 100).inset(10),
            Rect::new(10, 10, 80, 80)
        );
        assert!(Rect::new(0, 0, 10, 10).inset(20).is_empty());
        assert_eq!(Rect::new(0, 0, 10, 10).inset(-5), Rect::new(-5, -5, 20, 20));
    }

    #[test]
    fn adjacent_edges_are_detected() {
        let top = Rect::new(0, 0, 100, 50);
        let bottom = Rect::new(0, 50, 100, 20);
        let right = Rect::new(100, 0, 10, 50);
        assert!(top.shares_vertical_edge(bottom));
        assert!(!top.shares_vertical_edge(right));
        assert!(top.shares_horizontal_edge(right));
    }
}
