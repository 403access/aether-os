//! Calendar app: month grid, agenda and selection.
//!
//! A first-party window like the Task Manager. The grid is always six rows of
//! seven cells so the layout never jumps between months, matching
//! `.os-calendar-app` in the Water OS demo.

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use smithay::input::keyboard::{keysyms, Keysym};

use super::super::App;
use super::menubar::Clock;

/// A seeded agenda entry. `day_offset` is relative to today so the demo data
/// follows the clock instead of going stale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarEvent {
    pub day_offset: i32,
    pub time: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
}

/// The agenda the demo ships with: a snapshot verification, a radio session,
/// an index rebuild and a backup window.
pub const EVENTS: &[CalendarEvent] = &[
    CalendarEvent {
        day_offset: 0,
        time: "09:30",
        title: "Snapshot verification",
        detail: "Jottacloud Archive · 45 min",
    },
    CalendarEvent {
        day_offset: 0,
        time: "14:00",
        title: "Tide Radio session",
        detail: "Deep Current · 1 hr",
    },
    CalendarEvent {
        day_offset: -3,
        time: "11:15",
        title: "Index rebuild",
        detail: "aether-index · 10k rows",
    },
    CalendarEvent {
        day_offset: 2,
        time: "08:00",
        title: "Backup window",
        detail: "Tank ⇄ Jottacloud · 2 hr",
    },
    CalendarEvent {
        day_offset: 4,
        time: "16:30",
        title: "Remote review",
        detail: "Kestrel display",
    },
    CalendarEvent {
        day_offset: 7,
        time: "10:00",
        title: "Agent policy audit",
        detail: "Sandboxes · 30 min",
    },
];

pub struct CalendarState {
    view: Clock,
    selected: Clock,
    today: Clock,
}

impl CalendarState {
    pub fn new(today: Clock) -> Self {
        Self {
            view: today.month_start(),
            selected: today,
            today,
        }
    }

    pub fn view(&self) -> Clock {
        self.view
    }

    pub fn selected(&self) -> Clock {
        self.selected
    }

    pub fn today(&self) -> Clock {
        self.today
    }

    pub fn previous_month(&mut self) {
        self.view = self.view.add_months(-1);
    }

    pub fn next_month(&mut self) {
        self.view = self.view.add_months(1);
    }

    pub fn go_today(&mut self) {
        self.today = Clock::now();
        self.view = self.today.month_start();
        self.selected = self.today;
    }

    pub fn select(&mut self, day: Clock) {
        self.selected = day;
    }

    pub fn select_offset(&mut self, days: i32) {
        self.selected = self.selected.add_days(days);
        self.view = self.selected.month_start();
    }

    /// Six rows of seven cells covering the visible month.
    pub fn cells(&self) -> Vec<CalendarCell> {
        let first = self.view.first_weekday() as i32;
        let days = self.view.days_in_month() as i32;
        let start = self.view.add_days(-first);
        (0..42)
            .map(|index| {
                let date = start.add_days(index);
                CalendarCell {
                    date,
                    in_month: index >= first && index < first + days,
                    today: date.day_key() == self.today.day_key(),
                    selected: date.day_key() == self.selected.day_key(),
                    events: events_on(date, self.today).len(),
                }
            })
            .collect()
    }

