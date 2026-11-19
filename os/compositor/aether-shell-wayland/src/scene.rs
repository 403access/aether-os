//! Machine-readable snapshot of the composed chrome.
//!
//! Setting `AETHER_SCENE_DUMP=/path/scene.json` makes every composed frame
//! write its layout truth — chrome rectangles, text runs, icons and panels —
//! as JSON. The parity tooling diffs that snapshot against the source DOM, so
//! missing labels and boxes are properties, not something a human has to spot
//! in a screenshot.

use std::io::Write;
use std::path::Path;

use serde_json::{json, Value};

use crate::app::text_style;
use crate::render::context::RenderChunk;

fn color(value: [u8; 4]) -> String {
    if value[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", value[0], value[1], value[2])
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            value[0], value[1], value[2], value[3]
        )
    }
}

fn round(value: f32) -> f64 {
    ((value as f64) * 100.0).round() / 100.0
}

/// Serialise one frame and replace `path` atomically. A reader can therefore
/// copy the file at any moment without seeing a half-written document.
pub fn write(path: &Path, width: u32, height: u32, chunk: &RenderChunk) {
    let chrome: Vec<Value> = chunk
        .chrome
        .iter()
        .map(|(rect, fill)| {
            json!({
                "x": rect.x,
                "y": rect.y,
                "w": rect.width,
                "h": rect.height,
                "color": color(fill.map(|value| (value * 255.0).round() as u8)),
            })
        })
        .collect();

    let labels: Vec<Value> = chunk
        .labels
        .iter()
        .map(|(text, x, y, scale, fill)| {
            let base = scale & 0xff;
            json!({
                "text": text,
                "x": x,
                "y": y,
                "px": round(text_style::text_px(base)),
                "weight": if text_style::text_bold(*scale) { "bold" } else { "regular" },
                "tracking": round(text_style::text_spacing(*scale)),
                "color": color(*fill),
            })
        })
        .collect();

    let icons: Vec<Value> = chunk
        .icons
        .iter()
        .map(|(name, x, y)| json!({"name": name, "x": x, "y": y, "size": 16}))
        .collect();

    let display_icons: Vec<Value> = chunk
        .display_icons
        .iter()
        .map(|(name, size, fill, x, y)| {
            json!({
                "name": name,
                "size": size,
                "x": x,
                "y": y,
                "color": color(*fill),
            })
        })
        .collect();

    // `MemoryRenderBuffer` does not expose its size, so panels carry their
    // origin only. Text labels and solid chrome rectangles still provide the
    // geometry the tree differ needs.
    let panels: Vec<Value> = chunk
        .panels
        .iter()
        .map(|(_, x, y)| json!({"x": x, "y": y}))
        .collect();

    let document = json!({
        "viewport": {"width": width, "height": height},
        "chrome": chrome,
        "labels": labels,
        "icons": icons,
        "display_icons": display_icons,
        "panels": panels,
    });

    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = Path::new(&tmp);
    let Ok(mut file) = std::fs::File::create(tmp) else {
        return;
    };
    if serde_json::to_writer(&mut file, &document).is_ok() {
        let _ = file.flush();
        let _ = std::fs::rename(tmp, path);
    } else {
        let _ = std::fs::remove_file(tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_geometry::Rect;

    #[test]
    fn writes_one_frame() {
        let mut chunk = RenderChunk::default();
        chunk
            .chrome
            .push((Rect::new(1, 2, 3, 4), [0.1, 0.2, 0.3, 0.4]));
        chunk
            .labels
            .push(("Hi".into(), 5, 6, 0x100 | 10, [255, 255, 255, 255]));
        let dir = std::env::temp_dir().join(format!("aether-scene-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scene.json");
        write(&path, 1280, 800, &chunk);
        let text = std::fs::read_to_string(&path).unwrap();
        let scene: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(scene["viewport"]["width"], 1280);
        assert_eq!(scene["labels"][0]["text"], "Hi");
        assert_eq!(scene["labels"][0]["px"], 12.54);
        assert_eq!(scene["labels"][0]["weight"], "bold");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
