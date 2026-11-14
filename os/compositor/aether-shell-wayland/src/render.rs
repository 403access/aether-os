//! Render element composition shared by every Aether backend.
//!
//! `App::compose` builds a single front-to-back element list covering client
//! surfaces, compositor chrome, palette icons/text, IME popups and cursors.
//! Winit draws the list into its GL framebuffer; the DRM backend hands the
//! same list to `DrmCompositor` for scan-out.

use std::collections::HashMap;

use aether_cursor::render as render_cursor;
use aether_geometry::Rect as AetherRect;
use aether_shell::{
    OutputId, PaletteAction, WindowId, PALETTE_VISIBLE_ROWS, WORKSPACE_INDICATOR_BAR_HEIGHT,
};
use aether_text::{text_width, TextBitmap};
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
pub const BACKGROUND: Color32F = Color32F::new(0.01, 0.06, 0.08, 1.0);

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
}

impl App {
    /// Compose one output (or the single logical output when `None`).
    pub fn compose(
        &mut self,
        renderer: &mut GlesRenderer,
        output: Option<OutputId>,
    ) -> ComposedFrame {
        let mut elements: Vec<AetherRenderElement> = Vec::new();

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
                    (rect.x, rect.y),
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
        let show_chrome = output.is_none() || output == Some(primary);

        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut icons: Vec<(String, i32, i32)> = Vec::new();

        if show_chrome {
            if let Some(rect) = self.shell.overlay_rect() {
                chrome.push((rect, [0.05, 0.24, 0.29, 0.96]));
                chrome.push((
                    AetherRect::new(rect.x, rect.y, rect.width, 36.min(rect.height)),
                    [0.09, 0.42, 0.5, 1.0],
                ));
                labels.push((
                    "Aether Palette".into(),
                    rect.x + 14,
                    rect.y + 10,
                    2,
                    [235, 252, 255, 255],
                ));
                let hint = "Super+Space";
                labels.push((
                    hint.into(),
                    rect.x + rect.width as i32 - text_width(hint, 1) as i32 - 14,
                    rect.y + 14,
                    1,
                    [179, 235, 250, 255],
                ));

                if rect.height > 96 {
                    let input = AetherRect::new(
                        rect.x + 16,
                        rect.y + 52,
                        rect.width.saturating_sub(32),
                        28,
                    );
                    chrome.push((input, [0.03, 0.12, 0.16, 1.0]));
                    let query = self.shell.palette_query();
                    if query.is_empty() {
                        labels.push((
                            "Search workspaces...".into(),
                            input.x + 10,
                            input.y + 10,
                            1,
                            [122, 173, 189, 255],
                        ));
                    } else {
                        labels.push((
                            format!("> {query}_"),
                            input.x + 10,
                            input.y + 10,
                            1,
                            [235, 252, 255, 255],
                        ));
                    }

                    let entries = self.shell.palette_entries();
                    let selected = self.shell.palette_selected();
                    if entries.is_empty() {
                        labels.push((
                            "No matching commands".into(),
                            rect.x + 26,
                            rect.y + 106,
                            1,
                            [128, 184, 199, 255],
                        ));
                    }
                    let start = self.shell.palette_visible_start();
                    for (row, entry) in entries
                        .iter()
                        .skip(start)
                        .take(PALETTE_VISIBLE_ROWS)
                        .enumerate()
                    {
                        let y = rect.y + 96 + row as i32 * 48;
                        if y + 32 < rect.y + rect.height as i32 {
                            let row_rect =
                                AetherRect::new(rect.x + 16, y, rect.width.saturating_sub(32), 32);
                            let active = start + row == selected;
                            chrome.push((
                                row_rect,
                                if active {
                                    [0.09, 0.42, 0.5, 1.0]
                                } else {
                                    [0.05, 0.31, 0.37, 1.0]
                                },
                            ));
                            if active {
                                chrome.push((
                                    AetherRect::new(row_rect.x, row_rect.y, 3, row_rect.height),
                                    [0.22, 0.84, 1.0, 1.0],
                                ));
                            }
                            let icon = match entry.action {
                                PaletteAction::LaunchApp(index) => self
                                    .shell
                                    .launcher(index)
                                    .and_then(|launcher| launcher.icon),
                                _ => None,
                            };
                            let title_x = if let Some(icon) = icon {
                                icons.push((icon, row_rect.x + 10, row_rect.y + 8));
                                row_rect.x + 34
                            } else {
                                row_rect.x + 12
                            };
                            labels.push((
                                entry.title.clone(),
                                title_x,
                                row_rect.y + 12,
                                1,
                                if active {
                                    [235, 252, 255, 255]
                                } else {
                                    [217, 247, 255, 255]
                                },
                            ));
                            labels.push((
                                entry.detail.clone(),
                                row_rect.x + row_rect.width as i32
                                    - text_width(&entry.detail, 1) as i32
                                    - 10,
                                row_rect.y + 12,
                                1,
                                [128, 184, 199, 255],
                            ));
                        }
                    }

                    let footer = "Esc close   Enter run   Up/Down select";
                    labels.push((
                        footer.into(),
                        rect.x + 16,
                        rect.y + rect.height as i32 - 22,
                        1,
                        [128, 184, 199, 255],
                    ));
                }
            }

            if self.shell.fullscreen_window().is_none() {
                let names = self.shell.workspace_names();
                for workspace in 0..self.shell.workspace_count() {
                    let Some(chip) = self.shell.workspace_indicator_rect(workspace) else {
                        continue;
                    };
                    let active = workspace == self.shell.active_workspace();
                    chrome.push((
                        AetherRect::new(chip.x, chip.y, chip.width, WORKSPACE_INDICATOR_BAR_HEIGHT),
                        if active {
                            [0.22, 0.84, 1.0, 1.0]
                        } else {
                            [0.12, 0.3, 0.35, 0.9]
                        },
                    ));
                    let name = names.get(workspace).map(String::as_str).unwrap_or("");
                    labels.push((
                        format!("{} {name}", workspace + 1),
                        chip.x + 4,
                        chip.y + 12,
                        1,
                        if active {
                            [56, 214, 255, 255]
                        } else {
                            [102, 158, 173, 255]
                        },
                    ));
                }
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
                        (rect.x, rect.y),
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
                    (*x as f64, *y as f64),
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

        // Text labels.
        for (text, x, y, scale, color) in &labels {
            let bitmap = TextBitmap::render(text, *scale, *color);
            if bitmap.is_empty() {
                continue;
            }
            let buffer = MemoryRenderBuffer::from_slice(
                bitmap.pixels(),
                Fourcc::Abgr8888,
                (bitmap.width() as i32, bitmap.height() as i32),
                1,
                Transform::Normal,
                None,
            );
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (*x as f64, *y as f64),
                &buffer,
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
            let origin = parent
                .and_then(|id| rects.get(&id).copied())
                .map(|rect| (rect.x, rect.y))
                .unwrap_or((0, 0));
            let location = popup.location();
            elements.extend(
                render_elements_from_surface_tree::<
                    GlesRenderer,
                    WaylandSurfaceRenderElement<GlesRenderer>,
                >(
                    renderer,
                    popup.wl_surface(),
                    (origin.0 + location.x, origin.1 + location.y),
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
                    let x = pointer.x.round() as i32 - entry.hotspot_x;
                    let y = pointer.y.round() as i32 - entry.hotspot_y;
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
                            pointer.x.round() as i32 - hotspot.0,
                            pointer.y.round() as i32 - hotspot.1,
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
        ComposedFrame {
            elements,
            clear: BACKGROUND,
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
