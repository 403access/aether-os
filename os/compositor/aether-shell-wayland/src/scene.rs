//! Debug-only scene recorder.
//!
//! Compiled only with the `scene-dump` feature. Compositors sections record
//! semantic nodes (role, rect, text, style) with `scene_node!`/`scene_text!`;
//! `AETHER_SCENE_DUMP` writes the tree plus the raw render chunk as JSON.
//! Production builds contain neither the recorder nor the call sites.

use std::cell::RefCell;
use std::io::Write;
use std::path::Path;

use serde_json::{json, Value};

use crate::app::text_style;
use crate::render::context::RenderChunk;
use aether_geometry::Rect;

#[derive(Clone, Debug)]
pub(crate) struct SceneNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub role: &'static str,
    pub text: Option<String>,
    pub rect: Rect,
    pub background: Option<[u8; 4]>,
    pub color: Option<[u8; 4]>,
    pub radius: f32,
    pub px: Option<f32>,
    pub weight: Option<&'static str>,
    pub tracking: Option<f32>,
}

#[derive(Default)]
struct SceneTree {
    nodes: Vec<SceneNode>,
}

thread_local! {
    static TREE: RefCell<SceneTree> = RefCell::new(SceneTree::default());
}

pub(crate) fn reset() {
    TREE.with(|tree| {
        let mut tree = tree.borrow_mut();
        tree.nodes.clear();
    });
}

fn push_node(node: SceneNode) -> usize {
    TREE.with(|tree| {
        let mut tree = tree.borrow_mut();
        let id = tree.nodes.len();
        tree.nodes.push(SceneNode { id, ..node });
        id
    })
}

/// Add a structural node and return its id for child calls.
pub(crate) fn push(
    parent: Option<usize>,
    role: &'static str,
    text: Option<&str>,
    rect: Rect,
    background: Option<[u8; 4]>,
    radius: f32,
) -> usize {
    push_node(SceneNode {
        id: 0,
        parent,
        role,
        text: text.map(str::to_string),
        rect,
        background,
        color: None,
        radius,
        px: None,
        weight: None,
        tracking: None,
    })
}

/// Add a text node; scale is the renderer's chrome text scale.
pub(crate) fn push_text(
    parent: Option<usize>,
    role: &'static str,
    text: &str,
    rect: Rect,
    scale: u32,
    color: [u8; 4],
) -> usize {
    let base = scale & 0xff;
    push_node(SceneNode {
        id: 0,
        parent,
        role,
        text: Some(text.to_string()),
        rect,
        background: None,
        color: Some(color),
        radius: 0.0,
        px: Some(text_style::text_px(base)),
        weight: Some(text_style::text_weight_label(scale)),
        tracking: Some(text_style::text_spacing(scale)),
    })
}

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

fn rect_json(rect: Rect) -> Value {
    json!({"x": rect.x, "y": rect.y, "w": rect.width, "h": rect.height})
}

/// Serialise the node tree and the raw render chunk, replacing `path`
/// atomically.
pub fn write(path: &Path, width: u32, height: u32, chunk: &RenderChunk) {
    let nodes: Vec<Value> = TREE.with(|tree| {
        tree.borrow()
            .nodes
            .iter()
            .map(|node| {
                json!({
                    "id": node.id,
                    "parent": node.parent,
                    "role": node.role,
                    "text": node.text,
                    "rect": rect_json(node.rect),
                    "background": node.background.map(color),
                    "color": node.color.map(color),
                    "radius": round(node.radius),
                    "px": node.px.map(round),
                    "weight": node.weight,
                    "tracking": node.tracking.map(round),
                })
            })
            .collect()
    });

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
                "weight": text_style::text_weight_label(*scale),
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
    // origin only. Scene nodes carry the semantic boxes.
    let panels: Vec<Value> = chunk
        .panels
        .iter()
        .map(|(_, x, y)| json!({"x": x, "y": y}))
        .collect();

    let document = json!({
        "viewport": {"width": width, "height": height},
        "nodes": nodes,
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

    #[test]
    fn records_a_tree() {
        reset();
        let root = push(None, "window", None, Rect::new(1, 2, 3, 4), None, 0.0);
        push_text(
            Some(root),
            "title",
            "Hi",
            Rect::new(5, 6, 7, 8),
            0x100 | 10,
            [255; 4],
        );
        let mut chunk = RenderChunk::default();
        chunk
            .chrome
            .push((Rect::new(1, 2, 3, 4), [0.1, 0.2, 0.3, 0.4]));
        let dir = std::env::temp_dir().join(format!("aether-scene-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scene.json");
        write(&path, 1280, 800, &chunk);
        let scene: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(scene["nodes"][0]["role"], "window");
        assert_eq!(scene["nodes"][1]["text"], "Hi");
        assert_eq!(scene["nodes"][1]["px"], 12.54);
        assert_eq!(scene["nodes"][1]["weight"], "bold");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
