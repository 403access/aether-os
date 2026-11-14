//! Backend-agnostic Wayland state for the Aether shell.
//!
//! `App` owns the Wayland display objects, seat, shell policy, client window
//! map and render composition. Backends (winit and native DRM) feed it input
//! and a renderer; it does not know how pixels reach the display.

use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use crate::agents::{parse_launchers, AgentSessions};
use crate::damage::buffer_damage_to_logical;
use aether_cursor::{CursorShape, CursorTheme};
use aether_geometry::{Point as AetherPoint, Rect as AetherRect, Size as AetherSize};
use aether_layout::LayoutConfig;
use aether_shell::{
    OutputId, OutputInfo, PaletteAction, PaletteFile, PaletteLauncher, Shell, SnapZone, WindowId,
    WindowSpec, MIN_FLOATING_HEIGHT, MIN_FLOATING_WIDTH,
};
use smithay::backend::input::{AxisSource, ButtonState, KeyState, Keycode, MouseButton};
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::input::keyboard::{
    keysyms, xkb, FilterResult, KeyboardHandle, Keysym, ModifiersState,
};
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, CursorIcon as PointerIcon, CursorImageStatus, MotionEvent,
};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::output::Output;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{
    Client, Display, DisplayHandle, ListeningSocket, Resource,
};
use smithay::utils::{
    Logical, Physical, Point as SPoint, Rectangle, Serial, Size as SSize, SERIAL_COUNTER,
};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    with_states, with_surface_tree_downward, CompositorClientState, CompositorHandler,
    CompositorState, Damage, SurfaceAttributes, TraversalAction,
};
use smithay::wayland::input_method::{
    InputMethodHandler, InputMethodManagerState, PopupSurface as InputMethodPopupSurface,
};
use smithay::wayland::output::{OutputHandler, OutputManagerState};
use smithay::wayland::selection::data_device::{
    set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
    ServerDndGrabHandler,
};
use smithay::wayland::selection::primary_selection::{
    set_primary_focus, PrimarySelectionHandler, PrimarySelectionState,
};
use smithay::wayland::selection::wlr_data_control::{DataControlHandler, DataControlState};
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::wayland::text_input::{TextInputManagerState, TextInputSeat};

impl BufferHandler for App {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for App {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        for entry in self.outputs.values() {
            entry.output.enter(surface.wl_surface());
        }
        let title = toplevel_title(&surface).unwrap_or_else(|| "Wayland client".to_string());
        let mut spec = WindowSpec::new(title, "wayland.client");
        spec.preferred_size = AetherSize::new(800, 600);
        // Place new windows on the output under the pointer (falling back to
        // the primary) and on that output's visible workspace.
        let point = AetherPoint::new(
            self.pointer_pos.x.round() as i32,
            self.pointer_pos.y.round() as i32,
        );
        let output = self
            .shell
            .output_at(point)
            .or_else(|| self.shell.primary_output().map(|output| output.id))
            .unwrap_or(1);
        let id = self.shell.add_window_on_output(spec, output);
        self.windows.insert(id, surface.clone());
        self.focus = Some(surface.wl_surface().clone());
        self.configure_window(id);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        let Some(title) = toplevel_title(&surface) else {
            return;
        };
        let id = self.windows.iter().find_map(|(id, toplevel)| {
            (toplevel.wl_surface() == surface.wl_surface()).then_some(*id)
        });
        if let Some(id) = id {
            self.shell.set_window_title(id, title);
        }
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}
    fn grab(
        &mut self,
        _surface: PopupSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: Serial,
    ) {
    }
    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        for entry in self.outputs.values() {
            entry.output.leave(surface.wl_surface());
        }
        let id = self.windows.iter().find_map(|(id, toplevel)| {
            (toplevel.wl_surface() == surface.wl_surface()).then_some(*id)
        });
        if let Some(id) = id {
            self.windows.remove(&id);
            self.configured.remove(&id);
            self.shell.close_window(id);
            self.sync_focus();
            self.configure_all();
        }
    }
}

impl SelectionHandler for App {
    type SelectionUserData = ();
}

impl DataDeviceHandler for App {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl PrimarySelectionHandler for App {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}

impl DataControlHandler for App {
    fn data_control_state(&self) -> &DataControlState {
        &self.data_control_state
    }
}

impl ClientDndGrabHandler for App {}
impl ServerDndGrabHandler for App {
    fn send(&mut self, _mime_type: String, _fd: std::os::unix::io::OwnedFd, _seat: Seat<Self>) {}
}

impl CompositorHandler for App {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        let id = self
            .windows
            .iter()
            .find_map(|(id, toplevel)| (toplevel.wl_surface() == surface).then_some(*id));
        let Some(window) = id.and_then(|id| {
            self.shell
                .windows()
                .into_iter()
                .find(|window| window.id == id)
        }) else {
            self.mark_full_damage();
            return;
        };
        if window.workspace != self.shell.output_workspace(window.output) {
            return;
        }