    pub fn agenda(&self) -> Vec<CalendarEvent> {
        events_on(self.selected, self.today)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn november() -> Clock {
        Clock {
            year: 2026,
            month: 11,
            day: 18,
            weekday: 0,
            hour: 16,
            minute: 47,
            second: 0,
        }
        .normalized()
    }

    #[test]
    fn grid_is_six_rows_of_seven_cells() {
        let state = CalendarState::new(november());
        let cells = state.cells();
        assert_eq!(cells.len(), 42);
        // 1 November 2026 is a Sunday, so the grid starts exactly on the 1st.
        assert_eq!(cells[0].date.day, 1);
        assert!(cells[0].in_month);
        assert_eq!(cells[29].date.day, 30);
        assert!(!cells[30].in_month);
    }

    #[test]
    fn today_and_selection_are_marked() {
        let state = CalendarState::new(november());
        let cells = state.cells();
        let today = cells.iter().find(|cell| cell.today).expect("today cell");
        assert_eq!(today.date.day, 18);
        assert!(today.selected);
        assert!(today.in_month);
    }

    #[test]
    fn agenda_is_seeded_for_today() {
        let state = CalendarState::new(november());
        let agenda = state.agenda();
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0].time, "09:30");
        assert_eq!(agenda[1].time, "14:00");
    }

    #[test]
    fn navigating_months_moves_the_grid() {
        let mut state = CalendarState::new(november());
        state.next_month();
        assert_eq!(state.view().month_label(), "December 2026");
        state.previous_month();
        state.previous_month();
        assert_eq!(state.view().month_label(), "October 2026");
    }

    #[test]
    fn selecting_a_day_updates_the_agenda() {
        let mut state = CalendarState::new(november());
        state.select(november().add_days(2));
        assert_eq!(state.selected().day, 20);
        let agenda = state.agenda();
        assert_eq!(agenda.len(), 1);
        assert_eq!(agenda[0].title, "Backup window");
    }

    #[test]
    fn keyboard_offsets_cross_month_boundaries() {
        let mut state = CalendarState::new(november());
        state.select_offset(20);
        assert_eq!(state.selected().day_key(), "2026-12-08");
        assert_eq!(state.view().month_label(), "December 2026");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarCell {
    pub date: Clock,
    pub in_month: bool,
    pub today: bool,
    pub selected: bool,
    pub events: usize,
}

fn events_on(date: Clock, today: Clock) -> Vec<CalendarEvent> {
    let key = date.day_key();
    EVENTS
        .iter()
        .filter(|event| today.add_days(event.day_offset).day_key() == key)
        .copied()
        .collect()
}

/// Chrome geometry. The window is 700 x 590, centered like `.os-window--calendar`.
pub const CALENDAR_WIDTH: u32 = 700;
pub const CALENDAR_HEIGHT: u32 = 590;
pub const TITLEBAR_HEIGHT: i32 = 40;
const PAD: i32 = 14;
const GRID_GAP: i32 = 3;
const TOOLBAR_HEIGHT: i32 = 36;
const WEEKDAY_HEIGHT: i32 = 22;

impl App {
    pub(crate) fn open_calendar(&mut self) {
        self.close_system();
        self.shortcuts_open = false;
        self.close_task_manager();
        self.close_sandboxes();
        self.shell.close_audio();
        self.shell.close_themes();
        self.shell.close_remote();
        self.shell.close_explorer();
        self.shell.close_chat();
        self.shell.close_overlay();
        self.calendar = Some(CalendarState::new(Clock::now()));
        println!("aether-shell: calendar opened");
        self.mark_full_damage();
    }

    pub fn calendar_open(&self) -> bool {
        self.calendar.is_some()
    }

    pub(crate) fn close_calendar(&mut self) {
        if self.calendar.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn calendar_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = CALENDAR_WIDTH.min(bounds.width.saturating_sub(32));
        let height = CALENDAR_HEIGHT.min(bounds.height.saturating_sub(60));
        AetherRect::new(
            bounds.x + (bounds.width as i32 - width as i32) / 2,
            bounds.y + 88,
            width,
            height,
        )
    }

    /// Body below the title bar.
    pub(crate) fn calendar_body_rect(&self) -> AetherRect {
        let rect = self.calendar_rect();
        AetherRect::new(
            rect.x + 1,
            rect.y + TITLEBAR_HEIGHT,
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(TITLEBAR_HEIGHT as u32 + 1),
        )
    }

    /// Month column: `minmax(0, 1.55fr)` of the body.
    pub(crate) fn calendar_month_rect(&self) -> AetherRect {
        let body = self.calendar_body_rect();
        let width = (body.width as f32 * 1.55 / 2.55).round() as u32;
        AetherRect::new(body.x, body.y, width, body.height)
    }

    pub(crate) fn calendar_agenda_rect(&self) -> AetherRect {
        let body = self.calendar_body_rect();
        let month = self.calendar_month_rect();
        AetherRect::new(
            month.right() as i32,
            body.y,
            body.width.saturating_sub(month.width),
            body.height,
        )
    }

    fn month_content(&self) -> AetherRect {
        self.calendar_month_rect().inset(PAD)
    }

    pub(crate) fn calendar_prev_rect(&self) -> AetherRect {
        let content = self.month_content();
        AetherRect::new(content.x, content.y, 26, 26)
    }

    pub(crate) fn calendar_next_rect(&self) -> AetherRect {
        let content = self.month_content();
        AetherRect::new(content.right() as i32 - 26 - 6 - 48 - 6, content.y, 26, 26)
    }

    pub(crate) fn calendar_today_rect(&self) -> AetherRect {
        let content = self.month_content();
        AetherRect::new(content.right() as i32 - 48, content.y, 48, 26)
    }

    pub(crate) fn calendar_grid_rect(&self) -> AetherRect {
        let content = self.month_content();
        let top = content.y + TOOLBAR_HEIGHT + WEEKDAY_HEIGHT;
        AetherRect::new(
            content.x,
            top,
            content.width,
            (content.bottom() as i32 - top).max(0) as u32,
        )
    }

    fn cell_size(&self) -> (u32, u32) {
        let grid = self.calendar_grid_rect();
        let width = (grid.width as i32 - GRID_GAP * 6).max(7) as u32 / 7;
        let height = (grid.height as i32 - GRID_GAP * 5).max(6) as u32 / 6;
        (width, height)
    }

    pub(crate) fn calendar_cell_rect(&self, index: usize) -> AetherRect {
        let grid = self.calendar_grid_rect();
        let (width, height) = self.cell_size();
        let column = (index % 7) as i32;
        let row = (index / 7) as i32;
        AetherRect::new(
            grid.x + column * (width as i32 + GRID_GAP),
            grid.y + row * (height as i32 + GRID_GAP),
            width,
            height,
        )
    }

    pub(crate) fn calendar_cell_at(&self, point: AetherPoint) -> Option<usize> {
        (0..42).find(|index| self.calendar_cell_rect(*index).contains(point))
    }

    pub(crate) fn calendar_light_rects(&self) -> [AetherRect; 3] {
        let rect = self.calendar_rect();
        [
            AetherRect::new(rect.x + 11, rect.y + 14, 11, 11),
            AetherRect::new(rect.x + 28, rect.y + 14, 11, 11),
            AetherRect::new(rect.x + 45, rect.y + 14, 11, 11),
        ]
    }

    pub(crate) fn calendar_select_at(&mut self, point: AetherPoint) {
        if self.calendar_prev_rect().contains(point) {
            if let Some(state) = self.calendar.as_mut() {
                state.previous_month();
            }
        } else if self.calendar_next_rect().contains(point) {
            if let Some(state) = self.calendar.as_mut() {
                state.next_month();
            }
        } else if self.calendar_today_rect().contains(point) {
            if let Some(state) = self.calendar.as_mut() {
                state.go_today();
            }
        } else if let Some(index) = self.calendar_cell_at(point) {
            if let Some(state) = self.calendar.as_mut() {
                let date = state.cells()[index].date;
                state.select(date);
            }
        }
        self.mark_calendar_damage();
    }

    pub(crate) fn mark_calendar_damage(&mut self) {
        if self.calendar.is_some() {
            self.mark_damage_rect(self.calendar_rect());
        }
    }

    pub(crate) fn handle_calendar_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let left: Keysym = keysyms::KEY_Left.into();
        let right: Keysym = keysyms::KEY_Right.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let today: Keysym = keysyms::KEY_t.into();
        if keysym == escape {
            self.close_calendar();
            return;
        }
        if keysym == page_up {
            if let Some(state) = self.calendar.as_mut() {
                state.previous_month();
            }
        } else if keysym == page_down {
            if let Some(state) = self.calendar.as_mut() {
                state.next_month();
            }
        } else if let Some(state) = self.calendar.as_mut() {
            match keysym {
                key if key == left => state.select_offset(-1),
                key if key == right => state.select_offset(1),
                key if key == up => state.select_offset(-7),
                key if key == down => state.select_offset(7),
                key if key == today => state.go_today(),
                _ => return,
            }
        }
        self.mark_calendar_damage();
    }
}
