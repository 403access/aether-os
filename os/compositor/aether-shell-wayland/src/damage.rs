//! Buffer-coordinate damage conversion.
//!
//! `wl_surface.damage_buffer` reports rectangles in buffer pixels. The shell
//! renders at logical scale, so divide by the surface buffer scale and round
//! outward to avoid under-damaging edges.

use aether_geometry::Rect;
use smithay::utils::{Buffer, Rectangle};

/// Convert a buffer-coordinate damage rectangle to logical coordinates.
/// Returns `None` for invalid scale or empty rectangles.
pub fn buffer_damage_to_logical(rect: Rectangle<i32, Buffer>, scale: i32) -> Option<Rect> {
    if scale <= 0 || rect.size.w <= 0 || rect.size.h <= 0 {
        return None;
    }
    let scale = scale as i64;
    let left = rect.loc.x as i64;
    let top = rect.loc.y as i64;
    let right = left + rect.size.w as i64;
    let bottom = top + rect.size.h as i64;

    let x = left.div_euclid(scale);
    let y = top.div_euclid(scale);
    let x2 = ceil_div(right, scale);
    let y2 = ceil_div(bottom, scale);
    if x2 <= x || y2 <= y {
        return None;
    }
    Some(Rect::new(
        x as i32,
        y as i32,
        (x2 - x) as u32,
        (y2 - y) as u32,
    ))
}

fn ceil_div(value: i64, divisor: i64) -> i64 {
    let quotient = value.div_euclid(divisor);
    if value.rem_euclid(divisor) == 0 {
        quotient
    } else {
        quotient + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn damage(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Buffer> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    #[test]
    fn scale_one_is_identity() {
        assert_eq!(
            buffer_damage_to_logical(damage(10, 20, 30, 40), 1),
            Some(Rect::new(10, 20, 30, 40))
        );
    }

    #[test]
    fn divides_aligned_damage_by_scale() {
        assert_eq!(
            buffer_damage_to_logical(damage(10, 6, 20, 10), 2),
            Some(Rect::new(5, 3, 10, 5))
        );
    }

    #[test]
    fn rounds_outward_for_unaligned_edges() {
        assert_eq!(
            buffer_damage_to_logical(damage(1, 1, 10, 10), 2),
            Some(Rect::new(0, 0, 6, 6))
        );
    }

    #[test]
    fn rejects_invalid_input() {
        assert_eq!(buffer_damage_to_logical(damage(0, 0, 0, 0), 1), None);
        assert_eq!(buffer_damage_to_logical(damage(0, 0, 10, 10), 0), None);
    }
}
