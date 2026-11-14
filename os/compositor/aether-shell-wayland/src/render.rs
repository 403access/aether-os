//! Render element composition shared by every Aether backend.
//!
//! `App::compose` builds a single front-to-back element list covering client
//! surfaces, compositor chrome, palette icons/text, IME popups and cursors.
//! Winit draws the list into its GL framebuffer; the DRM backend hands the
//! same list to `DrmCompositor` for scan-out.

use std::collections::HashMap;
use std::sync::Arc;

use aether_cursor::render as render_cursor;
use aether_geometry::Rect as AetherRect;
use aether_shell::{
    OutputId, PaletteAction, WindowId, PALETTE_CONTENT_TOP, PALETTE_ROW_HEIGHT,
    PALETTE_ROW_SPACING, PALETTE_VISIBLE_ROWS, WORKSPACE_INDICATOR_BAR_HEIGHT,
};
use aether_text::TextBitmap;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::Color32F;
use smithay::input::pointer::CursorImageSurfaceData;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{
    Buffer as BufferCoords, IsAlive, Physical, Point, Rectangle, Scale, Size, Transform,
};
use smithay::wayland::compositor::with_states;

use crate::app::{App, CursorCacheEntry};

/// Default Aether background colour.
pub const BACKGROUND: Color32F = Color32F::new(0.011_764_7, 0.098_039_2, 0.133_333_3, 1.0);

fn design_color(hex: u32, alpha: f32) -> [f32; 4] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        alpha,
    ]
}

/// One composited element. `DrmCompositor` and the winit draw loop both need a
/// single concrete element type, so surface, solid and memory elements are
/// wrapped in this enum.
pub enum AetherRenderElement {
    Surface(WaylandSurfaceRenderElement<GlesRenderer>),
    Solid(SolidColorRenderElement),
    Memory(MemoryRenderBufferRenderElement<GlesRenderer>),
}

impl From<WaylandSurfaceRenderElement<GlesRenderer>> for AetherRenderElement {
    fn from(element: WaylandSurfaceRenderElement<GlesRenderer>) -> Self {
        Self::Surface(element)
    }
}

impl From<SolidColorRenderElement> for AetherRenderElement {
    fn from(element: SolidColorRenderElement) -> Self {
        Self::Solid(element)
    }
}

impl From<MemoryRenderBufferRenderElement<GlesRenderer>> for AetherRenderElement {
    fn from(element: MemoryRenderBufferRenderElement<GlesRenderer>) -> Self {
        Self::Memory(element)
    }
}

