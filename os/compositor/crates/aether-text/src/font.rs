//! Antialiased system-font rasterization for compositor chrome.
//!
//! The 8x8 bitmap font remains the fallback and the preview renderer. When
//! `AETHER_FONT` (or a standard Inter path) is available, labels use this
//! renderer instead.

use std::path::{Path, PathBuf};

use fontdue::Font;

use crate::TextBitmap;

pub const BODY_PX: f32 = 13.0;
pub const SMALL_PX: f32 = 11.0;
pub const TITLE_PX: f32 = 20.0;
pub const HEADING_PX: f32 = 16.0;
/// Resource captions, weekday headers and event metadata.
pub const TINY_PX: f32 = 9.0;
/// Resource values and agenda times.
pub const MICRO_PX: f32 = 10.0;
/// Resource captions and other 8 px chrome labels.
pub const CAPTION_PX: f32 = 8.0;
/// Explicit OS-concept sizes: the demo scales its type by 1.14.
pub const OS_CAPTION_PX: f32 = 7.98;
pub const OS_MICRO_PX: f32 = 9.69;
pub const OS_SMALL_PX: f32 = 11.4;
pub const OS_XS_PX: f32 = 12.54;
pub const OS_SM_PX: f32 = 14.82;
/// Popover rows: `.os-menu__pop button` (`--ds-text-sm`).
pub const OS_MENU_PX: f32 = 13.68;

pub struct FontRenderer {
    font: Font,
    path: PathBuf,
}

impl FontRenderer {
    pub fn open(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let font = Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()?;
        Some(Self {
            font,
            path: path.to_path_buf(),
        })
    }

    /// `AETHER_FONT`, then common Inter locations.
    pub fn discover() -> Option<Self> {
        if let Some(path) = std::env::var_os("AETHER_FONT") {
            if let Some(font) = Self::open(Path::new(&path)) {
                return Some(font);
            }
        }
        let candidates = [
            "/run/current-system/sw/share/fonts/truetype/InterVariable.ttf",
            "/usr/share/fonts/truetype/inter/InterVariable.ttf",
            "/usr/share/fonts/truetype/InterVariable.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ];
        for candidate in candidates {
            if let Some(font) = Self::open(Path::new(candidate)) {
                return Some(font);
            }
        }
        // Nix stores are content-addressed; find an Inter font if it was not
        // exported explicitly (development shells).
        if let Ok(entries) = std::fs::read_dir("/nix/store") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("inter-") {
                    let path = entry.path().join("share/fonts/truetype/InterVariable.ttf");
                    if let Some(font) = Self::open(&path) {
                        return Some(font);
                    }
                }
            }
        }
        None
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn measure(&self, text: &str, px: f32, bold: bool, spacing: f32) -> (u32, u32) {
        let mut width = 0.0f32;
        let mut lines = 0u32;
        for line in text.split('\n') {
            lines += 1;
            let mut line_width = 0.0f32;
            let count = line.chars().count();
            for character in line.chars() {
                line_width += self.font.metrics(character, px).advance_width;
            }
            if count > 1 {
                line_width += spacing * (count - 1) as f32;
            }
            width = width.max(line_width);
        }
        let line_height = self
            .font
            .horizontal_line_metrics(px)
            .map_or(px * 1.2, |m| m.new_line_size);
        let extra = if bold { 1.0 } else { 0.0 };
        (
            (width + extra).ceil() as u32,
            (line_height * lines as f32).ceil() as u32,
        )
    }

    pub fn render(
        &self,
        text: &str,
        px: f32,
        color: [u8; 4],
        bold: bool,
        spacing: f32,
    ) -> TextBitmap {
        let (width, height) = self.measure(text, px, bold, spacing);
        if width == 0 || height == 0 {
            return TextBitmap::empty();
        }
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let line_metrics = self.font.horizontal_line_metrics(px);
        let ascent = line_metrics.map_or(px * 0.8, |m| m.ascent);
        for (line_index, line) in text.split('\n').enumerate() {
            let baseline = ascent + (line_index as f32 * px * 1.25);
            let mut x = 0.0f32;
            for character in line.chars() {
                let (metrics, bitmap) = self.font.rasterize(character, px);
                let glyph_x = x + metrics.xmin as f32;
                let glyph_y = baseline - metrics.height as f32 - metrics.ymin as f32;
                blit(
                    &mut pixels,
                    width,
                    height,
                    &bitmap,
                    metrics.width,
                    metrics.height,
                    glyph_x,
                    glyph_y,
                    color,
                );
                x += metrics.advance_width + spacing;
                if bold {
                    blit(
                        &mut pixels,
                        width,
                        height,
                        &bitmap,
                        metrics.width,
                        metrics.height,
                        glyph_x + 1.0,
                        glyph_y,
                        color,
                    );
                }
            }
        }
        TextBitmap::from_rgba(width, height, pixels)
    }
}

#[allow(clippy::too_many_arguments)]
fn blit(
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    coverage: &[u8],
    glyph_width: usize,
    glyph_height: usize,
    x: f32,
    y: f32,
    color: [u8; 4],
) {
    let origin_x = x.round() as i32;
    let origin_y = y.round() as i32;
    for row in 0..glyph_height {
        for column in 0..glyph_width {
            let alpha = coverage[row * glyph_width + column];
            if alpha == 0 {
                continue;
            }
            let px = origin_x + column as i32;
            let py = origin_y + row as i32;
            if px < 0 || py < 0 || px >= target_width as i32 || py >= target_height as i32 {
                continue;
            }
            let index = ((py as u32 * target_width + px as u32) * 4) as usize;
            let src_alpha = (u16::from(alpha) * u16::from(color[3]) / 255) as u8;
            let dst_alpha = target[index + 3];
            let out_alpha = (u16::from(src_alpha)
                + u16::from(dst_alpha) * (255 - u16::from(src_alpha)) / 255)
                .min(255) as u8;
            for channel in 0..3 {
                let src = u32::from(color[channel]) * u32::from(src_alpha) / 255;
                let dst = u32::from(target[index + channel])
                    * u32::from(dst_alpha)
                    * (255 - u32::from(src_alpha))
                    / (255 * 255);
                target[index + channel] = (src + dst).min(255) as u8;
            }
            target[index + 3] = out_alpha;
        }
    }
}
