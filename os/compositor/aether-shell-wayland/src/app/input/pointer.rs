//! Pointer motion, button and axis routing, including compositor drag/resize.

use aether_geometry::Point as AetherPoint;
use aether_shell::audio::AudioAction;
use aether_shell::{SnapZone, WindowId, MIN_FLOATING_HEIGHT, MIN_FLOATING_WIDTH};
use smithay::backend::input::{AxisSource, ButtonState, MouseButton};
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::utils::{Logical, Point as SPoint, SERIAL_COUNTER};

use super::cursor::resize_corner;
use super::{cursor_for, App, ResizeCorner, WindowDrag, WindowResize};

impl App {
    /// Absolute pointer motion in logical compositor coordinates.
    pub fn pointer_motion(&mut self, position: SPoint<f64, Logical>, time: u32) {
        let started = self.begin_input();
        let old_position = self.pointer_pos;
        let old_cursor = self.cursor;
        self.pointer_pos = position;
        let new_cursor = cursor_for(self);
        self.cursor = new_cursor;

        if let Some(resize) = self.window_resize {
            let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
            let dx = point.x - resize.start_pointer.x;
            let dy = point.y - resize.start_pointer.y;
            let start = resize.start_rect;
            let min_width = MIN_FLOATING_WIDTH as i32;
            let min_height = MIN_FLOATING_HEIGHT as i32;
            let mut rect = start;
            match resize.corner {
                ResizeCorner::BottomRight => {
                    rect.width = (start.width as i32 + dx).max(min_width) as u32;
                    rect.height = (start.height as i32 + dy).max(min_height) as u32;
                }
                ResizeCorner::BottomLeft => {
                    let width = (start.width as i32 - dx).max(min_width);
                    rect.x = start.x + start.width as i32 - width;
                    rect.width = width as u32;
                    rect.height = (start.height as i32 + dy).max(min_height) as u32;
                }
                ResizeCorner::TopRight => {
                    rect.width = (start.width as i32 + dx).max(min_width) as u32;
                    let height = (start.height as i32 - dy).max(min_height);
                    rect.y = start.y + start.height as i32 - height;
                    rect.height = height as u32;
                }
                ResizeCorner::TopLeft => {
                    let width = (start.width as i32 - dx).max(min_width);
                    let height = (start.height as i32 - dy).max(min_height);
                    rect.x = start.x + start.width as i32 - width;
                    rect.y = start.y + start.height as i32 - height;
                    rect.width = width as u32;
                    rect.height = height as u32;
                }
            }
            let before = self.window_rect(resize.id);
            if self.shell.resize_floating(resize.id, rect) {
                if let Some(before) = before {
                    self.mark_damage_rect(before);
                }
                if let Some(after) = self.window_rect(resize.id) {
                    self.mark_damage_rect(after);
                }
                self.configure_all();
            }
        } else if let Some(drag) = self.window_drag {
            let x = (position.x - drag.offset_x).round() as i32;
            let y = (position.y - drag.offset_y).round() as i32;
            let before = self.window_rect(drag.id);
            if self.shell.move_floating(drag.id, AetherPoint::new(x, y)) {
                if let Some(before) = before {
                    self.mark_damage_rect(before);
                }
                if let Some(after) = self.window_rect(drag.id) {
                    self.mark_damage_rect(after);
                }
                self.configure_all();
            }
        } else {
            let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
            if self.shell.overlay().is_some() {
                if let Some(index) = self.shell.palette_row_at(point) {
                    self.shell.palette_select(index);
                }
            }
            let chrome_owns_pointer =
                self.shell.overlay().is_some() || self.shell.workspace_at_global(point).is_some();
            if chrome_owns_pointer {
                // Compositor-owned chrome never focuses a client underneath.
            } else if let Some(id) = self.shell.window_at(point) {
                if self.shell.focus_window(id) {
                    self.sync_focus();
                }
                self.forward_pointer_motion(Some(id), position, time);
            } else {
                self.forward_pointer_motion(None, position, time);
            }
        }
        if old_cursor != new_cursor || old_position != position {
            self.mark_cursor_damage(old_position);
            self.mark_cursor_damage(position);
        }
        if self.shell.overlay().is_some() {
            self.mark_overlay_damage();
        }
        if self.shell.explorer_open() {
            self.mark_explorer_damage();
        }
        self.end_input(started);
    }