impl Element for AetherRenderElement {
    fn id(&self) -> &Id {
        match self {
            Self::Surface(element) => element.id(),
            Self::Solid(element) => element.id(),
            Self::Memory(element) => element.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            Self::Surface(element) => element.current_commit(),
            Self::Solid(element) => element.current_commit(),
            Self::Memory(element) => element.current_commit(),
        }
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        match self {
            Self::Surface(element) => element.location(scale),
            Self::Solid(element) => element.location(scale),
            Self::Memory(element) => element.location(scale),
        }
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        match self {
            Self::Surface(element) => element.src(),
            Self::Solid(element) => element.src(),
            Self::Memory(element) => element.src(),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            Self::Surface(element) => element.transform(),
            Self::Solid(element) => element.transform(),
            Self::Memory(element) => element.transform(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            Self::Surface(element) => element.geometry(scale),
            Self::Solid(element) => element.geometry(scale),
            Self::Memory(element) => element.geometry(scale),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        match self {
            Self::Surface(element) => Element::damage_since(element, scale, commit),
            Self::Solid(element) => Element::damage_since(element, scale, commit),
            Self::Memory(element) => Element::damage_since(element, scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        match self {
            Self::Surface(element) => Element::opaque_regions(element, scale),
            Self::Solid(element) => Element::opaque_regions(element, scale),
            Self::Memory(element) => Element::opaque_regions(element, scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            Self::Surface(element) => element.alpha(),
            Self::Solid(element) => element.alpha(),
            Self::Memory(element) => element.alpha(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Surface(element) => element.kind(),
            Self::Solid(element) => element.kind(),
            Self::Memory(element) => element.kind(),
        }
    }
}

impl RenderElement<GlesRenderer> for AetherRenderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        match self {
            Self::Surface(element) => {
                <WaylandSurfaceRenderElement<GlesRenderer> as RenderElement<GlesRenderer>>::draw(
                    element,
                    frame,
                    src,
                    dst,
                    damage,
                    opaque_regions,
                )
            }
            Self::Solid(element) => <SolidColorRenderElement as RenderElement<GlesRenderer>>::draw(
                element,
                frame,
                src,
                dst,
                damage,
                opaque_regions,
            ),
            Self::Memory(element) => {
                <MemoryRenderBufferRenderElement<GlesRenderer> as RenderElement<GlesRenderer>>::draw(
                    element,
                    frame,
                    src,
                    dst,
                    damage,
                    opaque_regions,
                )
            }
        }
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        match self {
            Self::Surface(element) => element.underlying_storage(renderer),
            Self::Solid(element) => element.underlying_storage(renderer),
            Self::Memory(element) => element.underlying_storage(renderer),
        }
    }
}

/// A composed frame in topmost-first order, matching `DrmCompositor` and
/// `draw_render_elements` expectations.
pub struct ComposedFrame {
    pub elements: Vec<AetherRenderElement>,
    pub clear: Color32F,
    /// Damage in this output's local physical coordinates.
    pub damage: Vec<Rectangle<i32, Physical>>,
}

impl App {
    fn chrome_icon(
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

    fn chrome_panel(
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

    fn chrome_round_top(
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

    fn chrome_round(
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

    fn chrome_background(&mut self, width: u32, height: u32) -> Option<Arc<MemoryRenderBuffer>> {
        if let Some(buffer) = self.chrome_background.get(&(width, height)) {
            return Some(buffer.clone());
        }
        let buffer = Arc::new(crate::chrome::background(width, height)?);
        self.chrome_background
            .insert((width, height), buffer.clone());
        Some(buffer)
    }

    /// Compose one output (or the single logical output when `None`).
    pub fn compose(
        &mut self,
        renderer: &mut GlesRenderer,
        output: Option<OutputId>,
    ) -> ComposedFrame {
        self.poll_status();
        let mut elements: Vec<AetherRenderElement> = Vec::new();
        // DRM compositors are output-sized; translate extended-desktop
        // coordinates into the output's local space. Winit (None) keeps the
        // global origin because its single output starts at (0, 0).
        let offset = output
            .map(|id| {
                let rect = self.shell.output_bounds(id);
                (rect.x, rect.y)
            })
            .unwrap_or((0, 0));

        let (width, height) = output
            .map(|id| {
                let bounds = self.shell.output_bounds(id);
                (bounds.width.max(1), bounds.height.max(1))
            })
            .unwrap_or_else(|| {
                let viewport = self.shell.viewport();
                (viewport.width.max(1), viewport.height.max(1))
            });
        if let Some(background) = self.chrome_background(width, height) {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (0.0, 0.0),
                &background,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        let visible: Vec<WindowId> = match output {
            Some(output) => self.shell.visible_windows_on(output),
            None => self
                .shell
                .window_stack()
                .into_iter()
                .filter(|id| {
                    self.shell
                        .windows()
                        .into_iter()
                        .find(|window| window.id == *id)
                        .is_some_and(|window| window.workspace == self.shell.active_workspace())
                })
                .collect(),
        };
        let rects: HashMap<WindowId, AetherRect> = self
            .shell
            .windows()
            .into_iter()
            .map(|window| (window.id, window.rect))
            .collect();

        // Client surfaces, bottom-most first.
        for id in visible.iter().rev() {
            let Some(rect) = rects.get(id) else {
                continue;
            };
            let Some(toplevel) = self.windows.get(id) else {
                continue;
            };
            elements.extend(
                render_elements_from_surface_tree::<
                    GlesRenderer,
                    WaylandSurfaceRenderElement<GlesRenderer>,
                >(
                    renderer,
                    toplevel.wl_surface(),
                    (rect.x - offset.0, rect.y - offset.1),
                    1.0,
                    1.0,
                    Kind::Unspecified,
                )
                .into_iter()
                .map(AetherRenderElement::from),
            );
        }

        let primary = self
            .shell
            .primary_output()
            .map(|output| output.id)
            .unwrap_or(1);
        let chrome_output = output.unwrap_or(primary);
        let palette_here = !self.shell.explorer_open()
            && (output.is_none()
                || self.shell.overlay_output() == Some(chrome_output)
                || self.shell.overlay_output().is_none());
        let explorer_here =
            self.shell.explorer_open() && (output.is_none() || output == Some(primary));
        let chat_here = self.shell.chat_open() && (output.is_none() || output == Some(primary));
        let audio_here = self.shell.audio_open() && (output.is_none() || output == Some(primary));
        let shortcuts_here = self.shortcuts_open && (output.is_none() || output == Some(primary));
        let task_here = self.task_manager_open() && (output.is_none() || output == Some(primary));

        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut icons: Vec<(String, i32, i32)> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        {
            if palette_here {
                if let Some(rect) = self.shell.overlay_rect() {
                    let width = rect.width as i32;
                    let height = rect.height as i32;
                    if let Some(buffer) = self.chrome_panel(
                        rect.width,
                        rect.height,
                        14,
                        [7, 39, 51, 250],
                        Some([127, 230, 255, 34]),
                    ) {
                        panels.push((buffer, rect.x - 18, rect.y - 18));
                    }
                    // Search header.
                    if let Some(buffer) = self.chrome_round_top(
                        rect.width.saturating_sub(2),
                        58.min(rect.height),
                        13,
                        [9, 49, 63, 255],
                    ) {
                        panels.push((buffer, rect.x + 1, rect.y + 1));
                    }
                    display_icons.push((
                        "search".into(),
                        18,
                        [56, 214, 255, 255],
                        rect.x + 18,
                        rect.y + 20,
                    ));
                    let query = self.shell.palette_query();
                    if query.is_empty() {
                        labels.push((
                            "Search workspaces, windows, files…".into(),
                            rect.x + 46,
                            rect.y + 18,
                            1,
                            [107, 169, 182, 255],
                        ));
                    } else {
                        labels.push((
                            format!("{query}_"),
                            rect.x + 46,
                            rect.y + 18,
                            1,
                            [233, 251, 255, 255],
                        ));
                    }
                    let esc = AetherRect::new(rect.x + width - 54, rect.y + 17, 36, 22);
                    if let Some(buffer) = self.chrome_round(36, 22, 6, [3, 25, 34, 255]) {
                        panels.push((buffer, esc.x, esc.y));
                    }
                    labels.push(("esc".into(), esc.x + 10, esc.y + 5, 0, [107, 169, 182, 255]));
                    chrome.push((
                        AetherRect::new(rect.x + 1, rect.y + 58, rect.width.saturating_sub(2), 1),
                        [127, 230, 255, 22].map(|value| value as f32 / 255.0),
                    ));

                    let entries = self.shell.palette_entries();
                    let selected = self.shell.palette_selected();
                    if entries.is_empty() {
                        labels.push((
                            "No matching commands".into(),
                            rect.x + width / 2 - 66,
                            rect.y + 96,
                            1,
                            [165, 219, 229, 255],
                        ));
                        labels.push((
                            "Try a different query, or a prefix such as > or /".into(),
                            rect.x + width / 2 - 148,
                            rect.y + 118,
                            0,
                            [107, 169, 182, 255],
                        ));
                    }
                    let start = self.shell.palette_visible_start();
                    for (row, entry) in entries
                        .iter()
                        .skip(start)
                        .take(PALETTE_VISIBLE_ROWS)
                        .enumerate()
                    {
                        let index = start + row;
                        let row_y =
                            rect.y + PALETTE_CONTENT_TOP + row as i32 * PALETTE_ROW_SPACING as i32;
                        let row_rect = AetherRect::new(
                            rect.x + 10,
                            row_y,
                            rect.width.saturating_sub(20),
                            PALETTE_ROW_HEIGHT,
                        );
                        let active = index == selected;
                        if active {
                            if let Some(buffer) = self.chrome_round(
                                row_rect.width,
                                row_rect.height,
                                8,
                                [56, 214, 255, 46],
                            ) {
                                panels.push((buffer, row_rect.x, row_rect.y));
                            }
                            if let Some(buffer) = self.chrome_round(
                                3,
                                row_rect.height.saturating_sub(16),
                                2,
                                [56, 214, 255, 255],
                            ) {
                                panels.push((buffer, row_rect.x, row_rect.y + 8));
                            }
                        }
                        // Icon tile.
                        let tile = AetherRect::new(row_rect.x + 10, row_rect.y + 7, 26, 26);
                        if let Some(buffer) = self.chrome_round(26, 26, 7, [3, 25, 34, 210]) {
                            panels.push((buffer, tile.x, tile.y));
                        }
                        let icon = match entry.action {
                            PaletteAction::LaunchApp(index) => self
                                .shell
                                .launcher(index)
                                .and_then(|launcher| launcher.icon.clone()),
                            _ => None,
                        };
                        let tag = match entry.action {
                            PaletteAction::SwitchWorkspace(_) => "grid",
                            PaletteAction::FocusWindow(_) => "explorer",
                            PaletteAction::LaunchApp(_) => "terminal",
                            PaletteAction::SpawnTerminal => "terminal",
                            PaletteAction::OpenTaskManager => "chart",
                            PaletteAction::OpenFile(_) => "file",
                            PaletteAction::Close => "close",
                        };
                        if let Some(path) = icon {
                            icons.push((path, tile.x + 5, tile.y + 5));
                        } else {
                            display_icons.push((
                                tag.into(),
                                16,
                                if active {
                                    [56, 214, 255, 255]
                                } else {
                                    [165, 219, 229, 255]
                                },
                                tile.x + 5,
                                tile.y + 5,
                            ));
                        }
                        labels.push((
                            entry.title.clone(),
                            row_rect.x + 48,
                            row_rect.y + 11,
                            1,
                            if active {
                                [233, 251, 255, 255]
                            } else {
                                [217, 247, 255, 255]
                            },
                        ));
                        labels.push((
                            entry.detail.clone(),
                            row_rect.x + row_rect.width as i32
                                - self.text_width(&entry.detail, 0) as i32
                                - 12,
                            row_rect.y + 12,
                            0,
                            [107, 169, 182, 255],
                        ));
                    }

                    // Footer with keycaps.
                    let footer_y = rect.y + height - 34;
                    let mut caps: Vec<(AetherRect, String, bool)> = Vec::new();
                    let mut cx = rect.x + 14;
                    for (text, label) in [("↑↓", "navigate"), ("↵", "run"), ("esc", "close")]
                    {
                        let key_width = (text.chars().count() as u32 * 7 + 14).max(22);
                        caps.push((
                            AetherRect::new(cx, footer_y + 6, key_width, 20),
                            text.to_string(),
                            true,
                        ));
                        let label_x = cx + key_width as i32 + 5;
                        labels.push((label.into(), label_x, footer_y + 8, 0, [107, 169, 182, 255]));
                        cx = label_x + self.text_width(label, 0) as i32 + 16;
                    }
                    for (key_rect, text, is_key) in caps {
                        if is_key {
                            if let Some(buffer) = self.chrome_round(
                                key_rect.width,
                                key_rect.height,
                                5,
                                [3, 25, 34, 255],
                            ) {
                                panels.push((buffer, key_rect.x, key_rect.y));
                            }
                            labels.push((
                                text.clone(),
                                key_rect.x
                                    + (key_rect.width as i32 - self.text_width(&text, 0) as i32)
                                        / 2,
                                key_rect.y + 5,
                                0,
                                [165, 219, 229, 255],
                            ));
                        }
                    }
                    labels.push((
                        "> commands   @ agents   / files".into(),
                        rect.x + width - 210,
                        footer_y + 8,
                        0,
                        [107, 169, 182, 255],
                    ));
                }
            }

            if chat_here {
                let rect = self.shell.chat_rect();
                let width = rect.width as i32;
                let height = rect.height as i32;
                if let Some(buffer) = self.chrome_panel(
                    rect.width,
                    rect.height,
                    14,
                    [7, 39, 51, 244],
                    Some([127, 230, 255, 30]),
                ) {
                    panels.push((buffer, rect.x - 18, rect.y - 18));
                }
                // Header band.
                if let Some(buffer) =
                    self.chrome_round_top(rect.width.saturating_sub(2), 58, 13, [9, 49, 63, 255])
                {
                    panels.push((buffer, rect.x + 1, rect.y + 1));
                }
                if let Some(buffer) = self.chrome_round(28, 28, 8, [56, 214, 255, 42]) {
                    panels.push((buffer, rect.x + 14, rect.y + 15));
                }
                display_icons.push((
                    "chat".into(),
                    17,
                    [56, 214, 255, 255],
                    rect.x + 20,
                    rect.y + 21,
                ));
                labels.push((
                    "Aether".into(),
                    rect.x + 52,
                    rect.y + 13,
                    2,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    "Streaming markdown".into(),
                    rect.x + 52,
                    rect.y + 35,
                    0,
                    [107, 169, 182, 255],
                ));
                // Live status pill.
                let pill_w = 86;
                let pill =
                    AetherRect::new(rect.x + width - pill_w - 14, rect.y + 18, pill_w as u32, 22);
                if let Some(buffer) = self.chrome_round(pill_w as u32, 22, 11, [67, 214, 138, 28]) {
                    panels.push((buffer, pill.x, pill.y));
                }
                if let Some(buffer) = self.chrome_round(6, 6, 3, [67, 214, 138, 255]) {
                    panels.push((buffer, pill.x + 10, pill.y + 8));
                }
                labels.push((
                    "live".into(),
                    pill.x + 22,
                    pill.y + 5,
                    0,
                    [126, 235, 178, 255],
                ));
                display_icons.push((
                    "close".into(),
                    14,
                    [107, 169, 182, 255],
                    rect.x + width - 30,
                    rect.y + 22,
                ));

                // Assistant message card.
                let bubble = AetherRect::new(
                    rect.x + 14,
                    rect.y + 70,
                    rect.width.saturating_sub(28),
                    rect.height.saturating_sub(70 + 76),
                );
                if let Some(buffer) =
                    self.chrome_round(bubble.width, bubble.height, 10, [5, 37, 48, 255])
                {
                    panels.push((buffer, bubble.x, bubble.y));
                }
                if let Some(buffer) = self.chrome_round(3, 18, 2, [56, 214, 255, 160]) {
                    panels.push((buffer, bubble.x + 1, bubble.y + 16));
                }
                let mut y = bubble.y + 14;
                let bottom = bubble.y + bubble.height as i32 - 12;
                let text_left = bubble.x + 18;
                let max_width = bubble.width as i32 - 36;
                let chat_lines: Vec<aether_shell::ChatLine> =
                    self.shell.visible_chat_lines().to_vec();
                for line in &chat_lines {
                    let row_height = if line.scale >= 2 {
                        26
                    } else if line.scale == 0 {
                        14
                    } else {
                        18
                    };
                    if y + row_height > bottom {
                        break;
                    }
                    if line.code {
                        if let Some(buffer) = self.chrome_round(
                            bubble.width.saturating_sub(20),
                            row_height as u32,
                            5,
                            [3, 25, 34, 210],
                        ) {
                            panels.push((buffer, bubble.x + 10, y - 2));
                        }
                    }
                    let mut x = text_left + line.indent;
                    for span in &line.spans {
                        let color = match span.kind {
                            aether_shell::ChatSpanKind::Text => [220, 240, 245, 255],
                            aether_shell::ChatSpanKind::Bold => [255, 255, 255, 255],
                            aether_shell::ChatSpanKind::Code => [120, 230, 180, 255],
                        };
                        let mut scale = if line.code { 1 } else { line.scale };
                        if span.kind == aether_shell::ChatSpanKind::Bold {
                            scale |= 0x100;
                        }
                        let span_width = self.text_width(&span.text, scale & 0xff) as i32;
                        if x + span_width > text_left + max_width {
                            break;
                        }
                        labels.push((span.text.clone(), x, y, scale, color));
                        x += span_width;
                    }
                    y += row_height;
                }
                let track = AetherRect::new(
                    rect.x + width - 10,
                    bubble.y + 10,
                    4,
                    bubble.height.saturating_sub(20),
                );
                if let Some(buffer) = self.chrome_round(4, track.height, 2, [3, 25, 34, 255]) {
                    panels.push((buffer, track.x, track.y));
                }
                let lines = self.shell.chat_line_count().max(1);
                let visible = self.shell.chat_visible_rows().max(1);
                let thumb = ((visible as f32 / lines as f32) * track.height as f32)
                    .clamp(12.0, track.height as f32) as u32;
                let offset =
                    ((self.shell.chat_scroll() as f32 / lines as f32) * track.height as f32) as i32;
                if let Some(buffer) = self.chrome_round(4, thumb, 2, [56, 214, 255, 190]) {
                    panels.push((
                        buffer,
                        track.x,
                        track.y + offset.min(track.height as i32 - thumb as i32).max(0),
                    ));
                }

                // Composer footer.
                let composer_y = rect.y + height - 58;
                let composer =
                    AetherRect::new(rect.x + 14, composer_y, rect.width.saturating_sub(28), 38);
                if let Some(buffer) = self.chrome_round(composer.width, 38, 10, [3, 25, 34, 255]) {
                    panels.push((buffer, composer.x, composer.y));
                }
                labels.push((
                    "Message Aether…".into(),
                    composer.x + 14,
                    composer.y + 10,
                    1,
                    [107, 169, 182, 255],
                ));
                if let Some(buffer) = self.chrome_round(26, 26, 8, [56, 214, 255, 44]) {
                    panels.push((
                        buffer,
                        composer.x + composer.width as i32 - 34,
                        composer.y + 6,
                    ));
                }
                display_icons.push((
                    "arrow".into(),
                    14,
                    [56, 214, 255, 255],
                    composer.x + composer.width as i32 - 28,
                    composer.y + 12,
                ));
                labels.push((
                    "esc close   PgUp/PgDn scroll".into(),
                    rect.x + width - 176,
                    composer_y - 16,
                    0,
                    [107, 169, 182, 255],
                ));
            }

            if shortcuts_here {
                let rect = self.shortcuts_rect();
                let width = rect.width as i32;
                let height = rect.height as i32;
                if let Some(buffer) = self.chrome_panel(
                    rect.width,
                    rect.height,
                    14,
                    [7, 39, 51, 250],
                    Some([127, 230, 255, 34]),
                ) {
                    panels.push((buffer, rect.x - 18, rect.y - 18));
                }
                if let Some(buffer) =
                    self.chrome_round_top(rect.width.saturating_sub(2), 58, 13, [9, 49, 63, 255])
                {
                    panels.push((buffer, rect.x + 1, rect.y + 1));
                }
                display_icons.push((
                    "command".into(),
                    20,
                    [56, 214, 255, 255],
                    rect.x + 18,
                    rect.y + 18,
                ));
                labels.push((
                    format!("Keyboard shortcuts · {}", self.key_profile.name()),
                    rect.x + 50,
                    rect.y + 12,
                    2,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    self.key_profile.description().into(),
                    rect.x + 50,
                    rect.y + 35,
                    0,
                    [107, 169, 182, 255],
                ));
                let key = self.key_profile.cheat_sheet_chord();
                let key_w = self.text_width(key, 0) as i32 + 22;
                if let Some(buffer) = self.chrome_round(key_w as u32, 22, 6, [3, 25, 34, 255]) {
                    panels.push((buffer, rect.x + width - key_w - 14, rect.y + 18));
                }
                labels.push((
                    key.into(),
                    rect.x + width - key_w - 3,
                    rect.y + 23,
                    0,
                    [107, 169, 182, 255],
                ));

                let entries = self.key_profile.entries();
                let half = entries.len().div_ceil(2);
                for (column, slice) in [&entries[..half], &entries[half..]].iter().enumerate() {
                    let column_x = rect.x + 18 + column as i32 * 368;
                    let mut y = rect.y + 76;
                    let mut group = "";
                    for entry in *slice {
                        if entry.group != group {
                            group = entry.group;
                            labels.push((
                                group.to_uppercase(),
                                column_x,
                                y,
                                0,
                                [56, 214, 255, 200],
                            ));
                            y += 20;
                        }
                        labels.push((entry.chord.into(), column_x, y + 5, 0, [165, 219, 229, 255]));
                        labels.push((
                            entry.action.into(),
                            column_x + 140,
                            y + 3,
                            0,
                            [217, 247, 255, 255],
                        ));
                        y += 28;
                    }
                }
                labels.push((
                    "Esc close   ↑↓ navigate overlays   Enter activate".into(),
                    rect.x + 18,
                    rect.y + height - 22,
                    0,
                    [107, 169, 182, 255],
                ));
            }

            if audio_here {
                let rect = self.shell.audio_rect();
                let width = rect.width as i32;
                let height = rect.height as i32;
                let routes: Vec<aether_audio::AudioRoute> = self.shell.audio_routes().to_vec();
                let selected = self
                    .shell
                    .audio_layer()
                    .map(|layer| layer.selected())
                    .unwrap_or(0);
                let preset = self
                    .shell
                    .audio_layer()
                    .map(|layer| layer.preset())
                    .unwrap_or_default();
                let xruns = self
                    .shell
                    .audio_layer()
                    .map(|layer| layer.xruns())
                    .unwrap_or(0);
                let outputs = self
                    .shell
                    .audio_layer()
                    .map(|layer| layer.output_count())
                    .unwrap_or(0);
                if let Some(buffer) = self.chrome_panel(
                    rect.width,
                    rect.height,
                    14,
                    [7, 39, 51, 250],
                    Some([127, 230, 255, 34]),
                ) {
                    panels.push((buffer, rect.x - 18, rect.y - 18));
                }
                if let Some(buffer) =
                    self.chrome_round_top(rect.width.saturating_sub(2), 58, 13, [9, 49, 63, 255])
                {
                    panels.push((buffer, rect.x + 1, rect.y + 1));
                }
                display_icons.push((
                    "audio".into(),
                    20,
                    [56, 214, 255, 255],
                    rect.x + 18,
                    rect.y + 18,
                ));
                labels.push((
                    "Audio".into(),
                    rect.x + 50,
                    rect.y + 12,
                    2,
                    [233, 251, 255, 255],
                ));
                let subtitle = if xruns > 0 {
                    format!("PipeWire · 48 kHz · {xruns} xruns")
                } else {
                    "PipeWire · 48 kHz · no xruns".to_string()
                };
                labels.push((
                    subtitle,
                    rect.x + 50,
                    rect.y + 35,
                    0,
                    if xruns > 0 {
                        [232, 197, 71, 255]
                    } else {
                        [107, 169, 182, 255]
                    },
                ));
                let key = "Super+A";
                let key_w = self.text_width(key, 0) as i32 + 22;
                if let Some(buffer) = self.chrome_round(key_w as u32, 22, 6, [3, 25, 34, 255]) {
                    panels.push((buffer, rect.x + width - key_w - 14, rect.y + 18));
                }
                labels.push((
                    key.into(),
                    rect.x + width - key_w - 3,
                    rect.y + 23,
                    0,
                    [107, 169, 182, 255],
                ));
                labels.push((
                    "OUTPUT".into(),
                    rect.x + 18,
                    rect.y + 72,
                    0,
                    [107, 169, 182, 255],
                ));
                if outputs > 0 {
                    if let Some(first_input) = self.shell.audio_route_rect(outputs) {
                        labels.push((
                            "INPUT".into(),
                            rect.x + 18,
                            first_input.y - 22,
                            0,
                            [107, 169, 182, 255],
                        ));
                    }
                }
                let row_rects: Vec<AetherRect> = (0..routes.len())
                    .filter_map(|index| self.shell.audio_route_rect(index))
                    .collect();
                for (index, route) in routes.iter().enumerate() {
                    let Some(row_rect) = row_rects.get(index).copied() else {
                        continue;
                    };
                    let active = index == selected;
                    if active {
                        if let Some(buffer) = self.chrome_round(
                            row_rect.width,
                            row_rect.height,
                            8,
                            [56, 214, 255, 46],
                        ) {
                            panels.push((buffer, row_rect.x, row_rect.y));
                        }
                        if let Some(buffer) = self.chrome_round(
                            3,
                            row_rect.height.saturating_sub(14),
                            2,
                            [56, 214, 255, 255],
                        ) {
                            panels.push((buffer, row_rect.x, row_rect.y + 7));
                        }
                    }
                    let tile = AetherRect::new(row_rect.x + 10, row_rect.y + 7, 28, 26);
                    if let Some(buffer) = self.chrome_round(28, 26, 7, [3, 25, 34, 210]) {
                        panels.push((buffer, tile.x, tile.y));
                    }
                    display_icons.push((
                        if route.kind == aether_audio::RouteKind::Output {
                            "audio".into()
                        } else {
                            "mic".into()
                        },
                        16,
                        if active {
                            [56, 214, 255, 255]
                        } else {
                            [165, 219, 229, 255]
                        },
                        tile.x + 6,
                        tile.y + 5,
                    ));
                    labels.push((
                        route.label().to_string(),
                        row_rect.x + 48,
                        row_rect.y + 6,
                        1,
                        if active {
                            [233, 251, 255, 255]
                        } else {
                            [217, 247, 255, 255]
                        },
                    ));
                    let mut detail = format!("{} · {}", route.transport.label(), route.profile);
                    if let Some(codec) = &route.codec {
                        detail.push_str(" · ");
                        detail.push_str(codec);
                    }
                    labels.push((
                        detail,
                        row_rect.x + 48,
                        row_rect.y + 22,
                        0,
                        [107, 169, 182, 255],
                    ));
                    if route.default {
                        let pill_w = 64u32;
                        let pill = AetherRect::new(
                            row_rect.x + row_rect.width as i32 - pill_w as i32 - 10,
                            row_rect.y + 10,
                            pill_w,
                            20,
                        );
                        if let Some(buffer) = self.chrome_round(pill_w, 20, 6, [56, 214, 255, 36]) {
                            panels.push((buffer, pill.x, pill.y));
                        }
                        labels.push((
                            "default".into(),
                            pill.x + 9,
                            pill.y + 5,
                            0,
                            [56, 214, 255, 255],
                        ));
                    }
                }
                // Input level meter (deterministic placeholder until live peaks).
                labels.push((
                    "INPUT LEVEL".into(),
                    rect.x + 18,
                    rect.y + height - 118,
                    0,
                    [107, 169, 182, 255],
                ));
                for bar in 0..14 {
                    let level = ((bar * 37 + 53) % 11 + 3) as u32;
                    let bar_x = rect.x + 96 + bar * 11;
                    let bar_y = rect.y + height - 122 - level as i32;
                    if let Some(buffer) = self.chrome_round(
                        6,
                        level * 2,
                        2,
                        [56, 214, 255, 90 + (bar as u8 % 5) * 28],
                    ) {
                        panels.push((buffer, bar_x, bar_y));
                    }
                }
                labels.push((
                    "mic locked".into(),
                    rect.x + width - 86,
                    rect.y + height - 118,
                    0,
                    [107, 169, 182, 255],
                ));
                display_icons.push((
                    "mic".into(),
                    14,
                    [107, 169, 182, 255],
                    rect.x + width - 106,
                    rect.y + height - 122,
                ));

                for (index, option) in aether_audio::LatencyPreset::ALL.into_iter().enumerate() {
                    let Some(chip) = self.shell.audio_preset_rect(index) else {
                        continue;
                    };
                    let active = option == preset;
                    if let Some(buffer) = self.chrome_round(
                        chip.width,
                        chip.height,
                        8,
                        if active {
                            [56, 214, 255, 46]
                        } else {
                            [3, 25, 34, 210]
                        },
                    ) {
                        panels.push((buffer, chip.x, chip.y));
                    }
                    let mut name = option.name().to_string();
                    if let Some(first) = name.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    labels.push((
                        name,
                        chip.x + 12,
                        chip.y + 7,
                        1,
                        if active {
                            [233, 251, 255, 255]
                        } else {
                            [165, 219, 229, 255]
                        },
                    ));
                    labels.push((
                        format!("{} frames", option.quantum()),
                        chip.x + chip.width as i32 - 68,
                        chip.y + 9,
                        0,
                        [107, 169, 182, 255],
                    ));
                }
                labels.push((
                    "↑↓ select   Enter switch   P preset   Esc close".into(),
                    rect.x + 16,
                    rect.y + height - 22,
                    0,
                    [107, 169, 182, 255],
                ));
            }

            if task_here {
                let bounds = self.task_rect();
                let pane = self.task_pane_rect();
                let width = bounds.width as i32;
                let height = bounds.height as i32;
                let (tab, selected, scroll, snapshot, process_rows, users, collapsed) = {
                    let state = self
                        .task_manager
                        .as_ref()
                        .expect("task manager is open");
                    let snapshot = state.snapshot().clone();
                    let process_rows: Vec<(usize, aether_system::Process)> = state
                        .process_rows()
                        .into_iter()
                        .map(|(depth, process)| (depth, process.clone()))
                        .collect();
                    let users = snapshot.users.clone();
                    (
                        state.tab(),
                        state.selected(),
                        state.scroll(),
                        snapshot,
                        process_rows,
                        users,
                        state.collapsed().clone(),
                    )
                };
                let visible = self.task_visible_rows();
                if let Some(buffer) =
                    self.chrome_round(bounds.width.max(1), 62, 0, [5, 37, 48, 236])
                {
                    panels.push((buffer, bounds.x, bounds.y));
                }
                if let Some(buffer) = self.chrome_round(
                    bounds.width.max(1),
                    36,
                    0,
                    [5, 37, 48, 242],
                ) {
                    panels.push((buffer, bounds.x, bounds.y + height - 36));
                }
                if let Some(buffer) = self.chrome_panel(
                    pane.width,
                    pane.height,
                    12,
                    [7, 39, 51, 246],
                    Some([127, 230, 255, 30]),
                ) {
                    panels.push((buffer, pane.x - 18, pane.y - 18));
                }
                display_icons.push((
                    "cpu".into(),
                    28,
                    [56, 214, 255, 255],
                    bounds.x + 18,
                    bounds.y + 17,
                ));
                labels.push((
                    "Task manager".into(),
                    bounds.x + 56,
                    bounds.y + 12,
                    2,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    "Live /proc · 1 s refresh".into(),
                    bounds.x + 56,
                    bounds.y + 35,
                    0,
                    [107, 169, 182, 255],
                ));
                // Tabs.
                let tabs_x = bounds.x + width / 2 - 180;
                for (index, tab_item) in crate::task::TaskTab::ALL.into_iter().enumerate() {
                    let tab_rect = AetherRect::new(tabs_x + index as i32 * 116, bounds.y + 15, 110, 32);
                    if tab_item == tab {
                        if let Some(buffer) = self.chrome_round(110, 32, 8, [56, 214, 255, 38]) {
                            panels.push((buffer, tab_rect.x, tab_rect.y));
                        }
                    }
                    labels.push((
                        tab_item.label().into(),
                        tab_rect.x + 14,
                        tab_rect.y + 9,
                        1,
                        if tab_item == tab {
                            [233, 251, 255, 255]
                        } else {
                            [165, 219, 229, 255]
                        },
                    ));
                }
                display_icons.push((
                    "close".into(),
                    16,
                    [107, 169, 182, 255],
                    self.task_close_rect().x,
                    self.task_close_rect().y + 4,
                ));
                // Pane header.
                display_icons.push((
                    "chart".into(),
                    20,
                    [56, 214, 255, 255],
                    pane.x + 16,
                    pane.y + 14,
                ));
                labels.push((
                    tab.label().into(),
                    pane.x + 46,
                    pane.y + 12,
                    3,
                    [233, 251, 255, 255],
                ));
                let subtitle = match tab {
                    crate::task::TaskTab::Processes => format!(
                        "{} processes · {} threads · {:.0}% cpu",
                        snapshot.process_count, snapshot.thread_count, snapshot.cpu_percent
                    ),
                    crate::task::TaskTab::Users => format!("{} users", users.len()),
                    crate::task::TaskTab::Resources => format!(
                        "{} cores · {:.1} load",
                        snapshot.cores.len(),
                        snapshot.load[0]
                    ),
                };
                labels.push((
                    subtitle,
                    pane.x + 46,
                    pane.y + 34,
                    0,
                    [107, 169, 182, 255],
                ));
                let count = format!("{} tasks", snapshot.process_count);
                labels.push((
                    count.clone(),
                    pane.x + pane.width as i32 - self.text_width(&count, 0) as i32 - 16,
                    pane.y + 20,
                    0,
                    [165, 219, 229, 255],
                ));
                // Column headers / divider.
                let header_y = pane.y + 56;
                chrome.push((
                    AetherRect::new(pane.x + 1, header_y, pane.width.saturating_sub(2), 30),
                    design_color(0x052530, 190.0 / 255.0),
                ));
                match tab {
                    crate::task::TaskTab::Processes => {
                        labels.push(("PID".into(), pane.x + 16, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("USER".into(), pane.x + 82, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("CPU".into(), pane.x + 190, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("MEM".into(), pane.x + 250, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("STATE".into(), pane.x + 320, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("COMMAND".into(), pane.x + 386, header_y + 9, 0, [107, 169, 182, 255]));
                    }
                    crate::task::TaskTab::Users => {
                        labels.push(("UID".into(), pane.x + 16, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("USER".into(), pane.x + 90, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("PROCESSES".into(), pane.x + 230, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("THREADS".into(), pane.x + 350, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("CPU".into(), pane.x + 460, header_y + 9, 0, [107, 169, 182, 255]));
                        labels.push(("MEM".into(), pane.x + 540, header_y + 9, 0, [107, 169, 182, 255]));
                    }
                    crate::task::TaskTab::Resources => {}
                }
                match tab {
                    crate::task::TaskTab::Processes => {
                        let row_rects: Vec<AetherRect> =
                            (0..visible).map(|index| self.task_row_rect(index)).collect();
                        for offset in 0..visible {
                            let index = scroll + offset;
                            let Some((depth, process)) = process_rows.get(index) else {
                                break;
                            };
                            let Some(row) = row_rects.get(offset).copied() else {
                                break;
                            };
                            let active = index == selected;
                            if active {
                                if let Some(buffer) = self.chrome_round(
                                    row.width,
                                    row.height,
                                    7,
                                    [56, 214, 255, 46],
                                ) {
                                    panels.push((buffer, row.x, row.y));
                                }
                                if let Some(buffer) = self.chrome_round(
                                    3,
                                    row.height.saturating_sub(10),
                                    2,
                                    [56, 214, 255, 255],
                                ) {
                                    panels.push((buffer, row.x, row.y + 5));
                                }
                            }
                            let mut prefix = "  ".repeat(*depth);
                            if process.children.is_empty() {
                                prefix.push('·');
                            } else if collapsed.contains(&process.pid) {
                                prefix.push('▸');
                            } else {
                                prefix.push('▾');
                            }
                            labels.push((
                                format!("{prefix} {}", process.pid),
                                row.x + 12,
                                row.y + 7,
                                0,
                                [107, 169, 182, 255],
                            ));
                            labels.push((
                                process.user.clone(),
                                row.x + 66,
                                row.y + 7,
                                0,
                                [165, 219, 229, 255],
                            ));
                            labels.push((
                                format!("{:.1}%", process.cpu_percent),
                                row.x + 174,
                                row.y + 7,
                                0,
                                if process.cpu_percent >= 50.0 {
                                    [232, 197, 71, 255]
                                } else if process.cpu_percent >= 10.0 {
                                    [56, 214, 255, 255]
                                } else {
                                    [165, 219, 229, 255]
                                },
                            ));
                            labels.push((
                                format!("{:.0}M", process.rss_kb as f64 / 1024.0),
                                row.x + 234,
                                row.y + 7,
                                0,
                                [165, 219, 229, 255],
                            ));
                            labels.push((
                                process.state.clone(),
                                row.x + 304,
                                row.y + 7,
                                0,
                                [107, 169, 182, 255],
                            ));
                            let command = truncate(&process.command, 70);
                            labels.push((
                                command,
                                row.x + 370,
                                row.y + 7,
                                0,
                                [217, 247, 255, 255],
                            ));
                        }
                        if process_rows.len() > visible {
                            let track = AetherRect::new(
                                pane.x + pane.width as i32 - 9,
                                self.task_rows_top(),
                                4,
                                (visible * 28) as u32,
                            );
                            if let Some(buffer) =
                                self.chrome_round(4, track.height, 2, [3, 25, 34, 255])
                            {
                                panels.push((buffer, track.x, track.y));
                            }
                            let thumb = ((visible as f32 / process_rows.len() as f32)
                                * track.height as f32)
                                .clamp(12.0, track.height as f32)
                                as u32;
                            let offset = ((scroll as f32 / process_rows.len() as f32)
                                * track.height as f32) as i32;
                            if let Some(buffer) =
                                self.chrome_round(4, thumb, 2, [56, 214, 255, 190])
                            {
                                panels.push((buffer, track.x, track.y + offset));
                            }
                        }
                    }
                    crate::task::TaskTab::Users => {
                        let row_rects: Vec<AetherRect> =
                            (0..visible).map(|index| self.task_row_rect(index)).collect();
                        for offset in 0..visible {
                            let Some(user) = users.get(offset) else {
                                break;
                            };
                            let Some(row) = row_rects.get(offset).copied() else {
                                break;
                            };
                            labels.push((
                                user.uid.to_string(),
                                row.x + 12,
                                row.y + 7,
                                0,
                                [107, 169, 182, 255],
                            ));
                            labels.push((
                                user.name.clone(),
                                row.x + 74,
                                row.y + 7,
                                0,
                                [217, 247, 255, 255],
                            ));
                            labels.push((
                                user.processes.to_string(),
                                row.x + 214,
                                row.y + 7,
                                0,
                                [165, 219, 229, 255],
                            ));
                            labels.push((
                                user.threads.to_string(),
                                row.x + 334,
                                row.y + 7,
                                0,
                                [165, 219, 229, 255],
                            ));
                            labels.push((
                                format!("{:.1}%", user.cpu_percent),
                                row.x + 444,
                                row.y + 7,
                                0,
                                [56, 214, 255, 255],
                            ));
                            labels.push((
                                format!("{:.0}M", user.rss_kb as f64 / 1024.0),
                                row.x + 524,
                                row.y + 7,
                                0,
                                [165, 219, 229, 255],
                            ));
                        }
                    }
                    crate::task::TaskTab::Resources => {
                        let content_y = pane.y + 68;
                        let card_w = (pane.width as i32 - 44) / 2;
                        let card_h = (pane.height as i32 - 68 - 56) / 2;
                        let cards = [
                            (pane.x + 16, content_y, "CPU"),
                            (pane.x + 28 + card_w, content_y, "MEMORY"),
                            (pane.x + 16, content_y + card_h + 12, "NETWORK"),
                            (pane.x + 28 + card_w, content_y + card_h + 12, "DISK"),
                        ];
                        for (x, y, name) in cards {
                            if let Some(buffer) = self.chrome_round(
                                card_w as u32,
                                card_h as u32,
                                8,
                                [5, 37, 48, 220],
                            ) {
                                panels.push((buffer, x, y));
                            }
                            labels.push((
                                name.into(),
                                x + 12,
                                y + 10,
                                0,
                                [107, 169, 182, 255],
                            ));
                        }
                        // CPU card.
                        let cpu_x = pane.x + 28;
                        let cpu_y = content_y + 28;
                        labels.push((
                            format!("{:.0}%", snapshot.cpu_percent),
                            cpu_x,
                            cpu_y - 2,
                            3,
                            [233, 251, 255, 255],
                        ));
                        for (index, core) in snapshot.cores.iter().take(8).enumerate() {
                            let bar_x = cpu_x + (index as i32 % 2) * (card_w / 2 - 8);
                            let bar_y = cpu_y + 28 + (index as i32 / 2) * 22;
                            labels.push((
                                format!("cpu{index}"),
                                bar_x,
                                bar_y,
                                0,
                                [107, 169, 182, 255],
                            ));
                            if let Some(buffer) = self.chrome_round(64, 6, 3, [3, 25, 34, 255]) {
                                panels.push((buffer, bar_x + 38, bar_y + 2));
                            }
                            let fill = ((core.usage / 100.0) * 64.0) as u32;
                            if fill > 0 {
                                if let Some(buffer) =
                                    self.chrome_round(fill, 6, 3, [56, 214, 255, 220])
                                {
                                    panels.push((buffer, bar_x + 38, bar_y + 2));
                                }
                            }
                        }
                        // Memory card.
                        let mem_x = pane.x + 28 + card_w;
                        let mem_y = content_y + 28;
                        let used = snapshot.memory.total_kb.saturating_sub(snapshot.memory.available_kb);
                        labels.push((
                            format!("{:.1} / {:.1} GB", used as f64 / 1_048_576.0, snapshot.memory.total_kb as f64 / 1_048_576.0),
                            mem_x,
                            mem_y,
                            1,
                            [233, 251, 255, 255],
                        ));
                        let mem_bar = card_w as u32 - 24;
                        if let Some(buffer) = self.chrome_round(mem_bar, 8, 4, [3, 25, 34, 255]) {
                            panels.push((buffer, mem_x, mem_y + 24));
                        }
                        let mem_fill = if snapshot.memory.total_kb > 0 {
                            ((used as f64 / snapshot.memory.total_kb as f64) * mem_bar as f64) as u32
                        } else {
                            0
                        };
                        if mem_fill > 0 {
                            if let Some(buffer) = self.chrome_round(mem_fill, 8, 4, [56, 214, 255, 220]) {
                                panels.push((buffer, mem_x, mem_y + 24));
                            }
                        }
                        labels.push((
                            format!("swap {:.1} / {:.1} GB", (snapshot.memory.swap_total_kb - snapshot.memory.swap_free_kb) as f64 / 1_048_576.0, snapshot.memory.swap_total_kb as f64 / 1_048_576.0),
                            mem_x,
                            mem_y + 40,
                            0,
                            [107, 169, 182, 255],
                        ));
                        // Network card.
                        let net_x = pane.x + 28;
                        let net_y = content_y + card_h + 40;
                        for (index, interface) in snapshot.network.iter().take(5).enumerate() {
                            labels.push((
                                format!("{}  ↓ {}/s  ↑ {}/s", interface.name, format_rate(interface.rx_rate), format_rate(interface.tx_rate)),
                                net_x,
                                net_y + index as i32 * 20,
                                0,
                                [165, 219, 229, 255],
                            ));
                        }
                        // Disk card.
                        let disk_x = pane.x + 28 + card_w;
                        let disk_y = content_y + card_h + 40;
                        for (index, disk) in snapshot.disks.iter().take(5).enumerate() {
                            labels.push((
                                format!("{}  r {}/s  w {}/s", disk.name, format_rate(disk.read_rate), format_rate(disk.write_rate)),
                                disk_x,
                                disk_y + index as i32 * 20,
                                0,
                                [165, 219, 229, 255],
                            ));
                        }
                        labels.push((
                            format!("load {:.2} {:.2} {:.2}", snapshot.load[0], snapshot.load[1], snapshot.load[2]),
                            disk_x,
                            disk_y + 112,
                            0,
                            [107, 169, 182, 255],
                        ));
                    }
                }
                // Footer + status.
                let footer_y = pane.y + pane.height as i32 - 30;
                chrome.push((
                    AetherRect::new(pane.x + 1, footer_y, pane.width.saturating_sub(2), 29),
                    design_color(0x052530, 190.0 / 255.0),
                ));
                if let Some((_, process)) = process_rows.get(selected) {
                    labels.push((
                        format!("{} · pid {}", truncate(&process.command, 58), process.pid),
                        pane.x + 16,
                        footer_y + 9,
                        0,
                        [165, 219, 229, 255],
                    ));
                }
                labels.push((
                    "↑↓ select  ←→ collapse  tab tabs  r refresh  esc close".into(),
                    pane.x + pane.width as i32 - 430,
                    footer_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                let status_y = bounds.y + height - 36;
                labels.push((
                    format!(
                        "{} processes · {} threads · cpu {:.0}% · load {:.2} · up {}",
                        snapshot.process_count,
                        snapshot.thread_count,
                        snapshot.cpu_percent,
                        snapshot.load[0],
                        format_uptime(snapshot.uptime_secs)
                    ),
                    bounds.x + 16,
                    status_y + 12,
                    0,
                    [165, 219, 229, 255],
                ));
                labels.push((
                    format!("mem {:.1}/{:.1} GB", (snapshot.memory.total_kb - snapshot.memory.available_kb) as f64 / 1_048_576.0, snapshot.memory.total_kb as f64 / 1_048_576.0),
                    bounds.x + width - 180,
                    status_y + 12,
                    0,
                    [107, 169, 182, 255],
                ));
            }

            if explorer_here {
                let bounds = self.shell.explorer_rect();
                let pane = self.shell.explorer_pane_rect();
                let width = bounds.width as i32;
                let height = bounds.height as i32;
                // App bar and status bar.
                if let Some(buffer) =
                    self.chrome_round(bounds.width.max(1), 62, 0, [5, 37, 48, 236])
                {
                    panels.push((buffer, bounds.x, bounds.y));
                }
                if let Some(buffer) =
                    self.chrome_round(bounds.width.max(1), 36, 0, [5, 37, 48, 242])
                {
                    panels.push((buffer, bounds.x, bounds.y + height - 36));
                }
                // Pane card.
                if let Some(buffer) = self.chrome_panel(
                    pane.width,
                    pane.height,
                    12,
                    [7, 39, 51, 246],
                    Some([127, 230, 255, 30]),
                ) {
                    panels.push((buffer, pane.x - 18, pane.y - 18));
                }
                // App bar brand.
                display_icons.push((
                    "explorer".into(),
                    28,
                    [56, 214, 255, 255],
                    bounds.x + 18,
                    bounds.y + 17,
                ));
                labels.push((
                    "Aether".into(),
                    bounds.x + 56,
                    bounds.y + 12,
                    2,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    "Files · indexed system".into(),
                    bounds.x + 56,
                    bounds.y + 35,
                    0,
                    [107, 169, 182, 255],
                ));
                // Center tabs.
                let tab_x = bounds.x + width / 2 - 150;
                let tab_active = AetherRect::new(tab_x, bounds.y + 15, 92, 32);
                if let Some(buffer) = self.chrome_round(92, 32, 8, [56, 214, 255, 38]) {
                    panels.push((buffer, tab_active.x, tab_active.y));
                }
                labels.push((
                    "Files".into(),
                    tab_x + 26,
                    bounds.y + 22,
                    1,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    "Palette".into(),
                    tab_x + 120,
                    bounds.y + 22,
                    1,
                    [165, 219, 229, 255],
                ));
                labels.push((
                    "Chat".into(),
                    tab_x + 215,
                    bounds.y + 22,
                    1,
                    [165, 219, 229, 255],
                ));
                // Search affordance.
                let search = AetherRect::new(bounds.x + width - 250, bounds.y + 15, 232, 32);
                if let Some(buffer) = self.chrome_round(232, 32, 8, [3, 25, 34, 255]) {
                    panels.push((buffer, search.x, search.y));
                }
                display_icons.push((
                    "search".into(),
                    16,
                    [107, 169, 182, 255],
                    search.x + 10,
                    search.y + 8,
                ));
                labels.push((
                    "Search files".into(),
                    search.x + 32,
                    search.y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                labels.push((
                    "Super+K".into(),
                    search.x + 176,
                    search.y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                // Pane header.
                let pane_icon = pane.x + 16;
                let pane_y = pane.y + 14;
                display_icons.push(("folder".into(), 22, [56, 214, 255, 255], pane_icon, pane_y));
                labels.push((
                    "Files".into(),
                    pane.x + 48,
                    pane.y + 14,
                    3,
                    [233, 251, 255, 255],
                ));
                labels.push((
                    "All indexed roots".into(),
                    pane.x + 48,
                    pane.y + 34,
                    0,
                    [107, 169, 182, 255],
                ));
                let count = format!("{} files", self.shell.explorer_match_count());
                labels.push((
                    count.clone(),
                    pane.x + pane.width as i32 - self.text_width(&count, 0) as i32 - 50,
                    pane.y + 20,
                    0,
                    [165, 219, 229, 255],
                ));
                display_icons.push((
                    "close".into(),
                    16,
                    [107, 169, 182, 255],
                    pane.x + pane.width as i32 - 32,
                    pane.y + 20,
                ));
                // Column header.
                let columns_y = pane.y + 56;
                chrome.push((
                    AetherRect::new(pane.x + 1, columns_y, pane.width.saturating_sub(2), 30),
                    design_color(0x052530, 190.0 / 255.0),
                ));
                let type_x = pane.x + (pane.width as f32 * 0.62) as i32;
                labels.push((
                    "NAME".into(),
                    pane.x + 16,
                    columns_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                labels.push((
                    "TYPE".into(),
                    type_x,
                    columns_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                labels.push((
                    "LOCATION".into(),
                    pane.x + pane.width as i32 - self.text_width("LOCATION", 0) as i32 - 16,
                    columns_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                // Rows.
                let rows: Vec<aether_shell::PaletteFile> = self
                    .shell
                    .visible_explorer_rows()
                    .into_iter()
                    .cloned()
                    .collect();
                let scroll = self.shell.explorer_scroll();
                let selected = self.shell.explorer_selected();
                let row_height = aether_shell::design::EXPLORER_ROW_HEIGHT as i32;
                let row_top = self.shell.explorer_rows_top();
                for (row, file) in rows.iter().enumerate() {
                    let index = scroll + row;
                    let y = row_top + row as i32 * row_height;
                    let row_rect = AetherRect::new(
                        pane.x + 8,
                        y,
                        pane.width.saturating_sub(16),
                        (row_height - 2) as u32,
                    );
                    if index == selected {
                        if let Some(buffer) = self.chrome_round(
                            row_rect.width,
                            row_rect.height,
                            7,
                            [56, 214, 255, 56],
                        ) {
                            panels.push((buffer, row_rect.x, row_rect.y));
                        }
                        chrome.push((
                            AetherRect::new(
                                row_rect.x,
                                row_rect.y + 4,
                                3,
                                row_rect.height.saturating_sub(8),
                            ),
                            design_color(aether_shell::design::ACCENT, 1.0),
                        ));
                    }
                    let extension = file
                        .name
                        .rsplit_once('.')
                        .map(|(_, extension)| extension.to_uppercase())
                        .unwrap_or_else(|| "FILE".to_string());
                    let kind = file_kind(&extension);
                    display_icons.push((kind.0.into(), 16, kind.1, row_rect.x + 8, row_rect.y + 7));
                    labels.push((
                        file.name.clone(),
                        row_rect.x + 30,
                        row_rect.y + 8,
                        1,
                        if index == selected {
                            [233, 251, 255, 255]
                        } else {
                            [217, 247, 255, 255]
                        },
                    ));
                    labels.push((extension, type_x, row_rect.y + 9, 0, [165, 219, 229, 255]));
                    let location = truncate(&file.detail, 34);
                    labels.push((
                        location,
                        pane.x + pane.width as i32
                            - self.text_width(&truncate(&file.detail, 34), 0) as i32
                            - 16,
                        row_rect.y + 9,
                        0,
                        [107, 169, 182, 255],
                    ));
                }
                // Pane footer.
                let footer_y = pane.y + pane.height as i32 - 30;
                chrome.push((
                    AetherRect::new(pane.x + 1, footer_y, pane.width.saturating_sub(2), 29),
                    design_color(0x052530, 190.0 / 255.0),
                ));
                labels.push((
                    format!(
                        "{} of {} files",
                        rows.len(),
                        self.shell.explorer_match_count()
                    ),
                    pane.x + 16,
                    footer_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                let hint = "Enter open   Esc close   PgUp/PgDn page";
                labels.push((
                    hint.into(),
                    pane.x + pane.width as i32 - self.text_width(hint, 0) as i32 - 16,
                    footer_y + 9,
                    0,
                    [107, 169, 182, 255],
                ));
                // Status bar.
                let status_y = bounds.y + height - 36;
                let selected_file = self.shell.explorer_selected_file();
                if let Some(file) = selected_file {
                    labels.push((
                        file.name.clone(),
                        bounds.x + 16,
                        status_y + 12,
                        1,
                        [233, 251, 255, 255],
                    ));
                    labels.push((
                        truncate(&file.path, 60),
                        bounds.x + 16 + self.text_width(&file.name, 1) as i32 + 10,
                        status_y + 13,
                        0,
                        [107, 169, 182, 255],
                    ));
                } else {
                    labels.push((
                        "No selection".into(),
                        bounds.x + 16,
                        status_y + 12,
                        1,
                        [107, 169, 182, 255],
                    ));
                }
                let status = self.status().clone();
                let mut sx = bounds.x + width - 16;
                let badges = [
                    ("agents", format!("{} agents", status.agents)),
                    ("cpu", format!("{}%", status.cpu_percent)),
                    (
                        "memory",
                        format!("{}/{} MB", status.mem_used_mb, status.mem_total_mb),
                    ),
                    ("audio", status.audio.clone()),
                    ("chart", format!("{:.1} ms", status.frame_ms)),
                ];
                for (icon, text) in badges.iter().rev() {
                    let tw = self.text_width(text, 0) as i32;
                    sx -= tw + 22;
                    display_icons.push((
                        (*icon).into(),
                        14,
                        [107, 169, 182, 255],
                        sx,
                        status_y + 11,
                    ));
                    labels.push((
                        text.clone(),
                        sx + 18,
                        status_y + 12,
                        0,
                        [165, 219, 229, 255],
                    ));
                    sx -= 14;
                }
            }

            if !task_here && !explorer_here && self.shell.fullscreen_window_on(chrome_output).is_none() {
                if !chat_here && !audio_here && !shortcuts_here && self.shell.overlay().is_none() {
                    let bar = crate::topbar::layout(self.shell.output_bounds(chrome_output));
                    if let Some(buffer) =
                        self.chrome_round(bar.rect.width, bar.rect.height, 10, [5, 37, 48, 225])
                    {
                        panels.push((buffer, bar.rect.x, bar.rect.y));
                    }
                    for (action, item) in &bar.items {
                        if let Some(buffer) =
                            self.chrome_round(item.width, item.height, 6, [3, 25, 34, 220])
                        {
                            panels.push((buffer, item.x, item.y));
                        }
                        if bar.compact {
                            display_icons.push((
                                action.icon().into(),
                                16,
                                [165, 219, 229, 255],
                                item.x + (item.width as i32 - 16) / 2,
                                item.y + 6,
                            ));
                        } else {
                            display_icons.push((
                                action.icon().into(),
                                15,
                                [56, 214, 255, 255],
                                item.x + 9,
                                item.y + 7,
                            ));
                            labels.push((
                                action.label().into(),
                                item.x + 29,
                                item.y + 8,
                                0,
                                [217, 247, 255, 255],
                            ));
                        }
                    }
                }
                if self.status_visible() && !chat_here {
                    let bounds = self.shell.output_bounds(chrome_output);
                    let text = self.status().label();
                    let width = self.text_width(&text, 0) as i32 + 42;
                    let x = bounds.x + bounds.width as i32 - width - 12;
                    if let Some(buffer) = self.chrome_round(width as u32, 26, 13, [5, 37, 48, 226])
                    {
                        panels.push((buffer, x, bounds.y + 10));
                    }
                    if let Some(buffer) = self.chrome_round(6, 6, 3, [67, 214, 138, 255]) {
                        panels.push((buffer, x + 12, bounds.y + 20));
                    }
                    labels.push((text, x + 24, bounds.y + 15, 0, [179, 235, 250, 255]));
                }
                let names = self.shell.workspace_names();
                for workspace in 0..self.shell.workspace_count() {
                    let Some(chip) = self
                        .shell
                        .workspace_indicator_rect_for(chrome_output, workspace)
                    else {
                        continue;
                    };
                    let active = workspace == self.shell.output_workspace(chrome_output);
                    if let Some(buffer) = self.chrome_round(
                        chip.width,
                        WORKSPACE_INDICATOR_BAR_HEIGHT,
                        3,
                        if active {
                            [56, 214, 255, 255]
                        } else {
                            [56, 214, 255, 46]
                        },
                    ) {
                        panels.push((buffer, chip.x, chip.y));
                    }
                    let name = names.get(workspace).map(String::as_str).unwrap_or("");
                    labels.push((
                        format!("{} {name}", workspace + 1),
                        chip.x + 2,
                        chip.y + 12,
                        0,
                        if active {
                            [56, 214, 255, 255]
                        } else {
                            [102, 158, 173, 255]
                        },
                    ));
                }
            }
        }

        for (buffer, x, y) in &panels {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (*x as f64, *y as f64),
                buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        // Chrome solids, bottom-up.
        let buffers: Vec<SolidColorBuffer> = chrome
            .iter()
            .map(|(rect, color)| {
                SolidColorBuffer::new((rect.width as i32, rect.height as i32), *color)
            })
            .collect();
        elements.extend(
            buffers
                .iter()
                .zip(chrome.iter())
                .map(|(buffer, (rect, _))| {
                    AetherRenderElement::from(SolidColorRenderElement::from_buffer(
                        buffer,
                        (rect.x - offset.0, rect.y - offset.1),
                        1.0f64,
                        1.0,
                        Kind::Unspecified,
                    ))
                }),
        );

        // Palette icons.
        for (path, x, y) in &icons {
            if !self.icons.contains_key(path) {
                if let Some((width, height, pixels)) = load_icon(path) {
                    let buffer = MemoryRenderBuffer::from_slice(
                        &pixels,
                        Fourcc::Abgr8888,
                        (width as i32, height as i32),
                        1,
                        Transform::Normal,
                        None,
                    );
                    self.icons.insert(path.clone(), buffer);
                }
            }
            if let Some(buffer) = self.icons.get(path) {
                if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    ((*x - offset.0) as f64, (*y - offset.1) as f64),
                    buffer,
                    None,
                    None,
                    Some(Size::from((16, 16))),
                    Kind::Unspecified,
                ) {
                    elements.push(AetherRenderElement::from(element));
                }
            }
        }

        // Vector icons from the design system.
        for (name, size, color, x, y) in &display_icons {
            if let Some(buffer) = self.chrome_icon(name, *size, *color) {
                if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    ((*x - offset.0) as f64, (*y - offset.1) as f64),
                    &buffer,
                    None,
                    None,
                    None,
                    Kind::Unspecified,
                ) {
                    elements.push(AetherRenderElement::from(element));
                }
            }
        }

        // Text labels. Cache by content/scale/colour so static chrome does not
        // rasterize and upload the same bitmap every frame.
        let mut rasterized = false;
        for (text, x, y, scale, color) in &labels {
            let base_scale = *scale & 0xff;
            let bold = *scale >= 2 || (*scale & 0x100) != 0;
            let key = (text.clone(), *scale, *color, bold);
            let buffer = self.text_cache.entry(key).or_insert_with(|| {
                rasterized = true;
                let bitmap = match self.font.as_ref() {
                    Some(font) => {
                        font.render(text, crate::app::App::text_px(base_scale), *color, bold)
                    }
                    None => TextBitmap::render(text, base_scale.max(1), *color),
                };
                MemoryRenderBuffer::from_slice(
                    bitmap.pixels(),
                    Fourcc::Abgr8888,
                    (bitmap.width() as i32, bitmap.height() as i32),
                    1,
                    Transform::Normal,
                    None,
                )
            });
            if rasterized {
                self.text_rasterizations += 1;
                rasterized = false;
            }
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                ((*x - offset.0) as f64, (*y - offset.1) as f64),
                buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        // IME candidate/preedit popups above client windows.
        for popup in &self.input_method_popups {
            if !popup.alive() {
                continue;
            }
            let parent = popup.get_parent().and_then(|parent| {
                self.windows.iter().find_map(|(id, toplevel)| {
                    (toplevel.wl_surface() == &parent.surface).then_some(*id)
                })
            });
            if output.is_some() && !parent.is_some_and(|id| visible.contains(&id)) {
                continue;
            }
            let parent_origin = parent
                .and_then(|id| rects.get(&id).copied())
                .map(|rect| (rect.x - offset.0, rect.y - offset.1))
                .unwrap_or((-offset.0, -offset.1));
            let location = popup.location();
            elements.extend(
                render_elements_from_surface_tree::<
                    GlesRenderer,
                    WaylandSurfaceRenderElement<GlesRenderer>,
                >(
                    renderer,
                    popup.wl_surface(),
                    (parent_origin.0 + location.x, parent_origin.1 + location.y),
                    1.0,
                    1.0,
                    Kind::Unspecified,
                )
                .into_iter()
                .map(AetherRenderElement::from),
            );
        }

        // Pointer and client cursor surfaces.
        let pointer = self.pointer_pos;
        let pointer_output = self.shell.output_at(aether_geometry::Point::new(
            pointer.x.round() as i32,
            pointer.y.round() as i32,
        ));
        let show_cursor = output.is_none()
            || pointer_output == output
            || (pointer_output.is_none() && output == Some(primary));

        if show_cursor {
            let shape = self.cursor;
            let theme_bitmap = self
                .cursor_theme
                .as_ref()
                .and_then(|theme| theme.load(shape, 24));
            self.cursor_buffers.entry(shape).or_insert_with(|| {
                let bitmap = theme_bitmap.unwrap_or_else(|| render_cursor(shape));
                let buffer = MemoryRenderBuffer::from_slice(
                    &bitmap.pixels,
                    Fourcc::Abgr8888,
                    (bitmap.width as i32, bitmap.height as i32),
                    1,
                    Transform::Normal,
                    None,
                );
                CursorCacheEntry {
                    buffer,
                    hotspot_x: bitmap.hotspot_x,
                    hotspot_y: bitmap.hotspot_y,
                }
            });
            if !self.cursor_hidden {
                if let Some(entry) = self.cursor_buffers.get(&shape) {
                    let x = pointer.x.round() as i32 - offset.0 - entry.hotspot_x;
                    let y = pointer.y.round() as i32 - offset.1 - entry.hotspot_y;
                    if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        (x as f64, y as f64),
                        &entry.buffer,
                        None,
                        None,
                        None,
                        Kind::Unspecified,
                    ) {
                        elements.push(AetherRenderElement::from(element));
                    }
                }
            }

            if let Some(surface) = self
                .client_cursor_surface
                .clone()
                .filter(|surface| surface.alive())
            {
                let hotspot = cursor_hotspot(&surface);
                elements.extend(
                    render_elements_from_surface_tree::<
                        GlesRenderer,
                        WaylandSurfaceRenderElement<GlesRenderer>,
                    >(
                        renderer,
                        &surface,
                        (
                            pointer.x.round() as i32 - offset.0 - hotspot.0,
                            pointer.y.round() as i32 - offset.1 - hotspot.1,
                        ),
                        1.0,
                        1.0,
                        Kind::Unspecified,
                    )
                    .into_iter()
                    .map(AetherRenderElement::from),
                );
            }
        }

        // Everything above is bottom-up; compositors expect topmost-first.
        elements.reverse();
        if std::env::var_os("AETHER_TEXT_STATS").is_some() {
            println!(
                "aether-shell: text rasterizations={} output={:?}",
                self.text_rasterizations, output
            );
        }
        let damage = self
            .output_damage(output)
            .map(|(_, damage)| damage)
            .unwrap_or_default();
        ComposedFrame {
            elements,
            clear: BACKGROUND,
            damage,
        }
    }
}

fn cursor_hotspot(surface: &WlSurface) -> (i32, i32) {
    with_states(surface, |states| {
        states
            .data_map
            .get::<CursorImageSurfaceData>()
            .and_then(|data| data.lock().ok())
            .map(|attributes| (attributes.hotspot.x, attributes.hotspot.y))
            .unwrap_or((0, 0))
    })
}

fn load_icon(path: &str) -> Option<(u32, u32, Vec<u8>)> {
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

fn format_rate(bytes_per_second: f64) -> String {
    if bytes_per_second >= 1_048_576.0 {
        format!("{:.1}M", bytes_per_second / 1_048_576.0)
    } else if bytes_per_second >= 1024.0 {
        format!("{:.0}K", bytes_per_second / 1024.0)
    } else {
        format!("{:.0}B", bytes_per_second)
    }
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut result: String = text.chars().take(limit.saturating_sub(1)).collect();
    result.push('…');
    result
}

fn file_kind(extension: &str) -> (&'static str, [u8; 4]) {
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
