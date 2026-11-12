//! Bitmap text rasterizer shared by the Aether preview and compositor backends.
//!
//! The renderer ships an 8x8 font, so text can be drawn into an RGBA buffer
//! without fontconfig, FreeType or a glyph atlas. Backends upload the buffer to
//! the GPU (Smithay's `MemoryRenderBuffer` in the winit/DRM adapters). The
//! preview binary draws the same pixels directly into its canvas.
//!
//! Buffers are tightly cropped and use straight (non-premultiplied) RGBA bytes,
//! matching `Fourcc::Abgr8888` on little-endian hosts.

#![forbid(unsafe_code)]

use font8x8::{UnicodeFonts, BASIC_FONTS};

/// Width of one glyph cell in pixels before scaling.
pub const GLYPH_WIDTH: u32 = 8;
/// Height of one glyph cell in pixels before scaling.
pub const GLYPH_HEIGHT: u32 = 8;

/// A tightly cropped RGBA bitmap containing rendered text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBitmap {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl TextBitmap {
    /// Render `text` at `scale` into a new RGBA bitmap.
    ///
    /// Newlines start a new line; unsupported characters advance the cursor but
    /// draw nothing, so layout stays stable for unknown input.
    pub fn render(text: &str, scale: u32, color: [u8; 4]) -> Self {
        let scale = scale.max(1);
        let lines: Vec<&str> = text.split('\n').collect();
        let columns = lines
            .iter()
            .map(|line| line.chars().count() as u32)
            .max()
            .unwrap_or(0);
        let width = columns.saturating_mul(GLYPH_WIDTH).saturating_mul(scale);
        let height = (lines.len() as u32)
            .saturating_mul(GLYPH_HEIGHT)
            .saturating_mul(scale);
        let mut bitmap = Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        };

        for (line_index, line) in lines.iter().enumerate() {
            let origin_y = line_index as u32 * GLYPH_HEIGHT * scale;
            for (column, character) in line.chars().enumerate() {
                let origin_x = column as u32 * GLYPH_WIDTH * scale;
                if let Some(glyph) = BASIC_FONTS.get(character) {
                    bitmap.blit_glyph(&glyph, origin_x, origin_y, scale, color);
                }
            }
        }

        bitmap
    }

    fn blit_glyph(&mut self, glyph: &[u8; 8], x: u32, y: u32, scale: u32, color: [u8; 4]) {
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..GLYPH_WIDTH {
                if bits & (1 << column) == 0 {
                    continue;
                }
                let pixel_x = x + column * scale;
                let pixel_y = y + row as u32 * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        self.set_pixel(pixel_x + dx, pixel_y + dy, color);
                    }
                }
            }
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = ((y * self.width + x) * 4) as usize;
        self.pixels[index..index + 4].copy_from_slice(&color);
    }

    /// Width of the bitmap in physical pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height of the bitmap in physical pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Straight RGBA bytes, row-major, four bytes per pixel.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Whether the bitmap has no drawable area.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Pixel width `text` would occupy when rendered at `scale`.
pub fn text_width(text: &str, scale: u32) -> u32 {
    text.lines()
        .map(|line| line.chars().count() as u32)
        .max()
        .unwrap_or(0)
        .saturating_mul(GLYPH_WIDTH)
        .saturating_mul(scale.max(1))
}

/// Pixel height `text` would occupy when rendered at `scale`.
pub fn text_height(text: &str, scale: u32) -> u32 {
    (text.lines().count().max(1) as u32)
        .saturating_mul(GLYPH_HEIGHT)
        .saturating_mul(scale.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_glyph_cells() {
        assert_eq!(text_width("Aether", 1), 6 * 8);
        assert_eq!(text_width("Aether", 2), 6 * 16);
        assert_eq!(text_height("Aether", 1), 8);
        assert_eq!(text_height("a\nbc", 1), 16);
    }

    #[test]
    fn renders_the_requested_size() {
        let bitmap = TextBitmap::render("AB", 2, [255, 0, 0, 255]);
        assert_eq!(bitmap.width(), 32);
        assert_eq!(bitmap.height(), 16);
        assert_eq!(bitmap.pixels().len(), 32 * 16 * 4);
    }

    #[test]
    fn draws_colored_pixels_and_leaves_background_transparent() {
        let bitmap = TextBitmap::render("I", 1, [10, 20, 30, 255]);
        let colored = bitmap
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| **pixel == [10, 20, 30, 255])
            .count();
        let transparent = bitmap
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| **pixel == [0, 0, 0, 0])
            .count();
        assert!(colored > 0);
        assert!(transparent > colored);
    }

    #[test]
    fn empty_text_produces_an_empty_bitmap() {
        let bitmap = TextBitmap::render("", 1, [255, 255, 255, 255]);
        assert!(bitmap.is_empty());
        assert_eq!(bitmap.pixels(), &[] as &[u8]);
    }

    #[test]
    fn unknown_characters_keep_their_advance() {
        let width = text_width("A☃B", 1);
        assert_eq!(width, 3 * 8);
    }
}
