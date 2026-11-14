//! Software cursor bitmaps shared by compositor backends.
//!
//! The cursors are small white shapes with a black outline and an explicit
//! hotspot. They are backend-neutral: the winit adapter uploads them as memory
//! buffers, and a future DRM backend can draw the same pixels without an
//! xcursor theme dependency.

#![forbid(unsafe_code)]

pub mod theme;

pub use theme::{parse_xcursor, CursorTheme};

/// Cursor shapes used by the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CursorShape {
    Arrow,
    Pointer,
    Text,
    Move,
    Grabbing,
    ResizeNwse,
    ResizeNesw,
    ResizeHorizontal,
}

impl CursorShape {
    /// Xcursor theme names to try for this shape.
    pub fn cursor_names(self) -> &'static [&'static str] {
        match self {
            CursorShape::Arrow => &["default", "left_ptr"],
            CursorShape::Pointer => &["pointer", "hand2", "hand1", "hand"],
            CursorShape::Text => &["text", "xterm"],
            CursorShape::Move => &["move", "fleur", "all-scroll"],
            CursorShape::Grabbing => &["grabbing", "hand2", "fleur"],
            CursorShape::ResizeNwse => &[
                "nwse-resize",
                "top_left_corner",
                "bottom_right_corner",
                "bd_double_arrow",
                "fd_double_arrow",
            ],
            CursorShape::ResizeNesw => &[
                "nesw-resize",
                "top_right_corner",
                "bottom_left_corner",
                "crossed_circle",
            ],
            CursorShape::ResizeHorizontal => {
                &["ew-resize", "sb_h_double_arrow", "left_side", "right_side"]
            }
        }
    }
}

/// A rendered cursor bitmap with straight RGBA pixels and a hotspot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorBitmap {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: i32,
    pub hotspot_y: i32,
    pub pixels: Vec<u8>,
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

struct Canvas {
    width: i32,
    height: i32,
    pixels: Vec<[u8; 4]>,
}

impl Canvas {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; width as usize * height as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        self.pixels[(y * self.width + x) as usize] = color;
    }

    fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 4]) {
        for row in y..y + height {
            for column in x..x + width {
                self.set(column, row, color);
            }
        }
    }

    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.set(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x0 += sx;
            }
            if doubled <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn polygon(&mut self, points: &[(i32, i32)], color: [u8; 4]) {
        if points.len() < 3 {
            return;
        }
        let min_y = points.iter().map(|point| point.1).min().unwrap_or(0);
        let max_y = points.iter().map(|point| point.1).max().unwrap_or(0);
        for y in min_y..=max_y {
            let scan_y = y as f32 + 0.5;
            let mut crossings = Vec::new();
            for index in 0..points.len() {
                let (x0, y0) = points[index];
                let (x1, y1) = points[(index + 1) % points.len()];
                let (y0, y1) = (y0 as f32, y1 as f32);
                if (y0 <= scan_y && y1 > scan_y) || (y1 <= scan_y && y0 > scan_y) {
                    let t = (scan_y - y0) / (y1 - y0);
                    crossings.push(x0 as f32 + t * (x1 - x0) as f32);
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.as_chunks::<2>().0 {
                let start = pair[0].ceil() as i32;
                let end = pair[1].floor() as i32;
                for x in start..=end {
                    self.set(x, y, color);
                }
            }
        }
    }

    fn outline(&mut self) {
        let source = self.pixels.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                if source[(y * self.width + x) as usize][3] != 0 {
                    continue;
                }
                let mut touched = false;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < 0 || ny < 0 || nx >= self.width || ny >= self.height {
                            continue;
                        }
                        if source[(ny * self.width + nx) as usize][3] != 0 {
                            touched = true;
                        }
                    }
                }
                if touched {
                    self.pixels[(y * self.width + x) as usize] = BLACK;
                }
            }
        }
    }

    fn finish(self, hotspot_x: i32, hotspot_y: i32) -> CursorBitmap {
        let mut pixels = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in self.pixels {
            pixels.extend_from_slice(&pixel);
        }
        CursorBitmap {
            width: self.width as u32,
            height: self.height as u32,
            hotspot_x,
            hotspot_y,
            pixels,
        }
    }
}

/// Render a cursor shape.
pub fn render(shape: CursorShape) -> CursorBitmap {
    match shape {
        CursorShape::Arrow | CursorShape::Pointer | CursorShape::Grabbing => arrow(),
        CursorShape::Text => text(),
        CursorShape::Move => move_cursor(),
        CursorShape::ResizeNwse => resize_nwse(),
        CursorShape::ResizeNesw => resize_nesw(),
        CursorShape::ResizeHorizontal => resize_horizontal(),
    }
}