        let (damage, buffer_scale, buffer_transform) = with_states(surface, |states| {
            let mut guard = states.cached_state.get::<SurfaceAttributes>();
            let attributes = guard.current();
            (
                std::mem::take(&mut attributes.damage),
                attributes.buffer_scale,
                attributes.buffer_transform,
            )
        });
        if damage.is_empty() {
            self.mark_window_damage(window.rect);
            return;
        }
        for damage in damage {
            match damage {
                Damage::Surface(rect) => {
                    if rect.size.w <= 0 || rect.size.h <= 0 {
                        continue;
                    }
                    let rect = AetherRect::new(
                        window.rect.x + rect.loc.x,
                        window.rect.y + rect.loc.y,
                        rect.size.w as u32,
                        rect.size.h as u32,
                    );
                    if let Some(clipped) = rect.intersect(window.rect) {
                        self.mark_damage_rect(clipped);
                    }
                }
                Damage::Buffer(rect) => {
                    let transform_ok = buffer_transform
                        == smithay::reexports::wayland_server::protocol::wl_output::Transform::Normal;
                    let converted = if transform_ok {
                        buffer_damage_to_logical(rect, buffer_scale)
                    } else {
                        None
                    };
                    if let Some(local) = converted {
                        let rect = AetherRect::new(
                            window.rect.x + local.x,
                            window.rect.y + local.y,
                            local.width,
                            local.height,
                        );
                        if let Some(clipped) = rect.intersect(window.rect) {
                            self.mark_damage_rect(clipped);
                        }
                    } else {
                        self.mark_window_damage(window.rect);
                    }
                }
            }
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl OutputHandler for App {}

impl InputMethodHandler for App {
    fn new_popup(&mut self, surface: InputMethodPopupSurface) {
        self.input_method_popups.push(surface);
    }

    fn dismiss_popup(&mut self, surface: InputMethodPopupSurface) {
        self.input_method_popups.retain(|popup| popup != &surface);
    }

    fn popup_repositioned(&mut self, _surface: InputMethodPopupSurface) {}

    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.windows
            .iter()
            .find_map(|(id, toplevel)| (toplevel.wl_surface() == parent).then_some(*id))
            .and_then(|id| {
                self.shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == id)
            })
            .map(|window| {
                Rectangle::new(
                    (window.rect.x, window.rect.y).into(),
                    (window.rect.width as i32, window.rect.height as i32).into(),
                )
            })
            .unwrap_or_default()
    }
}

impl SeatHandler for App {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        // Offer/refresh clipboard and primary selections for the newly focused
        // client so copy/paste works across clients.
        let client = focused.and_then(|surface| surface.client());
        set_data_device_focus(&self.display_handle, seat, client.clone());
        set_primary_focus(&self.display_handle, seat, client);

        // Text-input focus follows keyboard focus, so clients can use IMEs.
        let text_input = seat.text_input();
        text_input.leave();
        text_input.set_focus(focused.cloned());
        if focused.is_some() {
            text_input.enter();
        }
    }
    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        match image {
            CursorImageStatus::Hidden => {
                self.cursor_hidden = true;
                self.client_cursor = None;
                self.client_cursor_surface = None;
            }
            CursorImageStatus::Named(icon) => {
                self.cursor_hidden = false;
                self.client_cursor = Some(pointer_shape(icon));
                self.client_cursor_surface = None;
            }
            CursorImageStatus::Surface(surface) => {
                self.cursor_hidden = false;
                self.client_cursor = None;
                self.client_cursor_surface = Some(surface);
            }
        }
        self.cursor = cursor_for(self);
    }
}

fn desktop_data_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(std::path::PathBuf::from(data_home));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home).join(".local/share"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/share:/usr/share"));
    dirs.extend(std::env::split_paths(&data_dirs));
    dirs.dedup();
    dirs
}

fn file_roots() -> Vec<std::path::PathBuf> {
    if let Some(roots) = std::env::var_os("AETHER_FILE_ROOTS") {
        return std::env::split_paths(&roots).collect();
    }
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .into_iter()
        .collect()
}

fn file_index() -> Vec<PaletteFile> {
    let limit = std::env::var("AETHER_FILE_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000usize);
    let depth = std::env::var("AETHER_FILE_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5usize);
    aether_files::scan(&file_roots(), limit, depth)
        .into_iter()
        .map(|entry| PaletteFile {
            name: entry.name,
            detail: if entry.detail.is_empty() {
                "file".to_string()
            } else {
                entry.detail
            },
            path: entry.path.to_string_lossy().to_string(),
        })
        .collect()
}

fn external_launchers() -> Vec<PaletteLauncher> {
    let Some(path) = std::env::var_os("AETHER_PALETTE_JSON") else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return Vec::new();
    };
    parse_launchers(&value)
}

fn desktop_launchers() -> Vec<PaletteLauncher> {
    aether_desktop::discover(&desktop_data_dirs())
        .into_iter()
        .flat_map(|entry| {
            let detail = if entry.comment.is_empty() {
                "application".to_string()
            } else {
                entry.comment.clone()
            };
            let icon = entry
                .icon_path
                .map(|path| path.to_string_lossy().to_string());
            let mut launchers = vec![PaletteLauncher {
                name: entry.name.clone(),
                detail,
                icon: icon.clone(),
                program: entry.program.clone(),
                args: entry.args.clone(),
            }];
            for action in entry.actions {
                launchers.push(PaletteLauncher {
                    name: format!("{} - {}", entry.name, action.name),
                    detail: "action".to_string(),
                    icon: icon.clone(),
                    program: action.program,
                    args: action.args,
                });
            }
            launchers
        })
        .collect()
}

fn toplevel_title(surface: &ToplevelSurface) -> Option<String> {
    with_states(surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok())
            .and_then(|attributes| attributes.title.clone())
    })
}

