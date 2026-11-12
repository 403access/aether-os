//! Tiling layout.
//!
//! Master on the left, remaining windows in up to two stack columns. The
//! function is pure: it takes a viewport and a count and returns rectangles in
//! window order. Gaps are drawn by the compositor, not by clients.

#![forbid(unsafe_code)]

use aether_geometry::Rect;

#[derive(Clone, Copy, Debug)]
pub struct LayoutConfig {
    /// Fraction of the inner width given to the master window, clamped 0.2–0.8.
    pub master_ratio: f32,
    /// Gap between windows.
    pub gap: u32,
    /// Gap between the work area and the output edge.
    pub outer_gap: u32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            master_ratio: 0.5,
            gap: 6,
            outer_gap: 8,
        }
    }
}

pub fn tile(viewport: Rect, count: usize, config: LayoutConfig) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }

    let inner = viewport.inset(config.outer_gap as i32);
    if inner.is_empty() {
        return vec![Rect::default(); count];
    }
    if count == 1 {
        return vec![inner];
    }

    let gap = config.gap as i32;
    let ratio = f64::from(config.master_ratio).clamp(0.2, 0.8);
    let usable = (inner.width as i32 - gap).max(0);
    let master_width = (f64::from(usable) * ratio).round() as i32;
    let rest_x = inner.x + master_width + gap;
    let rest_width = (inner.width as i32 - master_width - gap).max(0);

    let master = Rect::new(inner.x, inner.y, master_width.max(0) as u32, inner.height);

    if count == 2 {
        return vec![
            master,
            Rect::new(rest_x, inner.y, rest_width.max(0) as u32, inner.height),
        ];
    }

    let remaining = count - 1;
    let column_width = ((rest_width - gap).max(0)) / 2;
    let second_x = rest_x;
    let third_x = rest_x + column_width + gap;
    let second_count = remaining - remaining / 2;
    let third_count = remaining / 2;

    let mut rects = Vec::with_capacity(count);
    rects.push(master);
    rects.extend(stack(second_x, inner, column_width, second_count, gap));
    rects.extend(stack(third_x, inner, column_width, third_count, gap));
    rects
}

fn stack(x: i32, inner: Rect, width: i32, count: usize, gap: i32) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }

    let total_gap = gap * (count as i32 - 1);
    let available = (inner.height as i32 - total_gap).max(0);
    let base = available / count as i32;
    let mut rects = Vec::with_capacity(count);
    let mut cursor = inner.y;
    let bottom = inner.y + inner.height as i32;

    for index in 0..count {
        let height = if index + 1 == count {
            (bottom - cursor).max(0)
        } else {
            base
        };
        rects.push(Rect::new(
            x,
            cursor,
            width.max(0) as u32,
            height.max(0) as u32,
        ));
        cursor += height + gap;
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORT: Rect = Rect::new(0, 0, 1600, 1000);

    fn assert_no_overlap(rects: &[Rect]) {
        for (index, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(index + 1) {
                assert!(a.intersect(*b).is_none(), "rects overlap: {a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn zero_and_one_window() {
        let config = LayoutConfig::default();
        assert!(tile(VIEWPORT, 0, config).is_empty());

        let single = tile(VIEWPORT, 1, config);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0], VIEWPORT.inset(8));
    }

    #[test]
    fn two_windows_split_master_and_stack() {
        let rects = tile(VIEWPORT, 2, LayoutConfig::default());
        assert_eq!(rects.len(), 2);
        assert_no_overlap(&rects);
        let inner = VIEWPORT.inset(8);
        for rect in &rects {
            assert!(inner.intersect(*rect) == Some(*rect), "outside inner area");
        }
        // Default ratio is 0.5, so the two columns are equal within rounding.
        assert!((rects[0].width as i32 - rects[1].width as i32).abs() <= 1);
        assert_eq!(rects[0].height, rects[1].height);
    }

    #[test]
    fn four_windows_use_three_columns() {
        let rects = tile(VIEWPORT, 4, LayoutConfig::default());
        assert_eq!(rects.len(), 4);
        assert_no_overlap(&rects);

        let mut xs: Vec<i32> = rects.iter().map(|rect| rect.x).collect();
        xs.sort_unstable();
        xs.dedup();
        assert_eq!(xs.len(), 3, "master plus two stack columns");
    }

    #[test]
    fn stack_rows_fill_the_column() {
        let rects = tile(VIEWPORT, 7, LayoutConfig::default());
        assert_eq!(rects.len(), 7);
        assert_no_overlap(&rects);

        let inner = VIEWPORT.inset(8);
        let third_column: Vec<&Rect> = rects
            .iter()
            .filter(|rect| rect.x > inner.x + inner.width as i32 / 2)
            .collect();
        assert!(third_column.len() >= 3);
        let top = third_column.iter().map(|r| r.y as i64).min().unwrap();
        let bottom = third_column.iter().map(|r| r.bottom()).max().unwrap();
        assert_eq!(top, inner.y as i64);
        assert_eq!(bottom, inner.bottom());
    }

    #[test]
    fn gap_is_respected() {
        let config = LayoutConfig {
            master_ratio: 0.5,
            gap: 12,
            outer_gap: 0,
        };
        let rects = tile(VIEWPORT, 2, config);
        let gap = rects[1].x - (rects[0].x + rects[0].width as i32);
        assert_eq!(gap, 12);
    }

    #[test]
    fn ratio_is_clamped() {
        let wide = tile(
            VIEWPORT,
            2,
            LayoutConfig {
                master_ratio: 5.0,
                ..LayoutConfig::default()
            },
        );
        let narrow = tile(
            VIEWPORT,
            2,
            LayoutConfig {
                master_ratio: -1.0,
                ..LayoutConfig::default()
            },
        );
        assert!(wide[0].width > VIEWPORT.width / 2);
        assert!(narrow[0].width < VIEWPORT.width / 2);
    }

    #[test]
    fn tiny_viewport_does_not_panic_and_keeps_the_count() {
        let tiny = Rect::new(0, 0, 10, 10);
        let rects = tile(tiny, 5, LayoutConfig::default());
        assert_eq!(rects.len(), 5);
        let empty = tile(Rect::new(0, 0, 0, 0), 3, LayoutConfig::default());
        assert_eq!(empty.len(), 3);
        assert!(empty.iter().all(|rect| rect.is_empty()));
    }
}
