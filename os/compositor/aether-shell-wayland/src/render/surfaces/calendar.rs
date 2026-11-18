//! Calendar app composition: window chrome, month grid and agenda.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::overlays::calendar::TITLEBAR_HEIGHT;
use crate::app::overlays::menubar::WEEKDAY_HEADERS;
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};

const WINDOW_FILL: [u8; 4] = [7, 39, 51, 235];
const TITLEBAR_FILL: [u8; 4] = [10, 51, 66, 255];
const AGENDA_FILL: [u8; 4] = [9, 47, 60, 126];
const RULE: [u8; 4] = [127, 230, 255, 38];
const EVENT_FILL: [u8; 4] = [5, 37, 48, 153];
const LIGHTS: [[u8; 4]; 3] = [
    [0xff, 0x8a, 0x92, 255],
    [0xff, 0xd2, 0x7a, 255],
    [0x5e, 0xf2, 0xb0, 255],
];

impl App {
    pub(crate) fn compose_calendar(&mut self, ctx: &FrameContext) -> RenderChunk {
        if self.calendar.is_none() {
            return RenderChunk::default();
        }
        let tokens = ctx.tokens;
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();

        let rect = self.calendar_rect();
        let month = self.calendar_month_rect();
        let agenda = self.calendar_agenda_rect();

        // Window body and title bar.
        if let Some(buffer) = self.chrome_round(rect.width, rect.height, 14, WINDOW_FILL) {
            panels.push((buffer, rect.x, rect.y));
        }
        if let Some(buffer) = self.chrome_round_top(rect.width, TITLEBAR_HEIGHT as u32, 14, TITLEBAR_FILL)
        {
            panels.push((buffer, rect.x, rect.y));
        }
        chrome.push((
            AetherRect::new(rect.x, rect.y + TITLEBAR_HEIGHT - 1, rect.width, 1),
            [RULE[0] as f32 / 255.0, RULE[1] as f32 / 255.0, RULE[2] as f32 / 255.0, 0.82],
        ));

        // Traffic lights.
        for (light, color) in self.calendar_light_rects().iter().zip(LIGHTS) {
            if let Some(buffer) = self.chrome_round(light.width, light.height, light.width / 2, color) {
                panels.push((buffer, light.x, light.y));
            }
        }

        // App mark and titles.
        let mark_x = rect.x + 66;
        let mark_y = rect.y + (TITLEBAR_HEIGHT - 23) / 2;
        let accent = tokens.accent(255);
        if let Some(buffer) = self.chrome_round(23, 23, 5, [accent[0], accent[1], accent[2], 36]) {
            panels.push((buffer, mark_x, mark_y));
        }
        display_icons.push(("calendar".into(), 13, accent, mark_x + 5, mark_y + 5));
        labels.push(("Calendar".into(), mark_x + 31, rect.y + 13, 0x100, tokens.text));
        labels.push((
            "Month and agenda".into(),
            mark_x + 31 + self.text_width("Calendar", 1) as i32 + 8,
            rect.y + 13,
            0,
            tokens.dim,
        ));

        // Badge on the right: event count for today.
        let event_count = self
            .calendar
            .as_ref()
            .map(|state| state.agenda().len())
            .unwrap_or(0);
        let badge = if event_count == 1 {
            "1 event".to_string()
        } else {
            format!("{event_count} events")
        };
        let badge_w = 8 + 6 + 6 + self.text_width(&badge, 0) + 8;
        let badge_x = rect.x + rect.width as i32 - 12 - badge_w as i32;
        if let Some(buffer) = self.chrome_round(badge_w, 22, 11, [255, 255, 255, 20]) {
            panels.push((buffer, badge_x, rect.y + 9));
        }
        if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.soft) {
            panels.push((buffer, badge_x + 8, rect.y + 17));
        }
        labels.push((badge, badge_x + 20, rect.y + 14, 0, tokens.text_soft));

        // Month column divider.
        chrome.push((
            AetherRect::new(month.right() as i32 - 1, month.y, 1, month.height),
            [RULE[0] as f32 / 255.0, RULE[1] as f32 / 255.0, RULE[2] as f32 / 255.0, RULE[3] as f32 / 255.0],
        ));

        let (view_label, selected_label, clock_time, cells, events) = {
            let Some(state) = self.calendar.as_ref() else {
                return RenderChunk::default();
            };
            (
                state.view().month_label(),
                state.selected().long_date(),
                state.today().time_label(),
                state.cells(),
                state.agenda(),
            )
        };
        let content = month.inset(14);
        labels.push((
            view_label,
            content.x + 32,
            content.y + 4,
            3,
            tokens.text,
        ));
        // Toolbar buttons.
        for (rect, glyph) in [
            (self.calendar_prev_rect(), "chevron-left"),
            (self.calendar_next_rect(), "chevron-right"),
        ] {
            display_icons.push((glyph.into(), 13, tokens.text_soft, rect.x + 6, rect.y + 6));
        }
        let today_rect = self.calendar_today_rect();
        if let Some(buffer) = self.chrome_round(today_rect.width, today_rect.height, 5, [255, 255, 255, 10])
        {
            panels.push((buffer, today_rect.x, today_rect.y));
        }
        labels.push((
            "Today".into(),
            today_rect.x + 9,
            today_rect.y + 7,
            4,
            tokens.text_soft,
        ));

