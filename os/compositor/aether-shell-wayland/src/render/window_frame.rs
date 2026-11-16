//! Compositor-drawn Aether frame around every client toplevel.
//!
//! Client apps only own their content surface. The title bar, app mark, title,
//! focus emphasis and close affordance are drawn by the shell, so a foot
//! terminal looks like the first-party surfaces instead of arriving with its
//! own window chrome.

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

const TITLE_RADIUS: u32 = 8;
const CLOSE_RADIUS: u32 = 6;

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

        let fill = if focused {
            tokens.header
        } else {
            tokens.surface
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

        // A thin accent rule marks the focused window without adding a full
        // border around the client content.
        let rule = tokens
            .accent(if focused { 120 } else { 36 })
            .map(|value| value as f32 / 255.0);
        let rule_buffer = SolidColorBuffer::new((width as i32, 1), rule);
        elements.push(AetherRenderElement::from(
            SolidColorRenderElement::from_buffer(
                &rule_buffer,
                (x, y + height as i32 - 1),
                1.0f64,
                1.0,
                Kind::Unspecified,
            ),
        ));

        let icon = window_icon(title);
        if let Some(buffer) = self.chrome_icon(
            icon,
            15,
            if focused {
                tokens.accent(255)
            } else {
                tokens.dim
            },
        ) {
            if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                ((x + 10) as f64, (y + 9) as f64),
                &buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(AetherRenderElement::from(element));
            }
        }

        let available = (width as i32 - 78).max(28);
        let mut label = title.to_string();
        if self.text_width(&label, 0) as i32 > available {
            let mut limit = title.chars().count();
            while limit > 4 && self.text_width(&truncate(title, limit), 0) as i32 > available {
                limit -= 1;
            }
            label = truncate(title, limit);
        }
        self.push_label(
            renderer,
            elements,
            &label,
            (x + 34, y + 11),
            0,
            if focused { tokens.text } else { tokens.soft },
        );

        if let Some(close) = self.window_close_rect(id) {
            let close_x = close.x - offset.0;
            let close_y = close.y - offset.1;
            if let Some(buffer) =
                self.chrome_round(close.width, close.height, CLOSE_RADIUS, tokens.base_a(200))
            {
                if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    (close_x as f64, close_y as f64),
                    &buffer,
                    None,
                    None,
                    None,
                    Kind::Unspecified,
                ) {
                    elements.push(AetherRenderElement::from(element));
                }
                if let Some(icon) = self.chrome_icon("close", 12, tokens.dim) {
                    if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        ((close_x + 4) as f64, (close_y + 4) as f64),
                        &icon,
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
    }
}

/// Pick a design-system mark for a client. Titles are the only metadata the
/// compositor has today, so this is intentionally a coarse mapping.
fn window_icon(title: &str) -> &'static str {
    let title = title.to_ascii_lowercase();
    if title.contains("term") {
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

#[cfg(test)]
mod tests {
    use super::window_icon;

    #[test]
    fn terminal_titles_get_the_terminal_mark() {
        assert_eq!(window_icon("Wayland Terminal"), "terminal");
        assert_eq!(window_icon("bash"), "grid");
        assert_eq!(window_icon("Files"), "folder");
    }
}
