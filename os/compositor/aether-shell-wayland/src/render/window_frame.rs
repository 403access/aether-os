//! Compositor-drawn Water frame around every client toplevel.
//!
//! Client apps only own their content surface. The title bar, traffic lights,
//! app mark, title and focus emphasis are drawn by the shell, so a foot
//! terminal looks like the first-party surfaces instead of arriving with its
//! own window chrome. Geometry mirrors `.os-titlebar`, `.os-lights` and
//! `.os-app-mark` in `design-system/os/os.css`.

use aether_geometry::Rect as AetherRect;
use aether_shell::theme::ThemeTokens;
use aether_shell::WindowId;
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;

use crate::app::{App, WINDOW_FRAME_HEIGHT};
use crate::render::element::AetherRenderElement;
use crate::render::helpers::truncate;

/// `.os-window { border-radius: var(--ds-radius-lg) }`.
const TITLE_RADIUS: u32 = 14;
/// `.os-app-mark` is a 23 px tile; `.os-app-mark .icon` is 13 px.
const APP_MARK: u32 = 23;
const APP_MARK_GLYPH: u32 = 13;
/// `.os-light` colours, left to right.
const LIGHT_COLORS: [[u8; 4]; 3] = [
    [0xff, 0x8a, 0x92, 255],
    [0xff, 0xd2, 0x7a, 255],
    [0x5e, 0xf2, 0xb0, 255],
];

