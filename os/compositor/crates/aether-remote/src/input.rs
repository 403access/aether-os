//! Virtual-seat input: scancode validation, motion coalescing and a rate limit.
//!
//! The browser never sends commands. It sends enough information to reconstruct
//! a physical input device: key scancodes, relative motion, buttons and wheel
//! deltas. Everything else is dropped and counted.

use serde::{Deserialize, Serialize};

/// Linux evdev `KEY_MAX` (0x2ff). Key 0 is reserved and is never accepted.
pub const MAX_SCANCODE: u16 = 0x2ff;
pub const MAX_MOTION_PER_EVENT: i32 = 2_048;
pub const DEFAULT_EVENTS_PER_SECOND: u32 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum InputEvent {
    Key {
        scancode: u16,
        pressed: bool,
        modifiers: u8,
    },
    PointerMotion {
        dx: i16,
        dy: i16,
    },
    PointerButton {
        button: u8,
        pressed: bool,
    },
    Wheel {
        dx: i16,
        dy: i16,
    },
    Focus {
        focused: bool,
    },
}

impl InputEvent {
    pub fn key(scancode: u16, pressed: bool) -> Self {
        Self::Key {
            scancode,
            pressed,
            modifiers: 0,
        }
    }

    pub fn validate(self) -> Result<Self, InputError> {
        match self {
            Self::Key { scancode, .. } => {
                if scancode == 0 || scancode > MAX_SCANCODE {
                    Err(InputError::Scancode { scancode })
                } else {
                    Ok(self)
                }
            }
            Self::PointerMotion { dx, dy } => {
                if i32::from(dx).abs() > MAX_MOTION_PER_EVENT
                    || i32::from(dy).abs() > MAX_MOTION_PER_EVENT
                {
                    Err(InputError::Motion { dx, dy })
                } else {
                    Ok(self)
                }
            }
            Self::PointerButton { button, .. } => {
                if button > 7 {
                    Err(InputError::Button { button })
                } else {
                    Ok(self)
                }
            }
            Self::Wheel { .. } | Self::Focus { .. } => Ok(self),
        }
    }

    pub fn is_motion(self) -> bool {
        matches!(self, Self::PointerMotion { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    Scancode { scancode: u16 },
    Motion { dx: i16, dy: i16 },
    Button { button: u8 },
    NotFocused,
    RateLimited,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scancode { scancode } => write!(f, "scancode {scancode} is out of range"),
            Self::Motion { dx, dy } => write!(f, "motion {dx},{dy} exceeds the per-event limit"),
            Self::Button { button } => write!(f, "button {button} is not a pointer button"),
            Self::NotFocused => write!(f, "input arrived while the viewer was unfocused"),
            Self::RateLimited => write!(f, "input rate limit reached"),
        }
    }
}

impl std::error::Error for InputError {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBatch {
    pub events: Vec<InputEvent>,
    pub coalesced_motion: u64,
    pub rejected: u64,
    pub dropped_rate: u64,
}

/// Coalesces pointer motion and enforces a per-second event budget.
#[derive(Clone, Debug)]
pub struct InputBatcher {
    max_per_second: u32,
    window_start_ms: u64,
    in_window: u32,
    focused: bool,
    pending_motion: (i32, i32),
    batch: InputBatch,
}

impl Default for InputBatcher {
    fn default() -> Self {
        Self::new(DEFAULT_EVENTS_PER_SECOND)
    }
}

impl InputBatcher {
    pub fn new(max_per_second: u32) -> Self {
        Self {
            max_per_second: max_per_second.max(1),
            window_start_ms: 0,
            in_window: 0,
            focused: false,
            pending_motion: (0, 0),
            batch: InputBatch::default(),
        }
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn push(&mut self, event: InputEvent, now_ms: u64) -> Result<(), InputError> {
        let event = event.validate()?;
        if now_ms >= self.window_start_ms + 1_000 {
            self.window_start_ms = now_ms;
            self.in_window = 0;
        }
        if let InputEvent::Focus { focused } = event {
            self.focused = focused;
            if !focused {
                self.pending_motion = (0, 0);
            }
            self.batch.events.push(event);
            return Ok(());
        }
        if !self.focused {
            self.batch.rejected += 1;
            return Err(InputError::NotFocused);
        }
        if self.in_window >= self.max_per_second {
            self.batch.dropped_rate += 1;
            return Err(InputError::RateLimited);
        }
        self.in_window += 1;
        match event {
            InputEvent::PointerMotion { dx, dy } => {
                self.pending_motion.0 += i32::from(dx);
                self.pending_motion.1 += i32::from(dy);
                self.batch.coalesced_motion += 1;
            }
            _ => self.batch.events.push(event),
        }
        Ok(())
    }

    /// Flush coalesced motion into one event. Called once per frame/packet.
    pub fn flush(&mut self) {
        if self.pending_motion == (0, 0) {
            return;
        }
        let dx = self
            .pending_motion
            .0
            .clamp(-MAX_MOTION_PER_EVENT, MAX_MOTION_PER_EVENT) as i16;
        let dy = self
            .pending_motion
            .1
            .clamp(-MAX_MOTION_PER_EVENT, MAX_MOTION_PER_EVENT) as i16;
        self.pending_motion = (0, 0);
        if dx != 0 || dy != 0 {
            self.batch.events.push(InputEvent::PointerMotion { dx, dy });
        }
    }

    pub fn drain(&mut self) -> InputBatch {
        self.flush();
        std::mem::take(&mut self.batch)
    }
}

/// The host-side seat. Applying an event is a counter here; the compositor
/// backend forwards it to its virtual input device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualSeat {
    pub applied: u64,
    pub keys: u64,
    pub buttons: u64,
    pub motion: (i64, i64),
    pub wheels: (i64, i64),
}

impl VirtualSeat {
    pub fn apply(&mut self, event: InputEvent) {
        self.applied += 1;
        match event {
            InputEvent::Key { .. } => self.keys += 1,
            InputEvent::PointerButton { .. } => self.buttons += 1,
            InputEvent::PointerMotion { dx, dy } => {
                self.motion.0 += i64::from(dx);
                self.motion.1 += i64::from(dy);
            }
            InputEvent::Wheel { dx, dy } => {
                self.wheels.0 += i64::from(dx);
                self.wheels.1 += i64::from(dy);
            }
            InputEvent::Focus { .. } => {}
        }
    }

