//! Small composition kit for compositor-owned app windows and trays.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

pub(crate) type Label = (String, i32, i32, u32, [u8; 4]);
pub(crate) type Icon = (String, u32, [u8; 4], i32, i32);
pub(crate) type Panel = (Arc<MemoryRenderBuffer>, i32, i32);
pub(crate) type Solid = (AetherRect, [f32; 4]);

#[derive(Default)]
pub(crate) struct Ui {
    pub labels: Vec<Label>,
    pub icons: Vec<Icon>,
    pub panels: Vec<Panel>,
    pub chrome: Vec<Solid>,
}

impl Ui {
    pub fn label(&mut self, text: impl Into<String>, x: i32, y: i32, scale: u32, color: [u8; 4]) {
        self.labels.push((text.into(), x, y, scale, color));
    }

    pub fn icon(&mut self, name: &str, size: u32, color: [u8; 4], x: i32, y: i32) {
        self.icons.push((name.to_string(), size, color, x, y));
    }

    pub fn panel(&mut self, buffer: Option<Arc<MemoryRenderBuffer>>, x: i32, y: i32) {
        if let Some(buffer) = buffer {
            self.panels.push((buffer, x, y));
        }
    }

    pub fn solid(&mut self, rect: AetherRect, color: [f32; 4]) {
        self.chrome.push((rect, color));
    }

    pub fn effect(&mut self, effect: Option<(Arc<MemoryRenderBuffer>, i32, i32)>) {
        if let Some((buffer, x, y)) = effect {
            self.panels.push((buffer, x, y));
        }
    }
}
