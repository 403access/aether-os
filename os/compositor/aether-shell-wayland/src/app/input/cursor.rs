//! Cursor shape resolution from compositor state and client cursor images.

use aether_cursor::CursorShape;
use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use smithay::input::pointer::CursorIcon as PointerIcon;

use crate::topbar;

use super::{App, ResizeCorner};

pub(crate) fn cursor_for(state: &App) -> CursorShape {
    let position = state.pointer_pos;
    let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);

    if let Some(resize) = state.window_resize {
        return resize_cursor(resize.corner);
    }
    if state.window_drag.is_some() {
        return CursorShape::Grabbing;
    }
    if state.task_manager_open() {
        if state.task_close_rect().contains(point) || state.task_row_at(point).is_some() {
            return CursorShape::Pointer;
        }
        return CursorShape::Arrow;
    }
    if state.shortcuts_open {
        return CursorShape::Arrow;
    }
    if let Some(output) = state.shell.output_at(point) {
        if state.top_bar_visible(output)
            && topbar::hit(&state.top_bar(output), point.x, point.y).is_some()
        {
            return CursorShape::Pointer;
        }
    }
    if let Some(overlay) = state.shell.overlay_rect() {
        if overlay.contains(point) {
            if state.shell.palette_row_at(point).is_some() {
                return CursorShape::Pointer;
            }
            let input_top = overlay.y + 12;
            if point.y >= input_top && point.y < input_top + 36 {
                return CursorShape::Text;
            }
        }
        return CursorShape::Arrow;
    }
    if state.task_manager_open() {
        if state.task_close_rect().contains(point) || state.task_row_at(point).is_some() {
            return CursorShape::Pointer;
        }
        return CursorShape::Arrow;
    }
    if state.shortcuts_open {
        return CursorShape::Arrow;
    }
    if state.shell.themes_open() {
        if state.shell.theme_row_at(point).is_some()
            || state.shell.theme_action_at(point)
            || state.shell.theme_reset_at(point)
            || state.shell.theme_close_at(point)
        {
            return CursorShape::Pointer;
        }
        return CursorShape::Arrow;
    }
    if state.shell.audio_open() {
        if state.shell.audio_rect().contains(point) {
            if state.shell.audio_route_at(point).is_some()
                || state.shell.audio_preset_at(point).is_some()
            {
                return CursorShape::Pointer;
            }
            return CursorShape::Arrow;
        }
        return CursorShape::Arrow;
    }
    if state.shell.remote_open() {
        if state.shell.remote_action_at(point).is_some() {
            return CursorShape::Pointer;
        }
        return CursorShape::Arrow;
    }
    if state.shell.chat_open() && state.shell.chat_rect().contains(point) {
        return CursorShape::Arrow;
    }
    if state.shell.explorer_open() {
        let rect = state.shell.explorer_rect();
        if rect.contains(point) {
            return CursorShape::Pointer;
        }
    }
    if state.shell.workspace_at_global(point).is_some() {
        return CursorShape::Pointer;
    }
    if let Some(id) = state.shell.window_at(point) {
        if let Some(info) = state
            .shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
        {
            if state.modifiers.alt {
                if info.floating {
                    if let Some(corner) = floating_corner(info.rect, point) {
                        return resize_cursor(corner);
                    }
                }
                return CursorShape::Move;
            }
        }
    }
    if let Some(shape) = state.client_cursor {
        return shape;
    }
    CursorShape::Arrow
}

pub(crate) fn pointer_shape(icon: PointerIcon) -> CursorShape {
    match icon {
        PointerIcon::Pointer => CursorShape::Pointer,
        PointerIcon::Text | PointerIcon::VerticalText => CursorShape::Text,
        PointerIcon::Move | PointerIcon::AllScroll => CursorShape::Move,
        PointerIcon::Grabbing => CursorShape::Grabbing,
        PointerIcon::NwseResize => CursorShape::ResizeNwse,
        PointerIcon::NeswResize => CursorShape::ResizeNesw,
        PointerIcon::ColResize | PointerIcon::EwResize => CursorShape::ResizeHorizontal,
        _ => CursorShape::Arrow,
    }
}

fn resize_cursor(corner: ResizeCorner) -> CursorShape {
    match corner {
        ResizeCorner::TopLeft | ResizeCorner::BottomRight => CursorShape::ResizeNwse,
        ResizeCorner::TopRight | ResizeCorner::BottomLeft => CursorShape::ResizeNesw,
    }
}

fn floating_corner(rect: AetherRect, point: AetherPoint) -> Option<ResizeCorner> {
    const CORNER: i32 = 14;
    let left = point.x - rect.x <= CORNER;
    let right = rect.right() - point.x as i64 <= CORNER as i64;
    let top = point.y - rect.y <= CORNER;
    let bottom = rect.bottom() - point.y as i64 <= CORNER as i64;
    match (top, bottom, left, right) {
        (true, _, true, _) => Some(ResizeCorner::TopLeft),
        (true, _, _, true) => Some(ResizeCorner::TopRight),
        (_, true, true, _) => Some(ResizeCorner::BottomLeft),
        (_, true, _, true) => Some(ResizeCorner::BottomRight),
        _ => None,
    }
}

pub(crate) fn resize_corner(rect: AetherRect, point: AetherPoint) -> ResizeCorner {
    let left = (point.x - rect.x) < rect.width as i32 / 2;
    let top = (point.y - rect.y) < rect.height as i32 / 2;
    match (top, left) {
        (true, true) => ResizeCorner::TopLeft,
        (true, false) => ResizeCorner::TopRight,
        (false, true) => ResizeCorner::BottomLeft,
        (false, false) => ResizeCorner::BottomRight,
    }
}
