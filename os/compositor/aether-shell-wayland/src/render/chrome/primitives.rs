//! Design-system chrome primitives: rounded glass panels, shadows, icons and
//! background gradients rendered into RGBA buffers.

use aether_shell::theme::{ThemeMode, WaterBackground, WaterSpeed, WaterTone};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::utils::Transform;

/// Inputs that change the workspace backdrop. Kept `Copy + Hash` so the
/// renderer can use it as a buffer cache key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BackgroundStyle {
    pub mode: ThemeMode,
    pub background: WaterBackground,
    pub tone: WaterTone,
    pub caustics: bool,
    pub effects: bool,
    pub speed: WaterSpeed,
}

fn buffer(width: u32, height: u32, pixels: &[u8]) -> MemoryRenderBuffer {
    MemoryRenderBuffer::from_slice(
        pixels,
        Fourcc::Abgr8888,
        (width as i32, height as i32),
        1,
        Transform::Normal,
        None,
    )
}

/// `MemoryRenderBuffer` expects premultiplied RGBA; tiny-skia produces the
/// same layout, so pass it through directly.
fn straight(pixmap: &resvg::tiny_skia::Pixmap) -> Vec<u8> {
    pixmap.data().to_vec()
}

fn rounded_path(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> Option<resvg::tiny_skia::Path> {
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let mut pb = resvg::tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + width - r, y);
    pb.quad_to(x + width, y, x + width, y + r);
    pb.line_to(x + width, y + height - r);
    pb.quad_to(x + width, y + height, x + width - r, y + height);
    pb.line_to(x + r, y + height);
    pb.quad_to(x, y + height, x, y + height - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Rasterize an inline SVG into a square RGBA buffer.
pub fn svg(svg: &str, size: u32) -> Option<MemoryRenderBuffer> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let scale = size as f32 / tree.size().width().max(1.0);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(buffer(size, size, &straight(&pixmap)))
}

/// Rounded panel with optional 1 px border and a soft stack shadow.
pub fn panel(
    width: u32,
    height: u32,
    radius: f32,
    fill: [u8; 4],
    border: Option<[u8; 4]>,
) -> Option<MemoryRenderBuffer> {
    const PAD: u32 = 18;
    let w = width + PAD * 2;
    let h = height + PAD * 2;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    // Soft shadow: a few translucent rounded layers.
    for (offset, alpha, spread) in [(10.0, 26u8, 8.0), (6.0, 34, 4.0), (3.0, 40, 1.0)] {
        let mut paint = resvg::tiny_skia::Paint::default();
        paint.set_color_rgba8(0, 6, 10, alpha);
        if let Some(path) = rounded_path(
            PAD as f32 - spread,
            PAD as f32 - spread + offset,
            width as f32 + spread * 2.0,
            height as f32 + spread * 2.0,
            radius + spread,
        ) {
            pixmap.fill_path(
                &path,
                &paint,
                resvg::tiny_skia::FillRule::Winding,
                resvg::tiny_skia::Transform::identity(),
                None,
            );
        }
    }
    let mut paint = resvg::tiny_skia::Paint::default();
    paint.set_color_rgba8(fill[0], fill[1], fill[2], fill[3]);
    let path = rounded_path(PAD as f32, PAD as f32, width as f32, height as f32, radius)?;
    pixmap.fill_path(
        &path,
        &paint,
        resvg::tiny_skia::FillRule::Winding,
        resvg::tiny_skia::Transform::identity(),
        None,
    );
    if let Some(border) = border {
        let mut paint = resvg::tiny_skia::Paint::default();
        paint.set_color_rgba8(border[0], border[1], border[2], border[3]);
        paint.anti_alias = true;
        if let Some(path) = rounded_path(
            PAD as f32 + 0.5,
            PAD as f32 + 0.5,
            width as f32 - 1.0,
            height as f32 - 1.0,
            radius,
        ) {
            pixmap.stroke_path(
                &path,
                &paint,
                &resvg::tiny_skia::Stroke {
                    width: 1.0,
                    ..Default::default()
                },
                resvg::tiny_skia::Transform::identity(),
                None,
            );
        }
    }
    Some(buffer(w, h, &straight(&pixmap)))
}

/// Rounded top corners with a square bottom edge, used for panel headers.
pub fn rounded_top(
    width: u32,
    height: u32,
    radius: f32,
    fill: [u8; 4],
) -> Option<MemoryRenderBuffer> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    let mut paint = resvg::tiny_skia::Paint::default();
    paint.set_color_rgba8(fill[0], fill[1], fill[2], fill[3]);
    let r = radius.min(width as f32 / 2.0).min(height as f32);
    let (x, y, w, h) = (
        0.5f32,
        0.5f32,
        width.max(1) as f32 - 1.0,
        height.max(1) as f32,
    );
    let mut pb = resvg::tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    if let Some(path) = pb.finish() {
        pixmap.fill_path(
            &path,
            &paint,
            resvg::tiny_skia::FillRule::Winding,
            resvg::tiny_skia::Transform::identity(),
            None,
        );
    }
    Some(buffer(width.max(1), height.max(1), &straight(&pixmap)))
}

