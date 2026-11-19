//! Cached chrome buffers: icons, panels, rounded fills and backdrops.
//!
//! Every cache here is byte-bounded (`BudgetCache`). These buffers are chrome
//! that a user can generate indefinitely — a status pill per second, a panel
//! per chat bubble height, an icon per theme colour — so an unbounded map would
//! turn ordinary use into permanent resident memory.

use std::sync::Arc;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;

pub(crate) mod primitives;

pub(crate) use primitives::BackgroundStyle;

/// Approximate per-entry overhead: the key tuple, the hash-map slot and the
/// allocator header. The buffer itself dominates; this keeps many tiny entries
/// from being treated as free.
const KEY_BYTES: usize = 64;

/// CPU-side weight of an RGBA buffer.
fn rgba_bytes(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
        .saturating_add(KEY_BYTES)
}

pub(crate) struct WindowEffects {
    pub shadow: Option<(Arc<MemoryRenderBuffer>, i32, i32)>,
    pub wave: Option<(Arc<MemoryRenderBuffer>, i32, i32)>,
}

impl App {
    pub(crate) fn wave_phase_step(&self) -> u32 {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0);
        ((millis % 8000) as f32 / 8000.0 * 24.0).round() as u32 % 24
    }

    /// Drop shadow and (for focused windows with effects on) the tide rim.
    pub(crate) fn window_effects(
        &mut self,
        rect: aether_geometry::Rect,
        radius: u32,
        focused: bool,
        hover: bool,
    ) -> WindowEffects {
        let shadow = self
            .chrome_shadow(rect.width, rect.height, radius, hover)
            .map(|buffer| {
                (
                    buffer,
                    rect.x - primitives::SHADOW_PAD,
                    rect.y - primitives::SHADOW_PAD,
                )
            });
        let wave = if focused && self.shell.theme_settings().effects {
            let phase = self.wave_phase_step();
            let opacity = if hover { 255 } else { 217 };
            self.chrome_wave(rect.width, rect.height, radius, phase, opacity)
                .map(|buffer| (buffer, rect.x - 6, rect.y - 6))
        } else {
            None
        };
        WindowEffects { shadow, wave }
    }

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
        let markup = primitives::icon_svg(name, &hex)?;
        let buffer = Arc::new(primitives::svg(&markup, size)?);
        self.chrome_icons
            .insert(key, buffer.clone(), rgba_bytes(size, size));
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
        let buffer = Arc::new(primitives::panel(
            width,
            height,
            radius as f32,
            fill,
            border,
        )?);
        self.chrome_panels
            .insert(key, buffer.clone(), rgba_bytes(width, height));
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
        let buffer = Arc::new(primitives::rounded_top(width, height, radius as f32, fill)?);
        self.chrome_tops
            .insert(key, buffer.clone(), rgba_bytes(width, height));
        Some(buffer)
    }

    pub(crate) fn chrome_shadow(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        strong: bool,
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, strong);
        if let Some(buffer) = self.chrome_shadows.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(primitives::shadow(width, height, radius as f32, strong)?);
        self.chrome_shadows
            .insert(key, buffer.clone(), rgba_bytes(width + 56, height + 56));
        Some(buffer)
    }

    pub(crate) fn chrome_wave(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        phase_step: u32,
        opacity: u8,
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, phase_step, opacity);
        if let Some(buffer) = self.chrome_waves.get(&key) {
            return Some(buffer.clone());
        }
        let phase = phase_step as f32 / 24.0 * std::f32::consts::TAU;
        let buffer = Arc::new(primitives::wave_rim(
            width,
            height,
            radius as f32,
            phase,
            opacity as f32 / 255.0,
        )?);
        self.chrome_waves
            .insert(key, buffer.clone(), rgba_bytes(width + 12, height + 12));
        Some(buffer)
    }

    pub(crate) fn chrome_round_gradient(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        top: [u8; 4],
        bottom: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, top, bottom);
        if let Some(buffer) = self.chrome_gradients.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(primitives::rounded_gradient(
            width,
            height,
            radius as f32,
            top,
            bottom,
        )?);
        self.chrome_gradients
            .insert(key, buffer.clone(), rgba_bytes(width, height));
        Some(buffer)
    }

    pub(crate) fn chrome_outline(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        border: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, border);
        if let Some(buffer) = self.chrome_outlines.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(primitives::outline(width, height, radius as f32, border)?);
        self.chrome_outlines
            .insert(key, buffer.clone(), rgba_bytes(width, height));
        Some(buffer)
    }

    pub(crate) fn chrome_round_bottom(
        &mut self,
        width: u32,
        height: u32,
        radius: u32,
        fill: [u8; 4],
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height, radius, fill);
        if let Some(buffer) = self.chrome_bottoms.get(&key) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(primitives::rounded_bottom(
            width,
            height,
            radius as f32,
            fill,
        )?);
        self.chrome_bottoms
            .insert(key, buffer.clone(), rgba_bytes(width, height));
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
        let buffer = Arc::new(primitives::rounded(width, height, radius as f32, fill)?);
        self.chrome_rounds
            .insert(key, buffer.clone(), rgba_bytes(width, height));
        Some(buffer)
    }

    /// One background per output size. A style change replaces the buffer for
    /// that size instead of retaining every visited theme; the byte budget
    /// evicts stale sizes left behind by window resizes.
    pub(crate) fn chrome_background(
        &mut self,
        width: u32,
        height: u32,
        style: BackgroundStyle,
    ) -> Option<Arc<MemoryRenderBuffer>> {
        let key = (width, height);
        if let Some((cached, buffer)) = self.chrome_background.get(&key) {
            if *cached == style {
                return Some(buffer.clone());
            }
        }
        let buffer = Arc::new(primitives::background(width, height, style)?);
        self.chrome_background
            .insert(key, (style, buffer.clone()), rgba_bytes(width, height));
        Some(buffer)
    }
}
