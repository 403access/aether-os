//! Input entry points shared by the backends.
//!
//! Keyboard and pointer routing live in their own submodules; this module owns
//! the seat/cursor accessors, the compositor drag/resize state and the input
//! timing contract.

mod cursor;
mod keyboard;
pub(crate) mod keymap;
mod pointer;

use aether_cursor::CursorShape;
use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use aether_shell::WindowId;
use smithay::input::Seat;
use smithay::utils::{Logical, Point as SPoint};

use super::App;

pub(crate) use cursor::{cursor_for, pointer_shape};
pub(crate) use keymap::KeyProfile;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy)]
pub(crate) struct WindowResize {
    id: WindowId,
    corner: ResizeCorner,
    start_pointer: AetherPoint,
    start_rect: AetherRect,
}

#[derive(Clone, Copy)]
pub(crate) struct WindowDrag {
    id: WindowId,
    offset_x: f64,
    offset_y: f64,
}

impl App {
    pub fn cursor(&self) -> CursorShape {
        self.cursor
    }

    pub fn seat(&self) -> &Seat<Self> {
        &self.seat
    }

    pub fn pointer_location(&self) -> SPoint<f64, Logical> {
        self.pointer_pos
    }

    /// Update the pointer location without synthesising client events.
    pub fn set_pointer_location(&mut self, position: SPoint<f64, Logical>) {
        self.pointer_pos = position;
        self.cursor = cursor_for(self);
    }

    /// Timestamp at the start of input handling. Damage is added by each
    /// handler for exactly what it changed.
    pub fn begin_input(&mut self) -> std::time::Instant {
        std::time::Instant::now()
    }

    /// Record input arrival → damage commit latency.
    pub fn end_input(&mut self, start: std::time::Instant) {
        self.last_input_to_damage_ns = start.elapsed().as_nanos() as u64;
    }

    pub fn last_input_to_damage_ns(&self) -> u64 {
        self.last_input_to_damage_ns
    }
}
