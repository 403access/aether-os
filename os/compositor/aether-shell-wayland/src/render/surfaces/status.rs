//! Menu bar composition: the Water system bar, its popovers and the global
//! status line that used to live in a separate top dock.

use std::sync::Arc;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::overlays::menubar::{menu_layout, MenuId, Menubar, TrayAction, MENUBAR_HEIGHT};
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

/// Bar fill: `color-mix(in srgb, var(--ds-bg-soft) 72%, transparent)`.
const BAR_FILL: [u8; 4] = [5, 37, 48, 184];
/// Popover fill: `--ds-popover` with a hairline border.
const POPOVER_FILL: [u8; 4] = [13, 61, 75, 250];
const RULE: [u8; 4] = [127, 230, 255, 38];

impl App {
    pub(crate) fn compose_status_chrome(&mut self, ctx: &FrameContext) -> RenderChunk {
        let chrome_output = ctx.chrome_output;
        if self.shell.fullscreen_window_on(chrome_output).is_some() {
            return RenderChunk::default();
        }
        let tokens = ctx.tokens;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();
        let mut chrome: Vec<(aether_geometry::Rect, [f32; 4])> = Vec::new();

        let bar = self.menubar(chrome_output);
        let x = bar.rect.x;
        let y = bar.rect.y;

        // Bar surface plus its bottom hairline.
        if let Some(buffer) = self.chrome_round(bar.rect.width, MENUBAR_HEIGHT, 0, BAR_FILL) {
            panels.push((buffer, x, y));
        }
        chrome.push((
            aether_geometry::Rect::new(x, y + MENUBAR_HEIGHT as i32 - 1, bar.rect.width, 1),
            [
                RULE[0] as f32 / 255.0,
                RULE[1] as f32 / 255.0,
                RULE[2] as f32 / 255.0,
                0.15,
            ],
        ));

        // Brand mark: two overlapping glass panels with a check, like `.os-logo`.
        display_icons.push((
            "water".into(),
            18,
            tokens.accent(255),
            bar.logo.x + 5,
            y + 8,
        ));

        // Application title (`.os-menubar__app`), 12.5 px semibold.
        labels.push((
            bar.app_title.clone(),
            bar.app.x + 2,
            y + 10,
            0x100 | 10,
            tokens.text,
        ));

        // Application menus.
        for (id, rect) in &bar.menus {
            let open = self.menu_open == Some(*id);
            if open {
                if let Some(buffer) =
                    self.chrome_round(rect.width, rect.height, 5, [255, 255, 255, 22])
                {
                    panels.push((buffer, rect.x, rect.y));
                }
            }
            labels.push((
                id.label().into(),
                rect.x + 9,
                y + 11,
                10,
                if open { tokens.text } else { tokens.soft },
            ));
        }

        // Sync pill: 50% surface fill with a hairline border.
        if let Some(buffer) =
            self.chrome_round(bar.sync.width, bar.sync.height, 5, tokens.surface_a(128))
        {
            panels.push((buffer, bar.sync.x, bar.sync.y));
        }
        if let Some(buffer) =
            self.chrome_outline(bar.sync.width, bar.sync.height, 5, tokens.border(38))
        {
            panels.push((buffer, bar.sync.x, bar.sync.y));
        }
        if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
            panels.push((buffer, bar.sync.x + 8, bar.sync.y + 10));
        }
        labels.push((
            bar.sync_label.clone(),
            bar.sync.x + 20,
            y + 11,
            9,
            tokens.soft,
        ));