impl App {
    /// Draw the frame above one client surface. Called from the window
    /// composition loop in bottom-to-top order so overlap follows the scene.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_window_frame(
        &mut self,
        renderer: &mut GlesRenderer,
        elements: &mut Vec<AetherRenderElement>,
        tokens: &ThemeTokens,
        offset: (i32, i32),
        id: WindowId,
        rect: AetherRect,
        title: &str,
        focused: bool,
    ) {
        let x = rect.x - offset.0;
        let y = rect.y - offset.1;
        let width = rect.width.max(1);
        let height = WINDOW_FRAME_HEIGHT as u32;
        let hover = self.pointer_in(rect);
        let effects = self.window_effects(rect, TITLE_RADIUS, focused, hover);
        if let Some((buffer, ex, ey)) = effects.shadow {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                ((ex - offset.0) as f64, (ey - offset.1) as f64),
                &buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        // Title bar fill: focused windows are brighter, unfocused ones sit
        // back. `.os-window:not(.is-focused) .os-titlebar { opacity: .78 }`.
        let fill = if focused {
            [10, 51, 66, 255]
        } else {
            [8, 42, 55, 255]
        };
        if let Some(buffer) = self.chrome_round_top(width, height, TITLE_RADIUS, fill) {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (x as f64, y as f64),
                &buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        // Focused windows carry the accent rim along the top edge and down the
        // sides; unfocused windows use the neutral line.
        let highlight =
            SolidColorBuffer::new((width.saturating_sub(28) as i32, 1), [1.0, 1.0, 1.0, 0.07]);
        elements.push(AetherRenderElement::from(
            SolidColorRenderElement::from_buffer(
                &highlight,
                (x + 14, y + 1),
                1.0f64,
                1.0,
                Kind::Unspecified,
            ),
        ));
        let rim = if hover {
            tokens.accent(215)
        } else if focused {
            tokens.accent(170)
        } else {
            tokens.border(56)
        };
        let rim_f = rim.map(|value| value as f32 / 255.0);
        let rule_buffer = SolidColorBuffer::new((width as i32, 1), rim_f);
        elements.push(AetherRenderElement::from(
            SolidColorRenderElement::from_buffer(
                &rule_buffer,
                (x, y + height as i32 - 1),
                1.0f64,
                1.0,
                Kind::Unspecified,
            ),
        ));
        let side = SolidColorBuffer::new((1, height as i32), rim_f);
        for side_x in [x, x + width as i32 - 1] {
            elements.push(AetherRenderElement::from(
                SolidColorRenderElement::from_buffer(
                    &side,
                    (side_x, y),
                    1.0f64,
                    1.0,
                    Kind::Unspecified,
                ),
            ));
        }

        // Traffic lights. `.os-light` is an 11 px disc with a dark hairline.
        if let Some(lights) = self.window_light_rects(id) {
            for (light, color) in lights.iter().zip(LIGHT_COLORS) {
                let dot = self.chrome_round(light.width, light.height, light.width / 2, color);
                if let Some(buffer) = dot {
                    if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        ((light.x - offset.0) as f64, (light.y - offset.1) as f64),
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
        }

        // App mark: 23 px rounded tile, 30% accent border over a 14% wash.
        let mark_x = x + 66;
        let mark_y = y + (height as i32 - APP_MARK as i32) / 2;
        let accent = tokens.accent(255);
        if let Some(buffer) =
            self.chrome_round(APP_MARK, APP_MARK, 5, [accent[0], accent[1], accent[2], 36])
        {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (mark_x as f64, mark_y as f64),
                &buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }
        if let Some(buffer) = self.chrome_icon(app_icon(title), APP_MARK_GLYPH, accent) {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (
                    (mark_x + (APP_MARK as i32 - APP_MARK_GLYPH as i32) / 2) as f64,
                    (mark_y + (APP_MARK as i32 - APP_MARK_GLYPH as i32) / 2) as f64,
                ),
                &buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        // Title and subtitle: `<b>title</b><span class="os-titlebar__sub">sub</span>`.
        let text_x = mark_x + APP_MARK as i32 + 8;
        let text_y = y + (height as i32 - 14) / 2;
        let subtitle = app_subtitle(title);
        let subtitle_width = subtitle
            .map(|sub| self.text_width(sub, 0) as i32 + 8)
            .unwrap_or(0);
        let available = (x + width as i32 - 12 - subtitle_width - text_x).max(28);
        let mut label = title.to_string();
        if self.text_width(&label, 1) as i32 > available {
            let mut limit = title.chars().count();
            while limit > 4 && self.text_width(&truncate(title, limit), 1) as i32 > available {
                limit -= 1;
            }
            label = truncate(title, limit);
        }
        self.push_label(
            renderer,
            elements,
            &label,
            (text_x, text_y),
            0x100 | 10,
            if focused { tokens.text } else { tokens.soft },
        );
        if let Some(subtitle) = subtitle {
            let sub_x = text_x + self.text_width(&label, 1) as i32 + 8;
            self.push_label(renderer, elements, subtitle, (sub_x, text_y), 9, tokens.dim);
        }

        if let Some((buffer, ex, ey)) = effects.wave {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                ((ex - offset.0) as f64, (ey - offset.1) as f64),
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
}

/// Pick a design-system mark for a client. Titles are the only metadata the
/// compositor has today, so this is intentionally a coarse mapping.
fn app_icon(title: &str) -> &'static str {
    let title = title.to_ascii_lowercase();
    if title.contains("calendar") {
        "calendar"
    } else if title.contains("term") {
        "terminal"
    } else if title.contains("browser")
        || title.contains("firefox")
        || title.contains("chrom")
        || title.contains("webkit")
    {
        "grid"
    } else if title.contains("file") || title.contains("explor") {
        "folder"
    } else {
        "grid"
    }
}

/// The dim second line in `.os-titlebar__title`. First-party windows describe
/// themselves; a bare client title has no subtitle.
fn app_subtitle(title: &str) -> Option<&'static str> {
    let title = title.to_ascii_lowercase();
    if title.contains("calendar") {
        Some("Month at a glance")
    } else if title.contains("term") {
        Some("Shell session")
    } else if title.contains("task") {
        Some("Live system processes")
    } else if title.contains("file") || title.contains("explor") {
        Some("Two-pane filesystem explorer")
    } else if title.contains("water settings") {
        Some("Water theme and effects")
    } else if title.contains("audio") {
        Some("Outputs, inputs and presets")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{app_icon, app_subtitle};

    #[test]
    fn terminal_titles_get_the_terminal_mark() {
        assert_eq!(app_icon("Wayland Terminal"), "terminal");
        assert_eq!(app_icon("bash"), "grid");
        assert_eq!(app_icon("Files"), "folder");
        assert_eq!(app_icon("Calendar"), "calendar");
    }

    #[test]
    fn first_party_windows_have_a_subtitle() {
        assert_eq!(app_subtitle("Calendar"), Some("Month at a glance"));
        assert_eq!(app_subtitle("Wayland Terminal"), Some("Shell session"));
        assert_eq!(app_subtitle("bash"), None);
    }
}