    /// Relative pointer motion (libinput mice/touchpads).
    pub fn pointer_motion_relative(&mut self, delta: (f64, f64), time: u32) {
        let viewport = self.shell.viewport();
        let x =
            (self.pointer_pos.x + delta.0).clamp(0.0, (viewport.width.saturating_sub(1)) as f64);
        let y =
            (self.pointer_pos.y + delta.1).clamp(0.0, (viewport.height.saturating_sub(1)) as f64);
        self.pointer_motion(SPoint::from((x, y)), time);
    }

    /// Absolute pointer motion in normalised device coordinates, mapped onto
    /// the output currently under the pointer (falling back to the primary).
    pub fn pointer_motion_absolute(&mut self, normalized: (f64, f64), time: u32) {
        let current = AetherPoint::new(
            self.pointer_pos.x.round() as i32,
            self.pointer_pos.y.round() as i32,
        );
        let output = self
            .shell
            .output_at(current)
            .or_else(|| self.shell.primary_output().map(|output| output.id))
            .unwrap_or(1);
        let bounds = self.shell.output_bounds(output);
        let x = bounds.x as f64 + normalized.0.clamp(0.0, 1.0) * bounds.width.max(1) as f64;
        let y = bounds.y as f64 + normalized.1.clamp(0.0, 1.0) * bounds.height.max(1) as f64;
        self.pointer_motion(SPoint::from((x, y)), time);
    }

