//! Frame orchestration: context, background, surfaces, chrome and cursor.

use std::collections::HashMap;

use aether_cursor::render as render_cursor;
use aether_geometry::Rect as AetherRect;
use aether_shell::{OutputId, WindowId};
use aether_text::TextBitmap;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{IsAlive, Size, Transform};

use crate::app::{App, CursorCacheEntry};
use crate::render::context::{FrameContext, RenderChunk, SurfaceFlags};
use crate::render::element::{AetherRenderElement, ComposedFrame, BACKGROUND};
use crate::render::helpers::{cursor_hotspot, load_icon};

impl App {
    /// Compose one output (or the single logical output when `None`).
    pub fn compose(
        &mut self,
        renderer: &mut GlesRenderer,
        output: Option<OutputId>,
    ) -> ComposedFrame {
        self.poll_status();
        let ctx = self.frame_context(output);
        let mut elements: Vec<AetherRenderElement> = Vec::new();

        self.compose_background(&ctx, renderer, &mut elements);
        self.compose_windows(&ctx, renderer, &mut elements);

        let mut chunk = RenderChunk::default();
        chunk.merge(self.compose_palette(&ctx));
        chunk.merge(self.compose_chat(&ctx));
        chunk.merge(self.compose_shortcuts(&ctx));
        chunk.merge(self.compose_themes(&ctx));
        chunk.merge(self.compose_audio(&ctx));
        chunk.merge(self.compose_remote(&ctx));
        chunk.merge(self.compose_task_manager(&ctx));
        chunk.merge(self.compose_sandboxes(&ctx));
        chunk.merge(self.compose_explorer(&ctx));
        chunk.merge(self.compose_calendar(&ctx));
        chunk.merge(self.compose_system_app(&ctx));
        chunk.merge(self.compose_tray(&ctx));
        chunk.merge(self.compose_dock(&ctx));
        chunk.merge(self.compose_status_chrome(&ctx));

        self.compose_flush(&ctx, renderer, &chunk, &mut elements);
        self.compose_ime(&ctx, renderer, &mut elements);
        self.compose_cursor(&ctx, renderer, &mut elements);

        // Everything above is bottom-up; compositors expect topmost-first.
        elements.reverse();
        if std::env::var_os("AETHER_TEXT_STATS").is_some() {
            println!(
                "aether-shell: text rasterizations={} cache_bytes={} cache_entries={} cache_evictions={} output={:?}",
                self.text_rasterizations,
                self.text_cache.bytes(),
                self.text_cache.len(),
                self.text_cache.evictions(),
                output
            );
            println!(
                "aether-shell: chrome cache bytes icons={} panels={} rounds={} tops={} backgrounds={}",
                self.chrome_icons.bytes(),
                self.chrome_panels.bytes(),
                self.chrome_rounds.bytes(),
                self.chrome_tops.bytes(),
                self.chrome_background.bytes()
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

    fn frame_context(&self, output: Option<OutputId>) -> FrameContext {
        let tokens = self.shell.theme_tokens();
        let settings = *self.shell.theme_settings();
        let panel_fill = if settings.glass {
            tokens.panel
        } else {
            tokens.panel_opaque()
        };
        let panel_fill_soft = if settings.glass {
            tokens.panel_soft
        } else {
            tokens.panel_soft_opaque()
        };
        let panel_border = settings.glass.then(|| tokens.border(34));
        let panel_border_soft = settings.glass.then(|| tokens.border(30));
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
        let content_rects: HashMap<WindowId, AetherRect> = self
            .shell
            .windows()
            .into_iter()
            .map(|window| {
                let inset = if window.fullscreen {
                    0
                } else {
                    crate::app::WINDOW_FRAME_HEIGHT
                };
                (
                    window.id,
                    AetherRect::new(
                        window.rect.x,
                        window.rect.y + inset,
                        window.rect.width,
                        window.rect.height.saturating_sub(inset as u32),
                    ),
                )
            })
            .collect();
        let primary = self
            .shell
            .primary_output()
            .map(|output| output.id)
            .unwrap_or(1);
        let chrome_output = output.unwrap_or(primary);
        let palette = !self.shell.explorer_open()
            && (output.is_none()
                || self.shell.overlay_output() == Some(chrome_output)
                || self.shell.overlay_output().is_none());
        let explorer = self.shell.explorer_open() && (output.is_none() || output == Some(primary));
        let chat = self.shell.chat_open() && (output.is_none() || output == Some(primary));
        let audio = self.shell.audio_open() && (output.is_none() || output == Some(primary));
        let themes = self.shell.themes_open() && (output.is_none() || output == Some(primary));
        let remote = self.shell.remote_open() && (output.is_none() || output == Some(primary));
        let shortcuts = self.shortcuts_open && (output.is_none() || output == Some(primary));
        let task = self.task_manager_open() && (output.is_none() || output == Some(primary));
        let sandboxes = self.sandbox_panel_open() && (output.is_none() || output == Some(primary));

        FrameContext {
            output,
            primary,
            chrome_output,
            offset,
            width,
            height,
            visible,
            content_rects,
            tokens,
            settings,
            panel_fill,
            panel_fill_soft,
            panel_border,
            panel_border_soft,
            flags: SurfaceFlags {
                palette,
                explorer,
                chat,
                audio,
                remote,
                shortcuts,
                task,
                themes,
                sandboxes,
            },
        }
    }

    fn compose_background(
        &mut self,
        ctx: &FrameContext,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
    ) {
        let settings = ctx.settings;
        let width = ctx.width;
        let height = ctx.height;
        let background_style = super::chrome::BackgroundStyle {
            mode: settings.mode,
            background: settings.background,
            tone: settings.tone,
            caustics: settings.caustics,
            effects: settings.effects,
            speed: settings.speed,
        };
        if let Some(background) = self.chrome_background(width, height, background_style) {
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
    }

    fn compose_windows(
        &mut self,
        ctx: &FrameContext,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
    ) {
        let offset = ctx.offset;
        let visible = &ctx.visible;
        let content_rects = &ctx.content_rects;
        // Client surfaces and their frames, bottom-most first so overlapping
        // floating windows keep the compositor's z-order.
        for id in visible.iter().rev() {
            let Some(rect) = content_rects.get(id) else {
                continue;
            };
            let Some(surface) = self
                .windows
                .get(id)
                .map(|toplevel| toplevel.wl_surface().clone())
            else {
                continue;
            };
            elements.extend(
                render_elements_from_surface_tree::<
                    GlesRenderer,
                    WaylandSurfaceRenderElement<GlesRenderer>,
                >(
                    renderer,
                    &surface,
                    (rect.x - offset.0, rect.y - offset.1),
                    1.0,
                    1.0,
                    Kind::Unspecified,
                )
                .into_iter()
                .map(AetherRenderElement::from),
            );
            let Some(window) = self
                .shell
                .windows()
                .into_iter()
                .find(|window| window.id == *id)
            else {
                continue;
            };
            if window.fullscreen {
                continue;
            }
            let focused = self.shell.focused_window() == Some(*id);
            self.render_window_frame(
                renderer,
                elements,
                &ctx.tokens,
                offset,
                *id,
                window.rect,
                &window.title,
                focused,
            );
        }
    }

    fn compose_flush(
        &mut self,
        ctx: &FrameContext,
        renderer: &mut GlesRenderer,
        chunk: &RenderChunk,
        elements: &mut Vec<AetherRenderElement>,
    ) {
        let offset = ctx.offset;
        let chrome = &chunk.chrome;
        let labels = &chunk.labels;
        let icons = &chunk.icons;
        let panels = &chunk.panels;
        let display_icons = &chunk.display_icons;
        for (buffer, x, y) in panels {
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
        for (path, x, y) in icons {
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
        for (name, size, color, x, y) in display_icons {
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

        for (text, x, y, scale, color) in labels {
            self.push_label(
                renderer,
                elements,
                text,
                (*x - offset.0, *y - offset.1),
                *scale,
                *color,
            );
        }
    }

    /// Draw one text label through the shared, byte-bounded cache. Callers
    /// pass already-offset coordinates in output-local space.
    pub(crate) fn push_label(
        &mut self,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
        text: &str,
        at: (i32, i32),
        scale: u32,
        color: [u8; 4],
    ) {
        let (x, y) = at;
        // Cache by content/scale/colour so static chrome does not rasterize
        // and upload the same bitmap every frame. The cache is byte-bounded: a
        // changing status line or a scrolling explorer must not retain every
        // label it has ever drawn.
        let base_scale = scale & 0xff;
        let bold = Self::text_bold(scale);
        let spacing = Self::text_spacing(scale);
        let key = (
            text.to_string(),
            scale,
            color,
            bold,
            (spacing * 1000.0) as i32,
        );
        let mut uncached = None;
        if !self.text_cache.contains(&key) {
            let bitmap = match self.font.as_ref() {
                Some(font) => font.render(
                    text,
                    crate::app::App::text_px(base_scale),
                    color,
                    bold,
                    spacing,
                ),
                None => TextBitmap::render(text, base_scale.max(1), color),
            };
            let bytes = bitmap.pixels().len() + text.len() + 64;
            let buffer = MemoryRenderBuffer::from_slice(
                bitmap.pixels(),
                Fourcc::Abgr8888,
                (bitmap.width() as i32, bitmap.height() as i32),
                1,
                Transform::Normal,
                None,
            );
            self.text_rasterizations += 1;
            if bytes <= self.text_cache.budget() {
                self.text_cache.insert(key.clone(), buffer, bytes);
            } else {
                // An oversized label is still drawn this frame; it simply is
                // not retained. Dropping the element would be a visual bug,
                // not a saving.
                uncached = Some(buffer);
            }
        }
        if let Some(buffer) = self.text_cache.get(&key).or(uncached.as_ref()) {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (x as f64, y as f64),
                buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }
    }

    fn compose_ime(
        &mut self,
        ctx: &FrameContext,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
    ) {
        let output = ctx.output;
        let offset = ctx.offset;
        let visible = &ctx.visible;
        let rects = &ctx.content_rects;
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
    }

    fn compose_cursor(
        &mut self,
        ctx: &FrameContext,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
    ) {
        let output = ctx.output;
        let primary = ctx.primary;
        let offset = ctx.offset;
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
    }
}
