//! Equal-column tiling.
//!
//! One window fills the work area. Every additional window gets its own
//! equal-width column. Gaps are drawn by the compositor, not by clients.

#![forbid(unsafe_code)]

use aether_geometry::Rect;

#[derive(Clone, Copy, Debug)]
pub struct LayoutConfig {
    /// Gap between windows.
    pub gap: u32,
    /// Gap between the work area and the output edge.
    pub outer_gap: u32,
    /// Extra space reserved above the work area for shell chrome such as the
    /// top shortcut dock. Applies to tiled and maximized windows.
    pub top_gap: u32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            gap: 6,
            outer_gap: 8,
            top_gap: 0,
        }
    }
}

/// The usable area below outer gaps and reserved top chrome.
pub fn work_area(viewport: Rect, config: LayoutConfig) -> Rect {
    let inner = viewport.inset(config.outer_gap as i32);
    Rect::new(
        inner.x,
        inner.y + config.top_gap as i32,
        inner.width,
        inner.height.saturating_sub(config.top_gap),
    )
}

/// Tile `count` windows into equal-width columns across `viewport`.
pub fn tile(viewport: Rect, count: usize, config: LayoutConfig) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }

    let inner = work_area(viewport, config);
    if inner.is_empty() {
        return vec![Rect::default(); count];
    }
    if count == 1 {
        return vec![inner];
    }

    let gap = config.gap as i32;
    let total_gap = gap * (count as i32 - 1);
    let available = (inner.width as i32 - total_gap).max(0);
    let base = available / count as i32;
    let mut rects = Vec::with_capacity(count);
    let mut x = inner.x;

    for index in 0..count {
        let width = if index + 1 == count {
            (inner.x + inner.width as i32 - x).max(0)
        } else {
            base.max(0)
        };
        rects.push(Rect::new(x, inner.y, width as u32, inner.height));
        x += width + gap;
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

    fn assert_equal_columns(rects: &[Rect]) {
        assert_no_overlap(rects);
        for window in rects.windows(2) {
            assert!(
                (window[0].width as i32 - window[1].width as i32).abs() <= 2,
                "columns differ: {window:?}"
            );
            assert_eq!(window[0].height, window[1].height);
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
    fn two_windows_split_into_equal_columns() {
        let rects = tile(VIEWPORT, 2, LayoutConfig::default());
        assert_eq!(rects.len(), 2);
        assert_equal_columns(&rects);
        let inner = VIEWPORT.inset(8);
        for rect in &rects {
            assert!(inner.intersect(*rect) == Some(*rect), "outside inner area");
        }
    }

    #[test]
    fn each_window_gets_its_own_column() {
        for count in 2..=7 {
            let rects = tile(VIEWPORT, count, LayoutConfig::default());
            assert_eq!(rects.len(), count);
            assert_equal_columns(&rects);
            let mut xs: Vec<i32> = rects.iter().map(|rect| rect.x).collect();
            xs.sort_unstable();
            xs.dedup();
            assert_eq!(xs.len(), count, "one column per window");
        }
    }

    #[test]
    fn top_gap_is_reserved_above_the_work_area() {
        let config = LayoutConfig {
            gap: 10,
            outer_gap: 8,
            top_gap: 48,
        };
        let area = work_area(VIEWPORT, config);
        assert_eq!(area, Rect::new(8, 56, 1584, 936));
        let rects = tile(VIEWPORT, 2, config);
        assert!(rects.iter().all(|rect| rect.y == 56 && rect.height == 936));
    }

    #[test]
    fn gap_is_respected() {
        let config = LayoutConfig {
            gap: 12,
            outer_gap: 0,
            top_gap: 0,
        };
        let rects = tile(VIEWPORT, 3, config);
        assert_eq!(rects[1].x - (rects[0].x + rects[0].width as i32), 12);
        assert_eq!(rects[2].x - (rects[1].x + rects[1].width as i32), 12);
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
