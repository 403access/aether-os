//! Small formatting, icon and cursor helpers shared by the composer.

use smithay::input::pointer::CursorImageSurfaceData;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;

pub(super) fn cursor_hotspot(surface: &WlSurface) -> (i32, i32) {
    with_states(surface, |states| {
        states
            .data_map
            .get::<CursorImageSurfaceData>()
            .and_then(|data| data.lock().ok())
            .map(|attributes| (attributes.hotspot.x, attributes.hotspot.y))
            .unwrap_or((0, 0))
    })
}

pub(super) fn load_icon(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    if path.ends_with(".svg") {
        let data = std::fs::read_to_string(path).ok()?;
        let tree = resvg::usvg::Tree::from_str(&data, &resvg::usvg::Options::default()).ok()?;
        let size = tree.size();
        let scale = 64.0 / size.width().max(1.0);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(64, 64)?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        let mut pixels = Vec::with_capacity(64 * 64 * 4);
        for chunk in pixmap.data().as_chunks::<4>().0 {
            let alpha = chunk[3];
            let (red, green, blue) = if alpha == 0 {
                (0, 0, 0)
            } else {
                (
                    (u32::from(chunk[0]) * 255 / u32::from(alpha)) as u8,
                    (u32::from(chunk[1]) * 255 / u32::from(alpha)) as u8,
                    (u32::from(chunk[2]) * 255 / u32::from(alpha)) as u8,
                )
            };
            pixels.extend_from_slice(&[red, green, blue, alpha]);
        }
        return Some((64, 64, pixels));
    }
    let file = std::fs::File::open(path).ok()?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let data = &buffer[..info.buffer_size()];
    let pixels = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        _ => return None,
    };
    Some((info.width, info.height, pixels))
}

pub(super) fn format_rate(bytes_per_second: f64) -> String {
    if bytes_per_second >= 1_048_576.0 {
        format!("{:.1}M", bytes_per_second / 1_048_576.0)
    } else if bytes_per_second >= 1024.0 {
        format!("{:.0}K", bytes_per_second / 1024.0)
    } else {
        format!("{:.0}B", bytes_per_second)
    }
}

pub(super) fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

pub(super) fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut result: String = text.chars().take(limit.saturating_sub(1)).collect();
    result.push('…');
    result
}

pub(super) fn file_kind(extension: &str) -> (&'static str, [u8; 4]) {
    match extension {
        "PNG" | "JPG" | "JPEG" | "GIF" | "WEBP" | "SVG" => ("file", [209, 88, 79, 255]),
        "MP4" | "MKV" | "MOV" | "WEBM" => ("file", [154, 99, 214, 255]),
        "MP3" | "FLAC" | "WAV" | "OGG" => ("file", [209, 132, 58, 255]),
        "ZIP" | "TAR" | "GZ" | "XZ" | "7Z" => ("file", [162, 118, 47, 255]),
        "RS" | "TS" | "TSX" | "JS" | "PY" | "GO" | "C" | "H" | "NIX" => {
            ("file", [111, 213, 255, 255])
        }
        "MD" | "TXT" | "PDF" => ("file", [165, 219, 229, 255]),
        _ => ("file", [165, 219, 229, 255]),
    }
}

#[cfg(test)]
mod icon_tests {
    use super::load_icon;

    #[test]
    fn rasterizes_svg_icons() {
        let path = std::env::temp_dir().join(format!("aether-icon-{}.svg", std::process::id()));
        std::fs::write(
            &path,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><rect width=\"16\" height=\"16\" fill=\"#38d6ff\"/></svg>",
        )
        .unwrap();
        let (width, height, pixels) = load_icon(path.to_str().unwrap()).expect("svg icon");
        assert_eq!((width, height), (64, 64));
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        let _ = std::fs::remove_file(path);
    }
}