    pub fn apply_batch(&mut self, batch: &InputBatch) -> u64 {
        for event in &batch.events {
            self.apply(*event);
        }
        batch.events.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_scancodes_and_motion() {
        assert_eq!(
            InputEvent::key(0, true).validate(),
            Err(InputError::Scancode { scancode: 0 })
        );
        assert_eq!(
            InputEvent::key(MAX_SCANCODE + 1, true).validate(),
            Err(InputError::Scancode {
                scancode: MAX_SCANCODE + 1
            })
        );
        assert!(InputEvent::key(30, true).validate().is_ok());
        assert_eq!(
            InputEvent::PointerMotion { dx: 5_000, dy: 0 }.validate(),
            Err(InputError::Motion { dx: 5_000, dy: 0 })
        );
        assert_eq!(
            InputEvent::PointerButton {
                button: 9,
                pressed: true
            }
            .validate(),
            Err(InputError::Button { button: 9 })
        );
    }

    #[test]
    fn requires_focus_and_coalesces_motion() {
        let mut batcher = InputBatcher::new(100);
        assert_eq!(
            batcher.push(InputEvent::key(30, true), 0),
            Err(InputError::NotFocused)
        );
        batcher
            .push(InputEvent::Focus { focused: true }, 0)
            .unwrap();
        batcher
            .push(InputEvent::PointerMotion { dx: 3, dy: 4 }, 1)
            .unwrap();
        batcher
            .push(InputEvent::PointerMotion { dx: -1, dy: 2 }, 2)
            .unwrap();
        batcher.push(InputEvent::key(30, true), 3).unwrap();
        let batch = batcher.drain();
        assert_eq!(batch.coalesced_motion, 2);
        assert_eq!(batch.rejected, 1);
        assert_eq!(batch.events.len(), 3);
        assert_eq!(batch.events[2], InputEvent::PointerMotion { dx: 2, dy: 6 });
    }

    #[test]
    fn enforces_the_rate_limit_per_window() {
        let mut batcher = InputBatcher::new(2);
        batcher
            .push(InputEvent::Focus { focused: true }, 0)
            .unwrap();
        batcher.push(InputEvent::key(30, true), 1).unwrap();
        batcher.push(InputEvent::key(31, true), 2).unwrap();
        assert_eq!(
            batcher.push(InputEvent::key(32, true), 3),
            Err(InputError::RateLimited)
        );
        let batch = batcher.drain();
        assert_eq!(batch.dropped_rate, 1);
        assert_eq!(batch.events.len(), 3);
        // A fresh window admits events again.
        assert!(batcher.push(InputEvent::key(33, true), 1_001).is_ok());
    }

    #[test]
    fn virtual_seat_counts_input() {
        let mut seat = VirtualSeat::default();
        let batch = InputBatch {
            events: vec![
                InputEvent::key(30, true),
                InputEvent::PointerMotion { dx: 5, dy: -3 },
                InputEvent::PointerButton {
                    button: 1,
                    pressed: true,
                },
            ],
            ..InputBatch::default()
        };
        assert_eq!(seat.apply_batch(&batch), 3);
        assert_eq!(seat.keys, 1);
        assert_eq!(seat.buttons, 1);
        assert_eq!(seat.motion, (5, -3));
    }
}
