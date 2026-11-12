//! Software renderer for [`aether_shell::FramePlan`].
//!
//! This is a development preview, not a GPU renderer: it draws the shell's
//! frame plan into an RGBA canvas and writes a PNG so the scene, tiling,
//! focus, overlay and damage can be inspected without a display server. The
//! Smithay backend will consume the same `FramePlan` on the target.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

use aether_geometry::{Rect, Size};
use aether_shell::{FrameItem, FramePlan};
use aether_text::TextBitmap;

const BG_TOP: [u8; 4] = [5, 31, 41, 255];
const BG_BOTTOM: [u8; 4] = [1, 9, 13, 255];
const WINDOW: [u8; 4] = [9, 39, 51, 255];
const TITLE_BAR: [u8; 4] = [11, 57, 71, 255];
const BORDER: [u8; 4] = [31, 76, 90, 255];
const ACCENT: [u8; 4] = [56, 214, 255, 255];
const TEXT: [u8; 4] = [233, 251, 255, 255];
const TEXT_DIM: [u8; 4] = [107, 169, 182, 255];
const OVERLAY: [u8; 4] = [13, 61, 75, 255];
const STATUS: [u8; 4] = [3, 20, 27, 255];
const DAMAGE: [u8; 4] = [255, 90, 90, 70];

pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(viewport: Size) -> Self {
        let width = viewport.width.max(1);
        let height = viewport.height.max(1);
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = ((y * self.width + x) * 4) as usize;
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ])
    }

    pub fn fill(&mut self, color: [u8; 4]) {
        for pixel in self.pixels.as_chunks_mut::<4>().0 {
            *pixel = color;
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: [u8; 4]) {
        let x0 = rect.x.max(0) as u32;
        let y0 = rect.y.max(0) as u32;
        let x1 = (rect.x as i64 + rect.width as i64).clamp(0, self.width as i64) as u32;
        let y1 = (rect.y as i64 + rect.height as i64).clamp(0, self.height as i64) as u32;
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, color);
            }
        }
    }

    pub fn fill_rect_alpha(&mut self, rect: Rect, color: [u8; 4]) {
        let alpha = color[3] as f32 / 255.0;
        for y in rect.y.max(0)..(rect.y as i64 + rect.height as i64).min(self.height as i64) as i32
        {
            for x in
                rect.x.max(0)..(rect.x as i64 + rect.width as i64).min(self.width as i64) as i32
            {
                if let Some(existing) = self.pixel(x as u32, y as u32) {
                    let blended = [
                        (existing[0] as f32 * (1.0 - alpha) + color[0] as f32 * alpha) as u8,
                        (existing[1] as f32 * (1.0 - alpha) + color[1] as f32 * alpha) as u8,
                        (existing[2] as f32 * (1.0 - alpha) + color[2] as f32 * alpha) as u8,
                        255,
                    ];
                    self.put(x as u32, y as u32, blended);
                }
            }
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: [u8; 4], thickness: u32) {
        if rect.is_empty() || thickness == 0 {
            return;
        }
        let thickness = thickness as i32;
        self.fill_rect(
            Rect::new(rect.x, rect.y, rect.width, thickness as u32),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x,
                rect.y + rect.height as i32 - thickness,
                rect.width,
                thickness as u32,
            ),
            color,
        );
        self.fill_rect(
            Rect::new(rect.x, rect.y, thickness as u32, rect.height),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x + rect.width as i32 - thickness,
                rect.y,
                thickness as u32,
                rect.height,
            ),
            color,
        );
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, scale: u32, color: [u8; 4]) {
        let bitmap = TextBitmap::render(text, scale, color);
        self.blit_rgba(x, y, bitmap.width(), bitmap.height(), bitmap.pixels());
    }

    /// Alpha-composite a tightly cropped RGBA bitmap at `(x, y)`.
    pub fn blit_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, pixels: &[u8]) {
        for row in 0..height {
            for column in 0..width {
                let index = ((row * width + column) * 4) as usize;
                let source = &pixels[index..index + 4];
                let alpha = source[3] as f32 / 255.0;
                if alpha == 0.0 {
                    continue;
                }
                let target_x = x + column as i32;
                let target_y = y + row as i32;
                if target_x < 0 || target_y < 0 {
                    continue;
                }
                let Some(existing) = self.pixel(target_x as u32, target_y as u32) else {
                    continue;
                };
                let blended = [
                    (existing[0] as f32 * (1.0 - alpha) + source[0] as f32 * alpha) as u8,
                    (existing[1] as f32 * (1.0 - alpha) + source[1] as f32 * alpha) as u8,
                    (existing[2] as f32 * (1.0 - alpha) + source[2] as f32 * alpha) as u8,
                    existing[3],
                ];
                self.put(target_x as u32, target_y as u32, blended);
            }
        }
    }

    pub fn save_png(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(io::Error::other)?;
        writer
            .write_image_data(&self.pixels)
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn background(&mut self) {
        for y in 0..self.height {
            let t = y as f32 / self.height as f32;
            let color = [
                lerp(BG_TOP[0], BG_BOTTOM[0], t),
                lerp(BG_TOP[1], BG_BOTTOM[1], t),
                lerp(BG_TOP[2], BG_BOTTOM[2], t),
                255,
            ];
            self.fill_rect(Rect::new(0, y as i32, self.width, 1), color);
        }
    }

    fn put(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = ((y * self.width + x) * 4) as usize;
        self.pixels[index..index + 4].copy_from_slice(&color);
    }
}