#[derive(Clone, Copy)]
struct ResizeDrag {
    last_ratio: f32,
}

#[derive(Clone, Copy, Debug)]
enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy)]
struct WindowResize {
    id: WindowId,
    corner: ResizeCorner,
    start_pointer: AetherPoint,
    start_rect: AetherRect,
}

fn cursor_for(state: &App) -> CursorShape {
    let position = state.pointer_pos;
    let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);

    if let Some(resize) = state.window_resize {
        return resize_cursor(resize.corner);
    }
    if state.window_drag.is_some() {
        return CursorShape::Grabbing;
    }
    if state.resize_drag.is_some() {
        return CursorShape::ResizeHorizontal;
    }
    if let Some(overlay) = state.shell.overlay_rect() {
        if overlay.contains(point) {
            if state.shell.palette_row_at(point).is_some() {
                return CursorShape::Pointer;
            }
            let input_top = overlay.y + 52;
            if point.y >= input_top && point.y < input_top + 28 {
                return CursorShape::Text;
            }
        }
        return CursorShape::Arrow;
    }
    if state.shell.workspace_at(point).is_some() {
        return CursorShape::Pointer;
    }
    if let Some(gap) = state.shell.master_gap_rect() {
        let handle = AetherRect::new(gap.x - 5, gap.y, gap.width + 10, gap.height);
        if handle.contains(point) {
            return CursorShape::ResizeHorizontal;
        }
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

fn pointer_shape(icon: PointerIcon) -> CursorShape {
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

fn resize_corner(rect: AetherRect, point: AetherPoint) -> ResizeCorner {
    let left = (point.x - rect.x) < rect.width as i32 / 2;
    let top = (point.y - rect.y) < rect.height as i32 / 2;
    match (top, left) {
        (true, true) => ResizeCorner::TopLeft,
        (true, false) => ResizeCorner::TopRight,
        (false, true) => ResizeCorner::BottomLeft,
        (false, false) => ResizeCorner::BottomRight,
    }
}

#[derive(Clone, Copy)]
struct WindowDrag {
    id: WindowId,
    offset_x: f64,
    offset_y: f64,
}

pub(crate) struct CursorCacheEntry {
    pub(crate) buffer: MemoryRenderBuffer,
    pub(crate) hotspot_x: i32,
    pub(crate) hotspot_y: i32,
}

pub struct App {
    display_handle: DisplayHandle,
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    primary_selection_state: PrimarySelectionState,
    data_control_state: DataControlState,
    outputs: HashMap<OutputId, OutputEntry>,
    pub(crate) seat: Seat<Self>,
    pub(crate) shell: Shell,
    pub(crate) windows: HashMap<WindowId, ToplevelSurface>,
    configured: HashMap<WindowId, (i32, i32, bool)>,
    focus: Option<WlSurface>,
    close_requested: bool,
    pub(crate) scale: f64,
    pub(crate) pointer_pos: smithay::utils::Point<f64, smithay::utils::Logical>,
    resize_drag: Option<ResizeDrag>,
    window_drag: Option<WindowDrag>,
    window_resize: Option<WindowResize>,
    pointer_captured: bool,
    modifiers: ModifiersState,
    pub(crate) cursor: CursorShape,
    socket_name: String,
    listener: ListeningSocket,
    clients: Vec<Client>,
    applied_focus: Option<WlSurface>,
    spawned_test_clients: bool,
    full_damage: bool,
    damage: Vec<Rectangle<i32, Physical>>,
    pub(crate) icons: HashMap<String, MemoryRenderBuffer>,
    pub(crate) cursor_buffers: HashMap<CursorShape, CursorCacheEntry>,
    pub(crate) cursor_theme: Option<CursorTheme>,
    pub(crate) client_cursor: Option<CursorShape>,
    pub(crate) client_cursor_surface: Option<WlSurface>,
    pub(crate) cursor_hidden: bool,
    index_socket: Option<String>,
    index_query: String,
    agent_sessions: AgentSessions,
    pub(crate) input_method_popups: Vec<InputMethodPopupSurface>,
}

struct OutputEntry {
    output: Output,
    global: GlobalId,
}

impl App {
    fn mark_window_damage(&mut self, rect: AetherRect) {
        self.mark_damage_rect(rect);
    }

    fn mark_damage_rect(&mut self, rect: AetherRect) {
        if self.full_damage || rect.is_empty() {
            return;
        }
        self.damage.push(Rectangle::new(
            SPoint::from((rect.x, rect.y)),
            SSize::from((rect.width as i32, rect.height as i32)),
        ));
    }

    fn switch_workspace(&mut self, workspace: usize) {
        if self.shell.switch_workspace(workspace) {
            println!("aether-shell: workspace -> {workspace}");
            self.sync_focus();
            self.configure_all();
        }
    }

    fn handle_shortcut(&mut self, modifiers: &ModifiersState, keysym: Keysym) -> bool {
        let escape: Keysym = keysyms::KEY_Escape.into();
        if self.shell.overlay().is_some() && keysym == escape {
            self.toggle_overlay();
            return true;
        }
        if !modifiers.logo {
            return false;
        }

        let one: Keysym = keysyms::KEY_1.into();
        let two: Keysym = keysyms::KEY_2.into();
        let three: Keysym = keysyms::KEY_3.into();
        let space: Keysym = keysyms::KEY_space.into();
        let close: Keysym = keysyms::KEY_q.into();
        let maximize: Keysym = keysyms::KEY_m.into();
        let fullscreen_key: Keysym = keysyms::KEY_f.into();

        if modifiers.shift {
            let target = if keysym == one {
                Some(0)
            } else if keysym == two {
                Some(1)
            } else if keysym == three {
                Some(2)
            } else {
                None
            };
            if let Some(workspace) = target {
                if let Some(id) = self.shell.focused_window() {
                    if self.shell.move_window_to_workspace(id, workspace) {
                        println!(
                            "aether-shell-smithay: moved window {id} -> workspace {workspace}"
                        );
                        self.sync_focus();
                        self.configure_all();
                        return true;
                    }
                }
            }
            return false;
        }

        let workspace_target = if keysym == one {
            Some(0)
        } else if keysym == two {
            Some(1)
        } else if keysym == three {
            Some(2)
        } else {
            None
        };
        // Super+Alt+1/2/3 switches the workspace shown on the output under
        // the pointer, leaving the primary output untouched.
        if modifiers.alt {
            if let Some(workspace) = workspace_target {
                let point = AetherPoint::new(
                    self.pointer_pos.x.round() as i32,
                    self.pointer_pos.y.round() as i32,
                );
                let output = self
                    .shell
                    .output_at(point)
                    .or_else(|| self.shell.primary_output().map(|output| output.id))
                    .unwrap_or(1);
                if self.shell.set_output_workspace(output, workspace) {
                    self.sync_focus();
                    self.configure_all();
                    self.mark_full_damage();
                    println!("aether-shell: output {output} workspace -> {workspace}");
                }
                return true;
            }
            return false;
        }
        if let Some(workspace) = workspace_target {
            self.switch_workspace(workspace);
            return true;
        }
        if keysym == space {
            self.toggle_overlay();
            return true;
        }
        if keysym == close {
            if let Some(id) = self.shell.focused_window() {
                if let Some(toplevel) = self.windows.get(&id) {
                    toplevel.send_close();
                }
            }
            return true;
        }
        if keysym == maximize {
            if let Some(id) = self.shell.focused_window() {
                let maximized = !self.shell.is_maximized(id);
                if self.shell.toggle_maximized(id) {
                    println!("aether-shell-smithay: maximized -> {maximized}");
                    self.sync_focus();
                    self.configure_all();
                }
            }
            return true;
        }
        if keysym == fullscreen_key {
            if let Some(id) = self.shell.focused_window() {
                let fullscreen = !self.shell.is_fullscreen(id);
                if self.shell.toggle_fullscreen(id) {
                    println!("aether-shell-smithay: fullscreen -> {fullscreen}");
                    self.sync_focus();
                    self.configure_all();
                }
            }
            return true;
        }
        false
    }

    pub fn toggle_overlay(&mut self) {
        if self.shell.overlay().is_none() {
            self.refresh_providers();
        }
        self.shell.toggle_overlay();
    }

    fn combined_launchers(&self) -> Vec<PaletteLauncher> {
        let mut launchers = desktop_launchers();
        launchers.extend(external_launchers());
        launchers.extend(self.agent_sessions.launchers().iter().cloned());
        launchers
    }

    fn refresh_launchers(&mut self) {
        let launchers = self.combined_launchers();
        self.shell.set_launchers(launchers);
    }

    fn refresh_providers(&mut self) {
        let launchers = self.combined_launchers();
        let files = if self.index_socket.is_some() {
            self.index_query.clear();
            Vec::new()
        } else {
            file_index()
        };
        println!(
            "aether-shell-smithay: reloaded {} desktop apps, {} files",
            launchers.len(),
            files.len()
        );
        self.shell.set_launchers(launchers);
        self.shell.set_files(files);
    }

    fn refresh_index_results(&mut self) {
        let Some(socket) = self.index_socket.clone() else {
            return;
        };
        if self.shell.overlay().is_none() {
            return;
        }
        let query = self.shell.palette_query().to_string();
        if query == self.index_query {
            return;
        }
        self.index_query = query.clone();
        if query.trim().is_empty() {
            self.shell.set_indexed_files(Vec::new());
            return;
        }

        let Ok(mut stream) = UnixStream::connect(&socket) else {
            println!("aether-shell-smithay: index connect failed {socket:?}");
            return;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
        let request = serde_json::json!({ "query": query, "limit": 8 });
        if writeln!(stream, "{request}").is_err() {
            return;
        }
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            println!("aether-shell-smithay: index read failed");
            return;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            return;
        };
        let files: Vec<PaletteFile> = value
            .get("results")
            .and_then(|results| results.as_array())
            .map(|results| {
                results
                    .iter()
                    .filter_map(|result| {
                        Some(PaletteFile {
                            name: result.get("title")?.as_str()?.to_string(),
                            detail: result
                                .get("subtitle")
                                .and_then(|value| value.as_str())
                                .unwrap_or("indexed file")
                                .to_string(),
                            path: result.get("path")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.shell.set_indexed_files(files);
    }

    fn activate_palette(&mut self) {
        let Some(action) = self.shell.palette_activate() else {
            return;
        };
        println!("aether-shell-smithay: palette action {action:?}");
        match action {
            PaletteAction::SwitchWorkspace(index) => {
                self.shell.close_overlay();
                self.switch_workspace(index);
            }
            PaletteAction::FocusWindow(id) => {
                self.shell.close_overlay();
                let target = self
                    .shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == id)
                    .map(|window| (window.output, window.workspace));
                if let Some((output, workspace)) = target {
                    if self.shell.output_workspace(output) != workspace {
                        let primary = self.shell.primary_output().map(|output| output.id);
                        if Some(output) == primary {
                            self.switch_workspace(workspace);
                        } else {
                            self.shell.set_output_workspace(output, workspace);
                        }
                    }
                    if self.shell.focus_window(id) {
                        self.sync_focus();
                    }
                    self.configure_all();
                }
            }
            PaletteAction::LaunchApp(index) => {
                self.shell.close_overlay();
                if let Some(launcher) = self.shell.launcher(index) {
                    self.spawn_launcher(launcher);
                }
            }
            PaletteAction::SpawnTerminal => {
                self.shell.close_overlay();
                self.spawn_terminal();
            }
            PaletteAction::OpenFile(index) => {
                self.shell.close_overlay();
                if let Some(file) = self.shell.file(index) {
                    self.open_path(&file.path);
                }
            }
            PaletteAction::Close => {
                self.shell.close_overlay();
            }
        }
    }

    fn spawn_terminal(&mut self) {
        let program =
            std::env::var("AETHER_TERMINAL").unwrap_or_else(|_| "weston-terminal".to_string());
        self.spawn_program(&program, &[], "terminal");
    }

    fn spawn_launcher(&mut self, launcher: PaletteLauncher) {
        self.spawn_program(&launcher.program, &launcher.args, &launcher.name);
    }

    fn open_path(&self, path: &str) {
        let opener = std::env::var("AETHER_OPENER").unwrap_or_else(|_| "xdg-open".to_string());
        let mut command = std::process::Command::new(&opener);
        command.arg(path);
        command.env("WAYLAND_DISPLAY", &self.socket_name);
        match command.spawn() {
            Ok(child) => println!("aether-shell-smithay: opened {path} (pid {})", child.id()),
            Err(error) => {
                eprintln!("aether-shell-smithay: opener {opener:?} failed for {path:?}: {error}")
            }
        }
    }

    fn spawn_program(&self, program: &str, args: &[String], label: &str) {
        let mut command = std::process::Command::new(program);
        command.args(args);
        command.env("WAYLAND_DISPLAY", &self.socket_name);
        match command.spawn() {
            Ok(child) => println!(
                "aether-shell-smithay: spawned app {label} (pid {})",
                child.id()
            ),
            Err(error) => {
                eprintln!("aether-shell-smithay: app {label} ({program:?}) failed: {error}")
            }
        }
    }

    fn handle_palette_key(&mut self, keysym: Keysym) {
        let enter: Keysym = keysyms::KEY_Return.into();
        let keypad_enter: Keysym = keysyms::KEY_KP_Enter.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let backspace: Keysym = keysyms::KEY_BackSpace.into();

        if keysym == enter || keysym == keypad_enter {
            self.activate_palette();
            return;
        }
        if keysym == up {
            self.shell.palette_move_selection(-1);
            return;
        }
        if keysym == down {
            self.shell.palette_move_selection(1);
            return;
        }
        if keysym == backspace {
            if self.shell.palette_backspace() {
                println!(
                    "aether-shell-smithay: palette query -> {:?}",
                    self.shell.palette_query()
                );
            }
            return;
        }

        let Some(character) = char::from_u32(xkb::keysym_to_utf32(keysym)) else {
            return;
        };
        if !character.is_control() && self.shell.palette_push_char(character) {
            println!(
                "aether-shell-smithay: palette query -> {:?}",
                self.shell.palette_query()
            );
        }
    }

    fn configure_window(&mut self, id: WindowId) {
        let Some(toplevel) = self.windows.get(&id).cloned() else {
            return;
        };
        let Some(rect) = self
            .shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| window.rect)
        else {
            return;
        };
        let size = (rect.width.max(1) as i32, rect.height.max(1) as i32);
        let fullscreen = self.shell.is_fullscreen(id);
        let config = (size.0, size.1, fullscreen);
        if self.configured.get(&id) == Some(&config) {
            return;
        }
        self.configured.insert(id, config);
        toplevel.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            if fullscreen {
                state.states.set(xdg_toplevel::State::Fullscreen);
            } else {
                state.states.unset(xdg_toplevel::State::Fullscreen);
            }
            state.size = Some(SSize::from(size));
        });
        toplevel.send_configure();
    }

    pub fn configure_all(&mut self) {
        let ids: Vec<WindowId> = self.windows.keys().copied().collect();
        for id in ids {
            self.configure_window(id);
        }
    }

    fn sync_focus(&mut self) {
        self.focus = self.shell.focused_window().and_then(|id| {
            self.windows
                .get(&id)
                .map(|toplevel| toplevel.wl_surface().clone())
        });
    }
}

impl App {
    /// Create the protocol globals, seat, shell policy and listening socket.
    /// Backends add outputs once they know the display modes, then drive input
    /// and rendering through the methods below.
    pub fn new(
        display_handle: &DisplayHandle,
    ) -> Result<(Self, KeyboardHandle<Self>), Box<dyn Error>> {
        let dh = display_handle;

        let compositor_state = CompositorState::new::<App>(dh);
        let shm_state = ShmState::new::<App>(dh, vec![]);
        let mut seat_state = SeatState::new();
        let seat = seat_state.new_wl_seat(dh, "aether");

        let _output_manager_state = OutputManagerState::new_with_xdg_output::<App>(dh);
        let _text_input_manager_state = TextInputManagerState::new::<App>(dh);
        let _input_method_manager_state = InputMethodManagerState::new::<App, _>(dh, |_| true);

        let primary_selection_state = PrimarySelectionState::new::<App>(dh);
        let data_control_state =
            DataControlState::new::<App, _>(dh, Some(&primary_selection_state), |_| true);
        let listener = ListeningSocket::bind_auto("aether", 0..8)?;
        let socket_name = listener
            .socket_name()
            .and_then(|name| name.to_str())
            .unwrap_or("aether-0")
            .to_string();

        let mut state = App {
            display_handle: dh.clone(),
            compositor_state,
            xdg_shell_state: XdgShellState::new::<App>(dh),
            shm_state,
            seat_state,
            data_device_state: DataDeviceState::new::<App>(dh),
            primary_selection_state,
            data_control_state,
            outputs: HashMap::new(),
            seat,
            shell: {
                let mut shell = Shell::new(
                    AetherSize::new(1280, 800),
                    LayoutConfig {
                        master_ratio: 0.55,
                        gap: 10,
                        outer_gap: 16,
                    },
                    120,
                );
                shell.add_workspace("code");
                shell.add_workspace("web");
                shell
            },
            windows: HashMap::new(),
            configured: HashMap::new(),
            focus: None,
            close_requested: false,
            scale: 1.0,
            pointer_pos: smithay::utils::Point::from((0.0, 0.0)),
            resize_drag: None,
            window_drag: None,
            window_resize: None,
            pointer_captured: false,
            modifiers: ModifiersState::default(),
            cursor: CursorShape::Arrow,
            socket_name: socket_name.clone(),
            listener,
            clients: Vec::new(),
            applied_focus: None,
            spawned_test_clients: false,
            full_damage: true,
            damage: Vec::new(),
            icons: HashMap::new(),
            cursor_buffers: HashMap::new(),
            cursor_theme: Some(CursorTheme::discover()),
            client_cursor: None,
            client_cursor_surface: None,
            cursor_hidden: false,
            index_socket: std::env::var("AETHER_INDEX_SOCKET").ok(),
            index_query: String::new(),
            agent_sessions: AgentSessions::bind(None),
            input_method_popups: Vec::new(),
        };

        state.agent_sessions = AgentSessions::bind(
            std::env::var_os("AETHER_AGENT_SOCKET").map(std::path::PathBuf::from),
        );
        state.refresh_launchers();
        let file_count = if state.index_socket.is_some() {
            println!("aether-shell: using index service {:?}", state.index_socket);
            0
        } else {
            let files = file_index();
            let file_count = files.len();
            state.shell.set_files(files);
            file_count
        };
        println!(
            "aether-shell: loaded {} desktop apps, {file_count} files on WAYLAND_DISPLAY={socket_name}",
            state.shell.launchers().len()
        );

        let keyboard = state
            .seat
            .add_keyboard(Default::default(), 200, 200)
            .map_err(|_| "failed to create keyboard")?;
        // Advertise wl_pointer, otherwise `get_pointer()` is `None`.
        let _pointer = state.seat.add_pointer();

        Ok((state, keyboard))
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    pub fn display_handle(&self) -> &DisplayHandle {
        &self.display_handle
    }

    pub fn shell(&self) -> &Shell {
        &self.shell
    }

    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }

    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    pub fn request_close(&mut self) {
        self.close_requested = true;
    }

    /// Register a Wayland output and extend the shell's desktop. The output
    /// must already have its preferred/current mode set by the backend.
    pub fn add_output(&mut self, info: OutputInfo, output: Output) -> GlobalId {
        let global = output.create_global::<App>(&self.display_handle);
        let mut outputs: Vec<OutputInfo> = self
            .shell
            .outputs()
            .iter()
            .filter(|candidate| candidate.id != info.id)
            .cloned()
            .collect();
        outputs.push(info.clone());
        outputs.sort_by_key(|candidate| candidate.id);
        if !outputs.iter().any(|candidate| candidate.primary) {
            if let Some(first) = outputs.first_mut() {
                first.primary = true;
            }
        }
        self.shell.set_outputs(outputs);
        for surface in self
            .windows
            .values()
            .map(|toplevel| toplevel.wl_surface().clone())
            .collect::<Vec<_>>()
        {
            output.enter(&surface);
        }
        println!(
            "aether-shell: output {} {}x{} at {}+{} ({} total)",
            info.name,
            info.rect.width,
            info.rect.height,
            info.rect.x,
            info.rect.y,
            self.shell.outputs().len()
        );
        self.outputs.insert(
            info.id,
            OutputEntry {
                output,
                global: global.clone(),
            },
        );
        self.configure_all();
        global
    }

    /// Remove an output (hotplug). Windows that lived on it fall back to the
    /// primary output in the shell policy.
    pub fn remove_output(&mut self, id: OutputId) -> Option<Output> {
        let entry = self.outputs.remove(&id)?;
        for surface in self
            .windows
            .values()
            .map(|toplevel| toplevel.wl_surface().clone())
            .collect::<Vec<_>>()
        {
            entry.output.leave(&surface);
        }
        self.display_handle.remove_global::<App>(entry.global);
        let remaining: Vec<OutputInfo> = self
            .shell
            .outputs()
            .iter()
            .filter(|candidate| candidate.id != id)
            .cloned()
            .collect();
        if !remaining.is_empty() {
            self.shell.set_outputs(remaining);
        }
        self.sync_focus();
        self.configure_all();
        println!(
            "aether-shell: output {id} removed ({} remaining)",
            self.shell.outputs().len()
        );
        Some(entry.output)
    }

    pub fn output(&self, id: OutputId) -> Option<&Output> {
        self.outputs.get(&id).map(|entry| &entry.output)
    }

    pub fn output_ids(&self) -> Vec<OutputId> {
        self.shell
            .outputs()
            .iter()
            .map(|output| output.id)
            .collect()
    }

    /// Accept pending clients and dispatch all Wayland work.
    pub fn poll_clients(&mut self, display: &mut Display<Self>) -> Result<(), Box<dyn Error>> {
        if let Some(stream) = self.listener.accept()? {
            let client = display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))?;
            println!("aether-shell: client connected");
            self.clients.push(client);
        }
        display.dispatch_clients(self)?;
        display.flush_clients()?;
        if self.agent_sessions.poll() {
            self.refresh_launchers();
            println!(
                "aether-shell: agent sessions -> {} launchers",
                self.shell.launchers().len()
            );
        }
        Ok(())
    }

    /// Spawn configured test clients once, for headless smoke tests.
    pub fn spawn_test_clients(&mut self) {
        if self.spawned_test_clients {
            return;
        }
        let Ok(path) = std::env::var("AETHER_TEST_CLIENT") else {
            return;
        };
        let count = std::env::var("AETHER_TEST_CLIENT_COUNT")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1)
            .max(1);
        for _ in 0..count {
            let mut command = std::process::Command::new(&path);
            command.env("WAYLAND_DISPLAY", &self.socket_name);
            match command.spawn() {
                Ok(child) => println!("aether-shell: spawned test client (pid {})", child.id()),
                Err(error) => eprintln!("aether-shell: test client failed: {error}"),
            }
        }
        self.spawned_test_clients = true;
    }

    /// Send frame callbacks to every client surface after a presented frame.
    pub fn send_frame_callbacks(&self, time: u32) {
        for surface in self.xdg_shell_state.toplevel_surfaces() {
            send_frames_surface_tree(surface.wl_surface(), time);
        }
    }

    /// Apply keyboard focus changes accumulated by input handling.
    pub fn flush_focus(&mut self, keyboard: &KeyboardHandle<Self>) {
        if self.focus != self.applied_focus {
            let focus = self.focus.clone();
            keyboard.set_focus(self, focus.clone(), 0.into());
            self.applied_focus = focus;
            self.mark_full_damage();
        }
    }

    /// Route a key event through xkb, shell shortcuts and the focused client.
    pub fn keyboard_key(
        &mut self,
        keyboard: &KeyboardHandle<Self>,
        key_code: Keycode,
        state: KeyState,
        time: u32,
    ) {
        let pressed = state == KeyState::Pressed;
        keyboard.input::<(), _>(
            self,
            key_code,
            state,
            0.into(),
            time,
            |state, modifiers, keysym| {
                state.modifiers = *modifiers;
                let sym = keysym.modified_sym();
                if pressed && state.handle_shortcut(modifiers, sym) {
                    FilterResult::Intercept(())
                } else if state.shell.overlay().is_some() {
                    if pressed {
                        state.handle_palette_key(sym);
                        state.refresh_index_results();
                    }
                    FilterResult::Intercept(())
                } else {
                    FilterResult::Forward
                }
            },
        );
        self.modifiers = keyboard.modifier_state();
        self.cursor = cursor_for(self);
        self.mark_full_damage();
    }

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

    /// Absolute pointer motion in logical compositor coordinates.
    pub fn pointer_motion(&mut self, position: SPoint<f64, Logical>, time: u32) {
        self.pointer_pos = position;
        self.cursor = cursor_for(self);
        self.mark_full_damage();

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
            if self.shell.resize_floating(resize.id, rect) {
                self.configure_all();
            }
        } else if let Some(drag) = self.window_drag {
            let x = (position.x - drag.offset_x).round() as i32;
            let y = (position.y - drag.offset_y).round() as i32;
            if self.shell.move_floating(drag.id, AetherPoint::new(x, y)) {
                self.configure_all();
            }
        } else if let Some(mut drag) = self.resize_drag {
            let layout = self.shell.layout();
            let output = self
                .shell
                .output_at(AetherPoint::new(
                    position.x.round() as i32,
                    position.y.round() as i32,
                ))
                .or_else(|| self.shell.primary_output().map(|output| output.id))
                .unwrap_or(1);
            let inner = self
                .shell
                .output_bounds(output)
                .inset(layout.outer_gap as i32);
            let usable = (inner.width as i32 - layout.gap as i32).max(1) as f64;
            let ratio = ((position.x - inner.x as f64) / usable) as f32;
            if (ratio - drag.last_ratio).abs() > 0.002 {
                drag.last_ratio = ratio;
                self.resize_drag = Some(drag);
                if self.shell.set_master_ratio(ratio) {
                    self.configure_all();
                }
            }
        } else {
            let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
            if self.shell.overlay().is_some() {
                if let Some(index) = self.shell.palette_row_at(point) {
                    self.shell.palette_select(index);
                }
            }
            let chrome_owns_pointer =
                self.shell.overlay().is_some() || self.shell.workspace_at(point).is_some();
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
        let position = self.pointer_pos;
        let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
        let modal = self.shell.overlay().is_some();
        let over_picker = self.shell.workspace_at(point).is_some();
        let over_divider = self.shell.master_gap_rect().is_some_and(|gap| {
            AetherRect::new(gap.x - 5, gap.y, gap.width + 10, gap.height).contains(point)
        });
        let mouse_button = match button {
            0x110 => Some(MouseButton::Left),
            0x111 => Some(MouseButton::Right),
            0x112 => Some(MouseButton::Middle),
            _ => None,
        };

        if state == ButtonState::Pressed {
            self.pointer_captured = modal || over_picker;
            if over_divider && mouse_button == Some(MouseButton::Left) {
                self.resize_drag = Some(ResizeDrag {
                    last_ratio: self.shell.master_ratio(),
                });
                self.pointer_captured = true;
                println!("aether-shell: resize drag started");
            } else if let Some(workspace) = self.shell.workspace_at(point) {
                self.switch_workspace(workspace);
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
                    if self.shell.snap_window(drag.id, zone) {
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
            if self.resize_drag.take().is_some() {
                println!(
                    "aether-shell: master ratio -> {:.2}",
                    self.shell.master_ratio()
                );
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
        self.mark_full_damage();
    }

    /// Pointer axis (scroll) event.
    pub fn pointer_axis(&mut self, horizontal: f64, vertical: f64, source: AxisSource, time: u32) {
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
        self.mark_full_damage();
    }

    /// Replace the scale used by the backend (winit HiDPI).
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = if scale > 0.0 { scale } else { 1.0 };
    }

    /// Whether compositor content changed since the last presented frame.
    pub fn needs_frame(&self) -> bool {
        self.full_damage || !self.damage.is_empty()
    }

    pub fn mark_full_damage(&mut self) {
        self.full_damage = true;
        self.damage.clear();
    }

    /// Drain accumulated damage for presenters that need explicit regions.
    pub fn take_damage(&mut self) -> (bool, Vec<Rectangle<i32, Physical>>) {
        let full = self.full_damage;
        let damage = if full {
            Vec::new()
        } else {
            std::mem::take(&mut self.damage)
        };
        self.full_damage = false;
        (full, damage)
    }
}

fn send_frames_surface_tree(surface: &wl_surface::WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surface, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

smithay::delegate_xdg_shell!(App);
smithay::delegate_compositor!(App);
smithay::delegate_shm!(App);
smithay::delegate_seat!(App);
smithay::delegate_data_device!(App);
smithay::delegate_primary_selection!(App);
smithay::delegate_data_control!(App);
smithay::delegate_output!(App);
smithay::delegate_text_input_manager!(App);
smithay::delegate_input_method_manager!(App);
