//! Antialiased system-font rasterization for compositor chrome.
//!
//! The 8x8 bitmap font remains the fallback and the preview renderer. When
//! `AETHER_FONT` (or a standard Inter path) is available, labels use this
//! renderer instead.
//!
//! Swash renders the variable font at the requested weight axis, so the
//! demo's 550/600/650/720 weights get real advances instead of faux bold.

use std::path::{Path, PathBuf};

use swash::scale::image::{Content, Image};
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;
use swash::FontRef;

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
/// Sidebar group labels: `.os-settings-nav__label` (`8px * 1.14`).
pub const OS_LABEL_CAPS_PX: f32 = 9.12;

const WGHT: u32 = 0x77676874; // "wght"

pub struct FontRenderer {
    data: Vec<u8>,
    path: PathBuf,
}

fn normalized_weight(font: &FontRef, weight: u16) -> Vec<i16> {
    font.variations()
        .map(|variation| {
            let value = if variation.tag() == WGHT {
                f32::from(weight)
            } else {
                variation.default_value()
            };
            variation.normalize(value)
        })
        .collect()
}

impl FontRenderer {
    pub fn open(path: &Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        FontRef::from_index(&data, 0)?;
        Some(Self {
            data,
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
            "/usr/share/fonts/truetype/inter-vf/InterVariable.ttf",
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

    fn line_height(font: &FontRef, coords: &[i16], px: f32) -> f32 {
        let metrics = font.metrics(coords).scale(px);
        metrics.ascent + metrics.descent + metrics.leading
    }

    pub fn measure(&self, text: &str, px: f32, weight: u16, spacing: f32) -> (u32, u32) {
        let Some(font) = FontRef::from_index(&self.data, 0) else {
            return (text.chars().count() as u32 * 6, (px * 1.2).ceil() as u32);
        };
        let coords = normalized_weight(&font, weight);
        let advances = font.glyph_metrics(&coords).scale(px);
        let mut width = 0.0f32;
        let mut lines = 0u32;
        for line in text.split('\n') {
            lines += 1;
            let mut line_width = 0.0f32;
            let count = line.chars().count();
            for character in line.chars() {
                line_width += advances.advance_width(font.charmap().map(character));
            }
            if count > 1 {
                line_width += spacing * (count - 1) as f32;
            }
            width = width.max(line_width);
        }
        let height = Self::line_height(&font, &coords, px);
        (width.ceil() as u32, (height * lines as f32).ceil() as u32)
    }

    pub fn render(
        &self,
        text: &str,
        px: f32,
        color: [u8; 4],
        weight: u16,
        spacing: f32,
    ) -> TextBitmap {
        let (width, height) = self.measure(text, px, weight, spacing);
        if width == 0 || height == 0 {
            return TextBitmap::empty();
        }
        let Some(font) = FontRef::from_index(&self.data, 0) else {
            return TextBitmap::empty();
        };
        let coords = normalized_weight(&font, weight);
        let advances = font.glyph_metrics(&coords).scale(px);
        let line_height = Self::line_height(&font, &coords, px);
        let ascent = font.metrics(&coords).scale(px).ascent;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let mut context = ScaleContext::new();
        let mut scaler = context
            .builder(font)
            .size(px)
            .variations(&[("wght", f32::from(weight))])
            .build();
        let sources = [Source::Outline];
        let mut render = Render::new(&sources);
        render.format(Format::Alpha);
        for (line_index, line) in text.split('\n').enumerate() {
            let baseline = ascent + (line_index as f32 * line_height);
            let mut pen = 0.0f32;
            for character in line.chars() {
                let glyph = font.charmap().map(character);
                if let Some(image) = render.render(&mut scaler, glyph) {
                    let x = pen + image.placement.left as f32;
                    let y = baseline - image.placement.top as f32;
                    blit_image(&mut pixels, width, height, &image, x, y, color);
                }
                pen += advances.advance_width(glyph) + spacing;
            }
        }
        TextBitmap::from_rgba(width, height, pixels)
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_image(
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    image: &Image,
    x: f32,
    y: f32,
    color: [u8; 4],
) {
    if image.content != Content::Mask {
        return;
    }
    let origin_x = x.round() as i32;
    let origin_y = y.round() as i32;
    let glyph_width = image.placement.width as usize;
    let glyph_height = image.placement.height as usize;
    for row in 0..glyph_height {
        for column in 0..glyph_width {
            let alpha = image.data[row * glyph_width + column];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_change_advances() {
        let Some(renderer) = FontRenderer::discover() else {
            return;
        };
        let regular = renderer.measure("Desktop", 12.54, 400, 0.0).0;
        let bold = renderer.measure("Desktop", 12.54, 700, 0.0).0;
        assert!(bold >= regular);
    }
}