fn lerp(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t.clamp(0.0, 1.0)) as u8
}

/// Render a shell frame to a canvas. `debug_damage` overlays the damage region.
pub fn render(plan: &FramePlan, debug_damage: bool) -> Canvas {
    let mut canvas = Canvas::new(plan.viewport);
    canvas.background();

    for item in &plan.items {
        match item.layer {
            aether_scene::Layer::OVERLAY => draw_overlay(&mut canvas, item),
            _ => draw_window(&mut canvas, item),
        }
    }

    draw_status(&mut canvas, plan);

    if debug_damage {
        for rect in plan.damage.rects() {
            canvas.fill_rect_alpha(*rect, DAMAGE);
            canvas.stroke_rect(*rect, [255, 90, 90, 255], 2);
        }
        canvas.draw_text(16, 16, "DAMAGE", 2, [255, 130, 130, 255]);
    }

    canvas
}

fn draw_window(canvas: &mut Canvas, item: &FrameItem) {
    let rect = item.rect;
    if rect.is_empty() {
        return;
    }
    canvas.fill_rect(rect, WINDOW);
    let title_height = 28.min(rect.height);
    canvas.fill_rect(
        Rect::new(rect.x, rect.y, rect.width, title_height),
        TITLE_BAR,
    );
    canvas.stroke_rect(
        rect,
        if item.focused { ACCENT } else { BORDER },
        if item.focused { 2 } else { 1 },
    );
    canvas.draw_text(
        rect.x + 10,
        rect.y + 9,
        &truncate(&item.title, (rect.width / 16) as usize),
        2,
        if item.focused { TEXT } else { TEXT_DIM },
    );

    // Fake application content: a few text-like rules.
    let content_y = rect.y + title_height as i32 + 12;
    let width = (rect.width as i32 - 24).max(0);
    for line in 0..6 {
        let line_width = (width as f32 * (0.8 - line as f32 * 0.09)).max(0.0) as u32;
        canvas.fill_rect(
            Rect::new(rect.x + 12, content_y + line * 16, line_width, 4),
            if line == 0 { TEXT_DIM } else { BORDER },
        );
    }
}

