//! Shared frame state for the composition sections.

use std::collections::HashMap;
use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use aether_shell::theme::{ThemeSettings, ThemeTokens};
use aether_shell::{OutputId, WindowId};
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

/// Which surfaces are being drawn for this output.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SurfaceFlags {
    pub palette: bool,
    pub explorer: bool,
    pub chat: bool,
    pub audio: bool,
    pub remote: bool,
    pub shortcuts: bool,
    pub task: bool,
    pub themes: bool,
}

/// Values computed once per output before the sections run.
pub(crate) struct FrameContext {
    pub output: Option<OutputId>,
    pub primary: OutputId,
    pub chrome_output: OutputId,
    pub offset: (i32, i32),
    pub width: u32,
    pub height: u32,
    pub visible: Vec<WindowId>,
    pub rects: HashMap<WindowId, AetherRect>,
    pub tokens: ThemeTokens,
    pub settings: ThemeSettings,
    pub panel_fill: [u8; 4],
    pub panel_fill_soft: [u8; 4],
    pub panel_border: Option<[u8; 4]>,
    pub panel_border_soft: Option<[u8; 4]>,
    pub accent_f: [f32; 4],
    pub flags: SurfaceFlags,
}

/// Accumulated chrome produced by one composition section.
#[derive(Default)]
pub(crate) struct RenderChunk {
    pub chrome: Vec<(AetherRect, [f32; 4])>,
    pub labels: Vec<(String, i32, i32, u32, [u8; 4])>,
    pub icons: Vec<(String, i32, i32)>,
    pub panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)>,
    pub display_icons: Vec<(String, u32, [u8; 4], i32, i32)>,
}

impl RenderChunk {
    pub(crate) fn merge(&mut self, mut other: RenderChunk) {
        self.chrome.append(&mut other.chrome);
        self.labels.append(&mut other.labels);
        self.icons.append(&mut other.icons);
        self.panels.append(&mut other.panels);
        self.display_icons.append(&mut other.display_icons);
    }
}