    fn forward_pointer_motion(
        &mut self,
        id: Option<WindowId>,
        position: SPoint<f64, Logical>,
        time: u32,
    ) {
        let focus = id.and_then(|id| {
            let rect = self
                .shell
                .windows()
                .into_iter()
                .find(|window| window.id == id)
                .map(|window| window.rect)?;
            let origin = SPoint::from((rect.x as f64, rect.y as f64));
            self.windows
                .get(&id)
                .map(|toplevel| (toplevel.wl_surface().clone(), origin))
        });
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().expect("seat pointer");
        pointer.motion(
            self,
            focus,
            &MotionEvent {
                location: position,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    /// Pointer button press/release. Compositor chrome, alt-drag and window
    /// focus are handled here; unconsumed buttons go to the focused client.
    pub fn pointer_button(&mut self, button: u32, state: ButtonState, time: u32) {
        let started = self.begin_input();
        let position = self.pointer_pos;
        let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
        let modal = self.shell.overlay().is_some();
        let over_picker = self.shell.workspace_at_global(point).is_some();
        let mouse_button = match button {
            0x110 => Some(MouseButton::Left),
            0x111 => Some(MouseButton::Right),
            0x112 => Some(MouseButton::Middle),
            _ => None,
        };

        if state == ButtonState::Pressed {
            self.pointer_captured = modal
                || over_picker
                || self.shell.explorer_open()
                || self.shell.chat_open()
                || self.shell.audio_open()
                || self.shortcuts_open
                || self.task_manager_open();
            if let Some(action) = self.top_shortcut_at(point) {
                self.pointer_captured = true;
                self.activate_top_shortcut(action);
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if self.shortcuts_open {
                if !self.shortcuts_rect().contains(point) {
                    self.close_shortcuts();
                }
                self.mark_shortcuts_damage();
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if self.task_manager_open() {
                if self.task_close_rect().contains(point) {
                    self.close_task_manager();
                } else if let Some(index) = self.task_row_at(point) {
                    let visible = self.task_visible_rows();
                    if let Some(state) = self.task_manager.as_mut() {
                        state.select(index, visible);
                    }
                }
                self.mark_task_damage();
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if self.shell.audio_open() {
                if let Some(index) = self.shell.audio_route_at(point) {
                    self.shell.audio_select(index);
                    self.activate_audio();
                } else if let Some(preset) = self.shell.audio_preset_at(point) {
                    if let Some(AudioAction::SetPreset(preset)) =
                        self.shell.audio_set_preset(preset)
                    {
                        self.apply_audio_preset(preset);
                    }
                } else if !self.shell.audio_rect().contains(point) {
                    self.shell.close_audio();
                    self.mark_full_damage();
                }
                self.mark_audio_damage();
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if self.shell.remote_open() {
                if let Some(index) = self.shell.remote_action_at(point) {
                    self.shell.remote_select(index);
                    self.activate_remote();
                } else if !self.shell.remote_rect().contains(point) {
                    self.shell.close_remote();
                    self.mark_full_damage();
                }
                self.mark_remote_damage();
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if self.shell.chat_open() {
                if self.shell.chat_rect().contains(point) {
                    self.mark_chat_damage();
                } else {
                    self.shell.close_chat();
                    self.mark_full_damage();
                }
                self.end_input(started);
                return;
            }
            if self.shell.explorer_open() {
                if self.shell.explorer_select_at(point).is_some() {
                    self.pointer_captured = true;
                } else if !self.shell.explorer_rect().contains(point) {
                    self.shell.close_explorer();
                    self.mark_full_damage();
                }
                self.mark_explorer_damage();
                self.cursor = cursor_for(self);
                self.mark_cursor_damage(self.pointer_pos);
                self.end_input(started);
                return;
            }
            if let Some(workspace) = self.shell.workspace_at_global(point) {
                let output = self.output_under_pointer();
                self.switch_workspace_on(output, workspace);
            } else if modal {
                if let Some(index) = self.shell.palette_row_at(point) {
                    self.shell.palette_select(index);
                    self.activate_palette();
                } else if self
                    .shell
                    .overlay_rect()
                    .is_some_and(|rect| !rect.contains(point))
                {
                    self.toggle_overlay();
                }
            } else if self.modifiers.alt && mouse_button == Some(MouseButton::Left) {
                if let Some(id) = self.shell.window_at(point) {
                    self.shell.focus_window(id);
                    self.sync_focus();
                    if let Some(rect) = self
                        .shell
                        .begin_floating_move(id, AetherPoint::new(point.x, point.y))
                    {
                        self.window_drag = Some(WindowDrag {
                            id,
                            offset_x: position.x - rect.x as f64,
                            offset_y: position.y - rect.y as f64,
                        });
                        self.pointer_captured = true;
                        self.mark_full_damage();
                        self.configure_all();
                        println!("aether-shell: window drag started id={id} rect={rect:?}");
                    }
                }
            } else if self.modifiers.alt && mouse_button == Some(MouseButton::Right) {
                if let Some(id) = self.shell.window_at(point) {
                    self.shell.focus_window(id);
                    self.sync_focus();
                    if let Some(rect) = self.shell.begin_floating_resize(id) {
                        let corner = resize_corner(rect, point);
                        self.window_resize = Some(WindowResize {
                            id,
                            corner,
                            start_pointer: point,
                            start_rect: rect,
                        });
                        self.pointer_captured = true;
                        self.mark_full_damage();
                        self.configure_all();
                        println!(
                            "aether-shell: window resize started id={id} rect={rect:?} corner={corner:?}"
                        );
                    }
                }
            } else if let Some(id) = self.shell.window_at(point) {
                self.shell.focus_window(id);
                self.sync_focus();
                // Ensure the client has an up-to-date pointer position before
                // the button, even if no motion arrived after a focus change.
                self.forward_pointer_motion(Some(id), position, time);
            }
        }

        let forward = !self.pointer_captured;
        if state == ButtonState::Released {
            if let Some(resize) = self.window_resize.take() {
                let rect = self
                    .shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == resize.id)
                    .map(|window| window.rect);
                println!(
                    "aether-shell: window resized id={} rect={rect:?}",
                    resize.id
                );
            }
            if let Some(drag) = self.window_drag.take() {
                let position = self.pointer_pos;
                let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
                let output = self
                    .shell
                    .output_at(point)
                    .or_else(|| self.shell.primary_output().map(|output| output.id))
                    .unwrap_or(1);
                let bounds = self.shell.output_bounds(output);
                let zone = if point.x <= bounds.x + 8 {
                    Some(SnapZone::LeftHalf)
                } else if point.x >= bounds.right() as i32 - 8 {
                    Some(SnapZone::RightHalf)
                } else if point.y <= bounds.y + 8 {
                    Some(SnapZone::Maximize)
                } else {
                    None
                };
                if let Some(zone) = zone {
                    let before = self.window_rect(drag.id);
                    if self.shell.snap_window(drag.id, zone) {
                        if let Some(before) = before {
                            self.mark_damage_rect(before);
                        }
                        if let Some(after) = self.window_rect(drag.id) {
                            self.mark_damage_rect(after);
                        }
                        self.configure_all();
                        println!("aether-shell: window snapped id={} zone={zone:?}", drag.id);
                    }
                }
                let rect = self
                    .shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == drag.id)
                    .map(|window| window.rect);
                println!("aether-shell: window moved id={} rect={rect:?}", drag.id);
            }
            self.pointer_captured = false;
        }

        if forward {
            let serial = SERIAL_COUNTER.next_serial();
            let pointer = self.seat.get_pointer().expect("seat pointer");
            pointer.button(
                self,
                &ButtonEvent {
                    button,
                    state,
                    serial,
                    time,
                },
            );
            pointer.frame(self);
        }
        self.cursor = cursor_for(self);
        self.mark_cursor_damage(self.pointer_pos);
        self.end_input(started);
    }

    /// Pointer axis (scroll) event.
    pub fn pointer_axis(&mut self, horizontal: f64, vertical: f64, source: AxisSource, time: u32) {
        let started = self.begin_input();
        if self.shell.chat_open() {
            let point = AetherPoint::new(
                self.pointer_pos.x.round() as i32,
                self.pointer_pos.y.round() as i32,
            );
            if self.shell.chat_rect().contains(point) {
                if vertical != 0.0 {
                    self.shell
                        .chat_scroll_by(if vertical > 0.0 { 3 } else { -3 });
                }
                self.mark_chat_damage();
                self.end_input(started);
                return;
            }
        }
        if self.shell.explorer_open() {
            let point = AetherPoint::new(
                self.pointer_pos.x.round() as i32,
                self.pointer_pos.y.round() as i32,
            );
            if self.shell.explorer_rect().contains(point) {
                if vertical != 0.0 {
                    self.shell
                        .explorer_scroll_by(if vertical > 0.0 { 3 } else { -3 });
                }
                self.mark_explorer_damage();
                self.end_input(started);
                return;
            }
        }
        let mut axis = AxisFrame::new(time).source(source);
        if horizontal != 0.0 {
            axis = axis.value(smithay::backend::input::Axis::Horizontal, horizontal);
        }
        if vertical != 0.0 {
            axis = axis.value(smithay::backend::input::Axis::Vertical, vertical);
        }
        if source == AxisSource::Finger {
            if horizontal == 0.0 {
                axis = axis.stop(smithay::backend::input::Axis::Horizontal);
            }
            if vertical == 0.0 {
                axis = axis.stop(smithay::backend::input::Axis::Vertical);
            }
        }
        let pointer = self.seat.get_pointer().expect("seat pointer");
        pointer.axis(self, axis);
        pointer.frame(self);
        // Scrolling is forwarded to the client; its commit carries the damage.
        self.end_input(started);
    }
}