        // Resource meters.
        if let Some(buffer) = self.chrome_round(
            bar.resources.width,
            bar.resources.height,
            5,
            tokens.surface_a(115),
        ) {
            panels.push((buffer, bar.resources.x, bar.resources.y));
        }
        for (index, (meter, rect)) in bar.meters.iter().enumerate() {
            if index > 0 {
                chrome.push((
                    aether_geometry::Rect::new(rect.x - 4, y + 6, 1, 14),
                    [
                        RULE[0] as f32 / 255.0,
                        RULE[1] as f32 / 255.0,
                        RULE[2] as f32 / 255.0,
                        RULE[3] as f32 / 255.0,
                    ],
                ));
            }
            let label_x = rect.x + 8;
            labels.push((meter.label.clone(), label_x, y + 14, 7, tokens.dim));
            let meter_x = label_x + self.text_width(&meter.label, 6) as i32 + 4;
            if let Some(buffer) = self.chrome_round(20, 3, 2, [255, 255, 255, 26]) {
                panels.push((buffer, meter_x, y + 18));
            }
            let fill = (20.0 * meter.fraction.clamp(0.0, 1.0)).round() as u32;
            if fill > 0 {
                let color = match index {
                    0 => tokens.accent(255),
                    1 => [0xc3, 0xa8, 0xff, 255],
                    _ => tokens.green,
                };
                if let Some(buffer) = self.chrome_round(fill.max(2), 3, 2, color) {
                    panels.push((buffer, meter_x, y + 18));
                }
            }
            labels.push((meter.value.clone(), meter_x + 24, y + 14, 8, tokens.soft));
        }

        // Tray glyphs.
        for (action, rect) in &bar.tray {
            let (glyph, width) = match action {
                TrayAction::Sync | TrayAction::Resources | TrayAction::Clock => continue,
                TrayAction::Battery => (Some("battery"), rect.width),
                TrayAction::Gallery => (Some("grid"), rect.width),
                TrayAction::Appearance => (Some("moon"), rect.width),
                other => (Some(other.icon()), rect.width),
            };
            let Some(glyph) = glyph else { continue };
            let icon_x = if *action == TrayAction::Battery {
                rect.x + 8
            } else {
                rect.x + (width as i32 - 15) / 2
            };
            display_icons.push((glyph.into(), 15, tokens.text_soft, icon_x, y + 5));
        }
        if let Some((_, battery)) = bar
            .tray
            .iter()
            .find(|(action, _)| *action == TrayAction::Battery)
        {
            labels.push(("78%".into(), battery.x + 29, y + 12, 9, tokens.text_soft));
        }

        // Clock: dim date, bright time.
        let clock_date_x = bar.clock.x + 8;
        labels.push((bar.clock_date.clone(), clock_date_x, y + 11, 9, tokens.dim));
        let time_x = clock_date_x + self.text_width(&bar.clock_date, 0) as i32 + 7;
        labels.push((
            bar.clock_time.clone(),
            time_x,
            y + 11,
            0x100 | 9,
            tokens.text,
        ));

        // Open popover.
        if let Some(menu) = self.menu_open {
            self.compose_menu_popover(
                ctx,
                menu,
                &bar,
                &mut labels,
                &mut panels,
                &mut display_icons,
            );
        }

        RenderChunk {
            chrome,
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }

    fn compose_menu_popover(
        &mut self,
        ctx: &FrameContext,
        menu: MenuId,
        bar: &Menubar,
        labels: &mut Vec<(String, i32, i32, u32, [u8; 4])>,
        panels: &mut Vec<(Arc<MemoryRenderBuffer>, i32, i32)>,
        display_icons: &mut Vec<(String, u32, [u8; 4], i32, i32)>,
    ) {
        let tokens = ctx.tokens;
        let measure = |text: &str, scale: u32| self.text_width(text, scale);
        let panel = menu_layout(bar, menu, &measure);
        if let Some(buffer) =
            self.chrome_round(panel.rect.width, panel.rect.height, 10, POPOVER_FILL)
        {
            panels.push((buffer, panel.rect.x, panel.rect.y));
        }
        for (entry, rect) in &panel.items {
            if entry.separator_before {
                let rule = aether_geometry::Rect::new(
                    rect.x + 8,
                    rect.y - 4,
                    rect.width.saturating_sub(16),
                    1,
                );
                if let Some(buffer) = self.chrome_round(rule.width, 1, 0, RULE) {
                    panels.push((buffer, rule.x, rule.y));
                }
            }
            display_icons.push((entry.icon.into(), 15, tokens.dim, rect.x + 8, rect.y + 7));
            labels.push((
                entry.label.into(),
                rect.x + 31,
                rect.y + 8,
                11,
                tokens.text_soft,
            ));
        }
    }
}
