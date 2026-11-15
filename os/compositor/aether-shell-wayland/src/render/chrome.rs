//! Cached chrome buffers: icons, panels, rounded fills and backdrops.

use std::sync::Arc;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;

impl App {
    pub(crate) fn chrome_icon(
        &mut self,
        name: &str,
        size: u32,
        color: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (name.to_string(), size, color);
        if let Some(buffer) = self.chrome_icons.get(&key) {
            return Some(buffer.clone());
        }
        let hex = format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]);
        let markup = crate::chrome::icon_svg(name, &hex)?;
        let buffer = Arc::new(crate::chrome::svg(&markup, size)?);
        self.chrome_icons.insert(key, buffer.clone());
        Some(buffer)
    }

    pub(crate) fn chrome_panel(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        fill: [u8; 4],
        border: Option<[u8; 4]>,
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, fill, border);
        if let Some(buffer) = self.chrome_panels.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(crate::chrome::panel(
            width,
            height,
            radius as f32,
            fill,
            border,
        )?);
        self.chrome_panels.insert(key, buffer.clone());
        Some(buffer)
    }

    pub(crate) fn chrome_round_top(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        fill: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, fill);
        if let Some(buffer) = self.chrome_tops.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(crate::chrome::rounded_top(
            width,
            height,
            radius as f32,
            fill,
        )?);
        self.chrome_tops.insert(key, buffer.clone());
        Some(buffer)
    }

    pub(crate) fn chrome_round(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        fill: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, fill);
        if let Some(buffer) = self.chrome_rounds.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(crate::chrome::rounded(width, height, radius as f32, fill)?);
        self.chrome_rounds.insert(key, buffer.clone());
        Some(buffer)
    }

    pub(crate) fn chrome_background(
        &mut self,
        width: u32,
        height: u32,
        style: crate::chrome::BackgroundStyle,
    ) -> Option<Arc<MemoryRenderBuffer>> {
        if let Some(buffer) = self.chrome_background.get(&(width, height, style)) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(crate::chrome::background(width, height, style)?);
        self.chrome_background
            .insert((width, height, style), buffer.clone());
        Some(buffer)
    }
}