        // Weekday headers.
        let grid = self.calendar_grid_rect();
        let cell_width = grid.width as i32 / 7;
        for (index, name) in WEEKDAY_HEADERS.iter().enumerate() {
            let width = self.text_width(name, 4) as i32;
            labels.push((
                name.to_uppercase(),
                content.x + index as i32 * cell_width + (cell_width - width) / 2,
                grid.y - 19,
                4,
                tokens.dim,
            ));
        }

        // Day cells.
        {
            for (index, cell) in cells.iter().enumerate() {
                let cell_rect = self.calendar_cell_rect(index);
                if cell.today {
                    if let Some(buffer) =
                        self.chrome_round(cell_rect.width, cell_rect.height, 8, tokens.accent(255))
                    {
                        panels.push((buffer, cell_rect.x, cell_rect.y));
                    }
                } else if cell.selected {
                    if let Some(buffer) = self.chrome_round(
                        cell_rect.width,
                        cell_rect.height,
                        8,
                        [tokens.accent(255)[0], tokens.accent(255)[1], tokens.accent(255)[2], 32],
                    ) {
                        panels.push((buffer, cell_rect.x, cell_rect.y));
                    }
                }
                let color = if cell.today {
                    [3, 25, 34, 255]
                } else if cell.in_month {
                    tokens.text
                } else {
                    tokens.dim
                };
                let day = cell.date.day.to_string();
                let width = self.text_width(&day, 1) as i32;
                labels.push((
                    day,
                    cell_rect.x + (cell_rect.width as i32 - width) / 2,
                    cell_rect.y + (cell_rect.height as i32 - 14) / 2,
                    if cell.today { 0x100 } else { 0 },
                    color,
                ));
                if cell.events > 0 {
                    let dot = if cell.today {
                        [3, 25, 34, 255]
                    } else {
                        tokens.accent(255)
                    };
                    if let Some(buffer) = self.chrome_round(4, 4, 2, dot) {
                        panels.push((buffer, cell_rect.x + cell_rect.width as i32 / 2 - 2, cell_rect.bottom() as i32 - 9));
                    }
                }
            }
        }

        // Agenda column.
        if let Some(buffer) = self.chrome_round(agenda.width, agenda.height, 0, AGENDA_FILL) {
            panels.push((buffer, agenda.x, agenda.y));
        }
        let pad = agenda.inset(14);
        labels.push((
            selected_label,
            pad.x,
            pad.y + 2,
            0x100,
            tokens.text,
        ));
        let time_w = self.text_width(&clock_time, 3) as i32;
        labels.push((
            clock_time.clone(),
            pad.right() as i32 - time_w,
            pad.y,
            3,
            tokens.text_soft,
        ));
        chrome.push((
            AetherRect::new(pad.x, pad.y + 28, pad.width, 1),
            [RULE[0] as f32 / 255.0, RULE[1] as f32 / 255.0, RULE[2] as f32 / 255.0, RULE[3] as f32 / 255.0],
        ));

        if events.is_empty() {
            labels.push((
                "Nothing scheduled.".into(),
                pad.x,
                pad.y + 48,
                0,
                tokens.dim,
            ));
        }
        for (index, event) in events.iter().enumerate() {
            let top = pad.y + 38 + index as i32 * 59;
            if let Some(buffer) = self.chrome_round(pad.width, 52, 8, EVENT_FILL) {
                panels.push((buffer, pad.x, top));
            }
            labels.push((event.time.into(), pad.x + 9, top + 8, 5, tokens.accent(255)));
            labels.push((event.title.into(), pad.x + 49, top + 7, 0x100, tokens.text));
            labels.push((event.detail.into(), pad.x + 49, top + 24, 5, tokens.dim));
        }

        // Frame rim: `.os-window` draws a 1 px accent border on focus.
        let rim = [accent[0] as f32 / 255.0, accent[1] as f32 / 255.0, accent[2] as f32 / 255.0, 0.47];
        chrome.push((AetherRect::new(rect.x, rect.y, rect.width, 1), rim));
        chrome.push((
            AetherRect::new(rect.x, rect.bottom() as i32 - 1, rect.width, 1),
            rim,
        ));
        chrome.push((AetherRect::new(rect.x, rect.y, 1, rect.height), rim));
        chrome.push((
            AetherRect::new(rect.right() as i32 - 1, rect.y, 1, rect.height),
            rim,
        ));

        RenderChunk {
            chrome,
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}

/// Kept for tests: the calendar window matches the demo's 700 x 590.
#[cfg(test)]
fn demo_size() -> (u32, u32) {
    (
        crate::app::overlays::calendar::CALENDAR_WIDTH,
        crate::app::overlays::calendar::CALENDAR_HEIGHT,
    )
}

#[cfg(test)]
mod tests {
    use super::demo_size;

    #[test]
    fn window_matches_the_demo() {
        assert_eq!(demo_size(), (700, 590));
    }
}