fn arrow() -> CursorBitmap {
    let mut canvas = Canvas::new(12, 19);
    canvas.polygon(
        &[
            (0, 0),
            (0, 16),
            (4, 12),
            (7, 18),
            (10, 17),
            (7, 11),
            (12, 11),
        ],
        WHITE,
    );
    canvas.outline();
    canvas.finish(0, 0)
}

fn text() -> CursorBitmap {
    let mut canvas = Canvas::new(13, 19);
    canvas.rect(5, 3, 2, 13, WHITE);
    canvas.rect(3, 3, 6, 1, WHITE);
    canvas.rect(3, 15, 6, 1, WHITE);
    canvas.outline();
    canvas.finish(6, 9)
}

fn move_cursor() -> CursorBitmap {
    let mut canvas = Canvas::new(16, 19);
    canvas.rect(7, 1, 2, 17, WHITE);
    canvas.rect(1, 8, 14, 2, WHITE);
    canvas.polygon(&[(7, 0), (3, 4), (11, 4)], WHITE);
    canvas.polygon(&[(7, 18), (3, 14), (11, 14)], WHITE);
    canvas.polygon(&[(0, 9), (4, 5), (4, 13)], WHITE);
    canvas.polygon(&[(15, 9), (11, 5), (11, 13)], WHITE);
    canvas.outline();
    canvas.finish(7, 9)
}

fn resize_nwse() -> CursorBitmap {
    let mut canvas = Canvas::new(19, 19);
    canvas.line(3, 3, 15, 15, WHITE);
    canvas.polygon(&[(0, 0), (7, 1), (1, 7)], WHITE);
    canvas.polygon(&[(18, 18), (11, 17), (17, 11)], WHITE);
    canvas.outline();
    canvas.finish(9, 9)
}

fn resize_nesw() -> CursorBitmap {
    let mut canvas = Canvas::new(19, 19);
    canvas.line(15, 3, 3, 15, WHITE);
    canvas.polygon(&[(18, 0), (11, 1), (17, 7)], WHITE);
    canvas.polygon(&[(0, 18), (7, 17), (1, 11)], WHITE);
    canvas.outline();
    canvas.finish(9, 9)
}

fn resize_horizontal() -> CursorBitmap {
    let mut canvas = Canvas::new(19, 17);
    canvas.rect(2, 8, 15, 2, WHITE);
    canvas.polygon(&[(0, 9), (6, 4), (6, 14)], WHITE);
    canvas.polygon(&[(18, 9), (12, 4), (12, 14)], WHITE);
    canvas.outline();
    canvas.finish(9, 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: [CursorShape; 8] = [
        CursorShape::Arrow,
        CursorShape::Pointer,
        CursorShape::Text,
        CursorShape::Move,
        CursorShape::Grabbing,
        CursorShape::ResizeNwse,
        CursorShape::ResizeNesw,
        CursorShape::ResizeHorizontal,
    ];

    #[test]
    fn every_shape_has_visible_pixels_and_an_outline() {
        for shape in SHAPES {
            let bitmap = render(shape);
            assert!(bitmap.width > 0 && bitmap.height > 0);
            assert_eq!(
                bitmap.pixels.len(),
                bitmap.width as usize * bitmap.height as usize * 4
            );
            let visible = bitmap
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[3] != 0)
                .count();
            let black = bitmap
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| **pixel == BLACK)
                .count();
            assert!(visible > 0, "shape {shape:?} is empty");
            assert!(black > 0, "shape {shape:?} has no outline");
        }
    }

    #[test]
    fn hotspots_are_inside_the_bitmap() {
        for shape in SHAPES {
            let bitmap = render(shape);
            assert!(bitmap.hotspot_x >= 0 && bitmap.hotspot_x < bitmap.width as i32);
            assert!(bitmap.hotspot_y >= 0 && bitmap.hotspot_y < bitmap.height as i32);
        }
    }

    #[test]
    fn shapes_have_distinct_pixels() {
        assert_ne!(
            render(CursorShape::Arrow).pixels,
            render(CursorShape::Text).pixels
        );
        assert_ne!(
            render(CursorShape::ResizeNwse).pixels,
            render(CursorShape::ResizeHorizontal).pixels
        );
    }
}