fn draw_overlay(canvas: &mut Canvas, item: &FrameItem) {
    let rect = item.rect;
    canvas.fill_rect(rect, OVERLAY);
    canvas.stroke_rect(rect, ACCENT, 2);
    canvas.draw_text(rect.x + 14, rect.y + 12, "Command Palette", 1, TEXT);
    canvas.fill_rect(
        Rect::new(
            rect.x + 12,
            rect.y + 32,
            (rect.width as i32 - 24).max(0) as u32,
            26,
        ),
        TITLE_BAR,
    );
    canvas.draw_text(rect.x + 20, rect.y + 38, "> index", 2, TEXT_DIM);

    let rows = ["indexer.rs", "Rebuild Search Index", "Refactor Worker"];
    for (index, row) in rows.iter().enumerate() {
        let y = rect.y + 78 + index as i32 * 42;
        if index == 0 {
            canvas.fill_rect(
                Rect::new(
                    rect.x + 10,
                    y - 8,
                    (rect.width as i32 - 20).max(0) as u32,
                    34,
                ),
                [20, 78, 94, 255],
            );
        }
        canvas.draw_text(
            rect.x + 20,
            y,
            row,
            1,
            if index == 0 { TEXT } else { TEXT_DIM },
        );
    }
}

fn draw_status(canvas: &mut Canvas, plan: &FramePlan) {
    let height = 30.min(plan.viewport.height);
    let y = plan.viewport.height as i32 - height as i32;
    canvas.fill_rect(Rect::new(0, y, plan.viewport.width, height), STATUS);
    canvas.fill_rect(Rect::new(0, y, plan.viewport.width, 1), BORDER);
    let text = format!(
        "aether-shell   workspace {}   {} windows   {} items   {} px damage",
        plan.workspace + 1,
        plan.items
            .iter()
            .filter(|item| item.window.is_some())
            .count(),
        plan.items.len(),
        plan.damage.area()
    );
    canvas.draw_text(12, y + 10, &text, 1, TEXT_DIM);
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars.max(4) {
        return text.to_string();
    }
    let mut result: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    result.push('…');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_geometry::Size;
    use aether_layout::LayoutConfig;
    use aether_shell::{Shell, WindowSpec};

    fn plan() -> FramePlan {
        let mut shell = Shell::new(Size::new(800, 600), LayoutConfig::default(), 120);
        shell.add_window(WindowSpec::new("terminal", "aether.terminal"));
        shell.add_window(WindowSpec::new("browser", "aether.browser"));
        shell.set_overlay(Some(Rect::new(120, 90, 420, 300)));
        shell.frame(0).expect("frame")
    }

    #[test]
    fn canvas_writes_a_png() {
        let mut canvas = Canvas::new(Size::new(64, 32));
        canvas.fill([10, 20, 30, 255]);
        let path = std::env::temp_dir().join(format!("aether-preview-{}.png", std::process::id()));
        canvas.save_png(&path).expect("png");
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn primitives_change_pixels() {
        let mut canvas = Canvas::new(Size::new(64, 32));
        canvas.fill([0, 0, 0, 255]);
        canvas.fill_rect(Rect::new(4, 4, 10, 10), [255, 0, 0, 255]);
        assert_eq!(canvas.pixel(5, 5), Some([255, 0, 0, 255]));
        canvas.draw_text(0, 0, "A", 1, [255, 255, 255, 255]);
        assert!(canvas
            .pixels()
            .as_chunks::<4>()
            .0
            .contains(&[255, 255, 255, 255]));
    }

    #[test]
    fn renders_the_shell_frame_and_damage_overlay() {
        let plan = plan();
        let normal = render(&plan, false);
        let debug = render(&plan, true);
        assert_eq!((normal.width(), normal.height()), (800, 600));
        assert_ne!(normal.pixels(), debug.pixels());
        assert!(plan.damage.area() > 0);
        assert!(plan
            .items
            .iter()
            .any(|item| item.layer == aether_scene::Layer::OVERLAY));
    }

    #[test]
    fn out_of_bounds_draws_are_clipped() {
        let mut canvas = Canvas::new(Size::new(32, 32));
        canvas.fill([0, 0, 0, 255]);
        canvas.fill_rect(Rect::new(-10, -10, 20, 20), [255, 0, 0, 255]);
        canvas.fill_rect(Rect::new(30, 30, 100, 100), [0, 255, 0, 255]);
        assert_eq!(canvas.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(canvas.pixel(31, 31), Some([0, 255, 0, 255]));
        assert_eq!(canvas.pixel(32, 32), None);
    }
}
