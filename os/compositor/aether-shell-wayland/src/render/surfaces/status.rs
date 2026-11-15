//! Status surface composition.

use std::sync::Arc;

use aether_shell::WORKSPACE_INDICATOR_BAR_HEIGHT;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

impl App {
    pub(crate) fn compose_status_chrome(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let chrome_output = ctx.chrome_output;
        let task_here = ctx.flags.task;
        let explorer_here = ctx.flags.explorer;
        let chat_here = ctx.flags.chat;
        let audio_here = ctx.flags.audio;
        let themes_here = ctx.flags.themes;
        let remote_here = ctx.flags.remote;
        let shortcuts_here = ctx.flags.shortcuts;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if !task_here && !explorer_here && self.shell.fullscreen_window_on(chrome_output).is_none()
        {
            if !chat_here
                && !audio_here
                && !themes_here
                && !remote_here
                && !shortcuts_here
                && self.shell.overlay().is_none()
            {
                let bar =
                    crate::app::overlays::topbar::layout(self.shell.output_bounds(chrome_output));
                if let Some(buffer) =
                    self.chrome_round(bar.rect.width, bar.rect.height, 10, tokens.surface_a(225))
                {
                    panels.push((buffer, bar.rect.x, bar.rect.y));
                }
                for (action, item) in &bar.items {
                    if let Some(buffer) =
                        self.chrome_round(item.width, item.height, 6, tokens.base_a(220))
                    {
                        panels.push((buffer, item.x, item.y));
                    }
                    if bar.compact {
                        display_icons.push((
                            action.icon().into(),
                            16,
                            tokens.soft,
                            item.x + (item.width as i32 - 16) / 2,
                            item.y + 6,
                        ));
                    } else {
                        display_icons.push((
                            action.icon().into(),
                            15,
                            tokens.accent(255),
                            item.x + 9,
                            item.y + 7,
                        ));
                        labels.push((
                            action.label().into(),
                            item.x + 29,
                            item.y + 8,
                            0,
                            tokens.text_soft,
                        ));
                    }
                }
            }
            if self.status_visible() && !chat_here {
                let bounds = self.shell.output_bounds(chrome_output);
                let text = self.status().label();
                let width = self.text_width(&text, 0) as i32 + 42;
                let x = bounds.x + bounds.width as i32 - width - 12;
                if let Some(buffer) = self.chrome_round(width as u32, 26, 13, tokens.surface_a(226))
                {
                    panels.push((buffer, x, bounds.y + 10));
                }
                if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
                    panels.push((buffer, x + 12, bounds.y + 20));
                }
                labels.push((text, x + 24, bounds.y + 15, 0, tokens.text_soft));
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
                        tokens.accent(255)
                    } else {
                        tokens.accent(46)
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
                        tokens.accent(255)
                    } else {
                        tokens.dim
                    },
                ));
            }
        }

        RenderChunk {
            chrome: Vec::new(),
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}