/// Rounded fill used for selected rows, pills and badges.
pub fn rounded(width: u32, height: u32, radius: f32, fill: [u8; 4]) -> Option<MemoryRenderBuffer> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    let mut paint = resvg::tiny_skia::Paint::default();
    paint.set_color_rgba8(fill[0], fill[1], fill[2], fill[3]);
    if let Some(path) = rounded_path(
        0.5,
        0.5,
        width.max(1) as f32 - 1.0,
        height.max(1) as f32 - 1.0,
        radius,
    ) {
        pixmap.fill_path(
            &path,
            &paint,
            resvg::tiny_skia::FillRule::Winding,
            resvg::tiny_skia::Transform::identity(),
            None,
        );
    }
    Some(buffer(width.max(1), height.max(1), &straight(&pixmap)))
}

/// Water/Dark/Light workspace backdrop. Water adds caustic glows and, when
/// the Water background is `Sea`, slow wave bands; both honour the effects
/// switches and the tone.
pub fn background(width: u32, height: u32, style: BackgroundStyle) -> Option<MemoryRenderBuffer> {
    let mode = style.mode.effective();
    let light = mode == ThemeMode::Light;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    let (top, middle, bottom) = match mode {
        ThemeMode::Light => ([245, 252, 254], [233, 246, 250], [226, 242, 246]),
        ThemeMode::Dark => ([10, 35, 48], [5, 22, 31], [2, 12, 18]),
        _ => ([8, 53, 69], [5, 37, 48], [3, 25, 34]),
    };
    let base = resvg::tiny_skia::LinearGradient::new(
        resvg::tiny_skia::Point::from_xy(0.0, 0.0),
        resvg::tiny_skia::Point::from_xy(width as f32, height as f32),
        vec![
            resvg::tiny_skia::GradientStop::new(
                0.0,
                resvg::tiny_skia::Color::from_rgba8(top[0], top[1], top[2], 255),
            ),
            resvg::tiny_skia::GradientStop::new(
                0.5,
                resvg::tiny_skia::Color::from_rgba8(middle[0], middle[1], middle[2], 255),
            ),
            resvg::tiny_skia::GradientStop::new(
                1.0,
                resvg::tiny_skia::Color::from_rgba8(bottom[0], bottom[1], bottom[2], 255),
            ),
        ],
        resvg::tiny_skia::SpreadMode::Pad,
        resvg::tiny_skia::Transform::identity(),
    )?;
    let paint = resvg::tiny_skia::Paint {
        shader: base,
        anti_alias: false,
        ..Default::default()
    };
    pixmap.fill_rect(
        resvg::tiny_skia::Rect::from_xywh(0.0, 0.0, width as f32, height as f32)?,
        &paint,
        resvg::tiny_skia::Transform::identity(),
        None,
    );

    let accent = style.tone.rgb(light);
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;

    if style.effects && style.caustics {
        for index in 0..3 {
            let radius = w.max(h) * (0.24 + 0.08 * index as f32);
            let cx = w * (0.18 + 0.3 * index as f32);
            let cy = h * (0.22 + 0.24 * index as f32);
            let mut paint = resvg::tiny_skia::Paint::default();
            let alpha = if light { 16 } else { 14 };
            paint.set_color_rgba8(accent[0], accent[1], accent[2], alpha);
            if let Some(path) =
                rounded_path(cx - radius, cy - radius, radius * 2.0, radius * 2.0, radius)
            {
                pixmap.fill_path(
                    &path,
                    &paint,
                    resvg::tiny_skia::FillRule::Winding,
                    resvg::tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
    }

    if style.effects && style.background == WaterBackground::Sea {
        let factor = style.speed.factor();
        for band in 0..style.speed.wave_count() {
            let mut pb = resvg::tiny_skia::PathBuilder::new();
            let base_y = h * (0.54 + band as f32 * 0.08);
            let amplitude = h * 0.014 * (1.0 + band as f32 * 0.25);
            pb.move_to(0.0, base_y);
            let steps = 6;
            for step in 1..=steps {
                let x = w * step as f32 / steps as f32;
                let y = base_y + amplitude * ((step as f32 * 1.7 + band as f32) * factor).sin();
                let prev_x = w * (step - 1) as f32 / steps as f32;
                pb.quad_to((prev_x + x) / 2.0, base_y - amplitude, x, y);
            }
            let mut paint = resvg::tiny_skia::Paint::default();
            let alpha = (if light { 40 } else { 30 }) - band as u8 * 6;
            paint.set_color_rgba8(accent[0], accent[1], accent[2], alpha);
            paint.anti_alias = true;
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &resvg::tiny_skia::Stroke {
                        width: 1.4,
                        ..Default::default()
                    },
                    resvg::tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
    }

    Some(buffer(width, height, &straight(&pixmap)))
}

pub fn icon_svg(name: &str, color: &str) -> Option<String> {
    let body = match name {
        "search" => r#"<circle cx="8.5" cy="8.5" r="5.2"/><path d="m12.5 12.5 4 4"/>"#,
        "keyboard" => {
            r#"<rect x="2.2" y="5.2" width="15.6" height="9.6" rx="2"/><path d="M5 8h.01M8 8h.01M11 8h.01M14 8h.01M5 11h.01M8 11h.01M11 11h4"/>"#
        }
        "command" => {
            r#"<path d="M7 5.5a2.5 2.5 0 1 0-2.5 2.5H7V10M7 5.5A2.5 2.5 0 1 1 9.5 8H7M13 5.5A2.5 2.5 0 1 1 10.5 8H13V10M13 14.5a2.5 2.5 0 1 0 2.5-2.5H13V10M7 14.5A2.5 2.5 0 1 0 9.5 12H7"/>"#
        }
        "folder" => {
            r#"<path d="M2.8 5.8A2 2 0 0 1 4.8 4h3l2 2h6.4a2 2 0 0 1 2 2v6.2a2 2 0 0 1-2 2H4.8a2 2 0 0 1-2-2z"/>"#
        }
        "file" => r#"<path d="M5 2.8h6l4 4v10.4H5z"/><path d="M11 2.8v4h4"/>"#,
        "terminal" => {
            r#"<rect x="2.5" y="3.5" width="15" height="13" rx="2"/><path d="m5.5 8 2 2-2 2M10 12h4"/>"#
        }
        "chat" => r#"<path d="M3 4.5h14v9H9l-4 3v-3H3z"/>"#,
        "chart" => r#"<path d="M3 16.5h14M5.5 13V8M10 13V5M14.5 13v-3"/>"#,
        "grid" => {
            r#"<rect x="3" y="3" width="6" height="6" rx="1"/><rect x="11" y="3" width="6" height="6" rx="1"/><rect x="3" y="11" width="6" height="6" rx="1"/><rect x="11" y="11" width="6" height="6" rx="1"/>"#
        }
        "close" => r#"<path d="m5 5 10 10M15 5 5 15"/>"#,
        "chevron" => r#"<path d="m5 7.5 5 5 5-5"/>"#,
        "arrow" => r#"<path d="M3.5 10h12M11 5.5l4.5 4.5-4.5 4.5"/>"#,
        "cpu" => {
            r#"<rect x="4" y="4" width="12" height="12" rx="2"/><rect x="7" y="7" width="6" height="6" rx="1"/><path d="M7 1.5v2M13 1.5v2M7 16.5v2M13 16.5v2M1.5 7h2M1.5 13h2M16.5 7h2M16.5 13h2"/>"#
        }
        "memory" => {
            r#"<rect x="3" y="6" width="14" height="8" rx="2"/><path d="M6 14v2M10 14v2M14 14v2"/>"#
        }
        "mic" => {
            r#"<rect x="6" y="2.8" width="8" height="10.4" rx="4"/><path d="M3.5 9.2a6.5 6.5 0 0 0 13 0M10 15.7v2.5M7.2 18.2h5.6"/>"#
        }
        "audio" => {
            r#"<path d="M4 8h3l4-3v10l-4-3H4z"/><path d="M13.5 7.5a3.5 3.5 0 0 1 0 5M15.5 5.5a6.4 6.4 0 0 1 0 9"/>"#
        }
        "agents" => {
            r#"<circle cx="7" cy="7" r="2.5"/><circle cx="14" cy="8" r="2"/><path d="M2.5 16c.4-3 2.2-4.5 4.5-4.5S11 13 11.5 16M12 12.5c2.4 0 3.7 1.2 4 3.5"/>"#
        }
        "palette" => {
            r#"<path d="M10 2.8a7.2 7.2 0 1 0 0 14.4c1.3 0 1.6-1 .9-1.9-.8-1.1.2-2.5 1.6-2.5h2A2.7 2.7 0 0 0 17.2 10 7.2 7.2 0 0 0 10 2.8z"/><circle cx="6.5" cy="8" r=".9"/><circle cx="9" cy="5.8" r=".9"/><circle cx="12.5" cy="6.4" r=".9"/>"#
        }
        "explorer" => {
            r#"<rect x="2.8" y="3.5" width="14.4" height="13" rx="2"/><path d="M2.8 7.5h14.4M8 7.5v9"/>"#
        }
        "remote" => {
            r#"<rect x="2.5" y="3.5" width="15" height="10" rx="2"/><path d="M7 16.5h6M10 13.5v3"/><path d="m15 8 2 2-2 2"/>"#
        }
        _ => return None,
    };
    Some(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="{color}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_rasterize() {
        let markup = icon_svg("search", "#38d6ff").unwrap();
        assert!(svg(&markup, 20).is_some());
        assert!(svg(&markup, 16).is_some());
    }

    #[test]
    fn panels_and_gradients_rasterize() {
        assert!(panel(100, 60, 10.0, [7, 39, 51, 255], Some([127, 230, 255, 38])).is_some());
        let style = BackgroundStyle {
            mode: ThemeMode::Water,
            background: WaterBackground::Sea,
            tone: WaterTone::Ocean,
            caustics: true,
            effects: true,
            speed: WaterSpeed::Flowing,
        };
        assert!(background(64, 48, style).is_some());
        let mut light = style;
        light.mode = ThemeMode::Light;
        assert!(background(64, 48, light).is_some());
        assert!(rounded(40, 20, 8.0, [56, 214, 255, 60]).is_some());
    }
}
