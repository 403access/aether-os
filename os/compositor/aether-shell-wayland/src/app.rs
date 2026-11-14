//! Backend-agnostic Wayland state for the Aether shell.
//!
//! `App` owns the Wayland display objects, seat, shell policy, client window
//! map and render composition. Backends (winit and native DRM) feed it input
//! and a renderer; it does not know how pixels reach the display.

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use crate::agents::{parse_launchers, AgentSessions};
use crate::chat_demo::ChatDemo;
use crate::damage::buffer_damage_to_logical;
use crate::index_worker::{should_apply, IndexWorker};
use crate::keymap::{lookup_profile, KeyProfile, Shortcut};
use crate::status::{StatusSnapshot, SAMPLE_INTERVAL};
use crate::topbar::{self, TopAction};
use aether_audio::{LatencyPreset, SystemAudio};
use crate::task::{TaskManagerState, TaskTab};
use aether_system::SystemMonitor;
use aether_cursor::{CursorShape, CursorTheme};
use aether_geometry::{Point as AetherPoint, Rect as AetherRect, Size as AetherSize};
use aether_layout::LayoutConfig;
use aether_shell::{
    audio::AudioAction, parse_palette_query, OutputId, OutputInfo, PaletteAction, PaletteFile,
    PaletteLauncher, PaletteMode, Shell, SnapZone, WindowId, WindowSpec, MIN_FLOATING_HEIGHT,
    MIN_FLOATING_WIDTH,
};
use aether_text::{text_width, FontRenderer};
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
        self.shell.focus_window(id);
        self.sync_focus();
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

/// Cache key for rasterized rounded panels: size, radius, fill and border.
pub(crate) type PanelCacheKey = (u32, u32, u32, [u8; 4], Option<[u8; 4]>);

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
    pub(crate) text_cache: HashMap<(String, u32, [u8; 4], bool), MemoryRenderBuffer>,
    pub(crate) text_rasterizations: u64,
    pub(crate) font: Option<FontRenderer>,
    pub(crate) chrome_icons: HashMap<(String, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_panels: HashMap<PanelCacheKey, Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_rounds: HashMap<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_tops: HashMap<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_background: HashMap<(u32, u32), Arc<MemoryRenderBuffer>>,
    pub(crate) cursor_buffers: HashMap<CursorShape, CursorCacheEntry>,
    pub(crate) cursor_theme: Option<CursorTheme>,
    pub(crate) client_cursor: Option<CursorShape>,
    pub(crate) client_cursor_surface: Option<WlSurface>,
    pub(crate) cursor_hidden: bool,
    index_socket: Option<String>,
    index_query: String,
    index_worker: Option<IndexWorker>,
    agent_sessions: AgentSessions,
    status: StatusSnapshot,
    status_cpu: Option<crate::status::CpuTimes>,
    chat_demo: Option<ChatDemo>,
    pub(crate) shortcuts_open: bool,
    pub(crate) task_manager: Option<TaskManagerState>,
    task_monitor: SystemMonitor,
    task_last: std::time::Instant,
    pub(crate) key_profile: KeyProfile,
    audio_backend: SystemAudio,
    status_last: std::time::Instant,
    status_enabled: bool,
    frame_time_ms: f32,
    pub(crate) input_method_popups: Vec<InputMethodPopupSurface>,
    last_input_to_damage_ns: u64,
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
            self.mark_full_damage();
        }
    }

    fn switch_workspace_on(&mut self, output: OutputId, workspace: usize) {
        if self.shell.switch_workspace_on(output, workspace) {
            println!("aether-shell: output {output} workspace -> {workspace}");
            self.sync_focus();
            self.configure_all();
            self.mark_full_damage();
        }
    }

    fn handle_shortcut(&mut self, modifiers: &ModifiersState, keysym: Keysym) -> bool {
        let escape: Keysym = keysyms::KEY_Escape.into();
        if self.shortcuts_open && keysym == escape {
            self.close_shortcuts();
            return true;
        }
        if self.shell.overlay().is_some() && keysym == escape {
            self.toggle_overlay();
            return true;
        }
        let Some(shortcut) = lookup_profile(
            self.key_profile,
            modifiers.logo,
            modifiers.ctrl,
            modifiers.shift,
            modifiers.alt,
            keysym,
        ) else {
            return false;
        };
        match shortcut {
            Shortcut::TogglePalette => self.toggle_overlay(),
            Shortcut::ToggleExplorer => self.toggle_explorer(),
            Shortcut::ToggleChat => self.toggle_chat(),
            Shortcut::ToggleAudio => self.toggle_audio(),
            Shortcut::ToggleShortcuts => self.toggle_shortcuts(),
            Shortcut::NewTerminal => {
                self.spawn_terminal();
                println!("aether-shell: shortcut new terminal");
            }
            Shortcut::FocusNext | Shortcut::FocusPrevious => {
                let delta = if shortcut == Shortcut::FocusNext {
                    1
                } else {
                    -1
                };
                if let Some(id) = self.shell.focus_mru(delta) {
                    self.sync_focus();
                    self.configure_all();
                    self.mark_full_damage();
                    println!("aether-shell: mru focus {id}");
                }
            }
            Shortcut::SwitchWorkspace(workspace) => self.switch_workspace(workspace),
            Shortcut::MoveWindowToWorkspace(workspace) => {
                if let Some(id) = self.shell.focused_window() {
                    if self.shell.move_window_to_workspace(id, workspace) {
                        println!(
                            "aether-shell-smithay: moved window {id} -> workspace {workspace}"
                        );
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
            Shortcut::SwitchOutputWorkspace(workspace) => {
                let output = self.output_under_pointer();
                if self.shell.set_output_workspace(output, workspace) {
                    self.sync_focus();
                    self.configure_all();
                    self.mark_full_damage();
                    println!("aether-shell: output {output} workspace -> {workspace}");
                }
            }
            Shortcut::CloseWindow => {
                if let Some(id) = self.shell.focused_window() {
                    if let Some(toplevel) = self.windows.get(&id) {
                        toplevel.send_close();
                    }
                }
            }
            Shortcut::Maximize => {
                if let Some(id) = self.shell.focused_window() {
                    let maximized = !self.shell.is_maximized(id);
                    if self.shell.toggle_maximized(id) {
                        println!("aether-shell-smithay: maximized -> {maximized}");
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
            Shortcut::Fullscreen => {
                if let Some(id) = self.shell.focused_window() {
                    let fullscreen = !self.shell.is_fullscreen(id);
                    if self.shell.toggle_fullscreen(id) {
                        println!("aether-shell-smithay: fullscreen -> {fullscreen}");
                        self.sync_focus();
                        self.configure_all();
                        self.mark_full_damage();
                    }
                }
            }
        }
        true
    }

    /// Output currently under the pointer, falling back to the primary.
    pub fn output_under_pointer(&self) -> OutputId {
        let point = AetherPoint::new(
            self.pointer_pos.x.round() as i32,
            self.pointer_pos.y.round() as i32,
        );
        self.shell
            .output_at(point)
            .or_else(|| self.shell.primary_output().map(|output| output.id))
            .unwrap_or(1)
    }

    pub fn toggle_overlay(&mut self) {
        if self.shell.overlay().is_none() {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.shell.close_audio();
            self.shell.close_explorer();
            self.shell.close_chat();
            self.refresh_providers();
        }
        let output = self.output_under_pointer();
        self.shell.toggle_overlay_on(output);
        self.mark_full_damage();
    }

    fn toggle_chat(&mut self) {
        if self.shell.chat_open() {
            self.shell.close_chat();
        } else {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.shell.close_overlay();
            self.shell.close_explorer();
            self.shell.close_audio();
            let _ = self.shell.open_chat();
        }
        self.mark_full_damage();
    }

    fn top_bar(&self, output: OutputId) -> topbar::TopBar {
        topbar::layout(self.shell.output_bounds(output))
    }

    fn top_bar_visible(&self, output: OutputId) -> bool {
        self.shell.fullscreen_window_on(output).is_none()
            && !self.shell.explorer_open()
            && !self.shell.chat_open()
            && !self.shell.audio_open()
            && !self.shortcuts_open
            && self.shell.overlay().is_none()
    }

    fn top_shortcut_at(&self, point: AetherPoint) -> Option<TopAction> {
        let output = self
            .shell
            .output_at(point)
            .or_else(|| self.shell.primary_output().map(|output| output.id))?;
        if !self.top_bar_visible(output) {
            return None;
        }
        topbar::hit(&self.top_bar(output), point.x, point.y)
    }

    fn activate_top_shortcut(&mut self, action: TopAction) {
        match action {
            TopAction::Palette => {
                println!("aether-shell: top shortcut palette");
                self.toggle_overlay();
            }
            TopAction::Terminal => {
                self.spawn_terminal();
                println!("aether-shell: top shortcut terminal");
            }
            TopAction::Files => {
                println!("aether-shell: top shortcut files");
                self.toggle_explorer();
            }
            TopAction::Chat => {
                println!("aether-shell: top shortcut chat");
                self.toggle_chat();
            }
            TopAction::Audio => {
                println!("aether-shell: top shortcut audio");
                self.toggle_audio();
            }
            TopAction::Shortcuts => {
                println!("aether-shell: top shortcut keys");
                self.toggle_shortcuts();
            }
        }
    }

    fn open_task_manager(&mut self) {
        self.shortcuts_open = false;
        self.shell.close_audio();
        self.shell.close_explorer();
        self.shell.close_chat();
        let snapshot = self.task_monitor.sample();
        println!(
            "aether-shell: task manager opened ({} processes, {} users)",
            snapshot.process_count,
            snapshot.users.len()
        );
        self.task_manager = Some(TaskManagerState::new(snapshot));
        self.task_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub fn task_manager_open(&self) -> bool {
        self.task_manager.is_some()
    }

    fn close_task_manager(&mut self) {
        if self.task_manager.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn task_rect(&self) -> AetherRect {
        self.shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800))
    }

    pub(crate) fn task_pane_rect(&self) -> AetherRect {
        let bounds = self.task_rect();
        AetherRect::new(
            bounds.x + 16,
            bounds.y + 74,
            bounds.width.saturating_sub(32),
            bounds.height.saturating_sub(74 + 52),
        )
    }

    pub(crate) fn task_rows_top(&self) -> i32 {
        self.task_pane_rect().y + 56 + 30
    }

    pub(crate) fn task_visible_rows(&self) -> usize {
        let pane = self.task_pane_rect();
        (pane.height.saturating_sub(56 + 30 + 30) / 28).max(1) as usize
    }

    pub(crate) fn task_row_rect(&self, index: usize) -> AetherRect {
        let pane = self.task_pane_rect();
        AetherRect::new(
            pane.x + 8,
            self.task_rows_top() + index as i32 * 28,
            pane.width.saturating_sub(16),
            26,
        )
    }

    pub(crate) fn task_close_rect(&self) -> AetherRect {
        let rect = self.task_rect();
        AetherRect::new(rect.right() as i32 - 42, rect.y + 20, 24, 24)
    }

    pub(crate) fn task_row_at(&self, point: AetherPoint) -> Option<usize> {
        let state = self.task_manager.as_ref()?;
        let rows = match state.tab() {
            TaskTab::Processes => state.rows_len(),
            TaskTab::Users => state.snapshot().users.len(),
            TaskTab::Resources => 0,
        };
        let start = state.scroll();
        (0..rows.min(self.task_visible_rows())).find(|offset| {
            self.task_row_rect(*offset)
                .contains(AetherPoint::new(point.x, point.y))
        }).map(|offset| start + offset)
    }

    pub fn poll_task_manager(&mut self) {
        if self.task_manager.is_none() || self.task_last.elapsed() < std::time::Duration::from_secs(1) {
            return;
        }
        let snapshot = self.task_monitor.sample();
        if let Some(state) = self.task_manager.as_mut() {
            state.refresh(snapshot);
        }
        self.task_last = std::time::Instant::now();
        self.mark_full_damage();
    }

    pub fn task_refresh_in(&self) -> Option<std::time::Duration> {
        if self.task_manager.is_none() {
            return None;
        }
        Some(
            std::time::Duration::from_secs(1)
                .saturating_sub(self.task_last.elapsed())
                .max(std::time::Duration::from_millis(50)),
        )
    }

    fn handle_task_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let tab: Keysym = keysyms::KEY_Tab.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let left: Keysym = keysyms::KEY_Left.into();
        let right: Keysym = keysyms::KEY_Right.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let refresh: Keysym = keysyms::KEY_r.into();
        let space: Keysym = keysyms::KEY_space.into();
        if keysym == escape {
            self.close_task_manager();
            return;
        }
        let visible = self.task_visible_rows();
        if keysym == tab {
            let shift = self.modifiers.shift;
            if let Some(state) = self.task_manager.as_mut() {
                state.set_tab(if shift { state.tab().previous() } else { state.tab().next() });
            }
            self.mark_task_damage();
        } else if keysym == up {
            if let Some(state) = self.task_manager.as_mut() {
                state.move_selection(-1, visible);
            }
            self.mark_task_damage();
        } else if keysym == down {
            if let Some(state) = self.task_manager.as_mut() {
                state.move_selection(1, visible);
            }
            self.mark_task_damage();
        } else if keysym == page_up {
            if let Some(state) = self.task_manager.as_mut() {
                state.page(-1, visible);
            }
            self.mark_task_damage();
        } else if keysym == page_down {
            if let Some(state) = self.task_manager.as_mut() {
                state.page(1, visible);
            }
            self.mark_task_damage();
        } else if keysym == space {
            if let Some(state) = self.task_manager.as_mut() {
                state.toggle_collapse(visible);
            }
            self.mark_task_damage();
        } else if keysym == left {
            if let Some(state) = self.task_manager.as_mut() {
                state.collapse(visible);
            }
            self.mark_task_damage();
        } else if keysym == right {
            if let Some(state) = self.task_manager.as_mut() {
                state.expand(visible);
            }
            self.mark_task_damage();
        } else if keysym == refresh {
            self.task_last = std::time::Instant::now() - std::time::Duration::from_secs(2);
            self.poll_task_manager();
            self.mark_task_damage();
        }
    }

    fn mark_task_damage(&mut self) {
        if self.task_manager.is_some() {
            let rect = self.task_rect();
            self.mark_damage_rect(rect);
        }
    }

    fn toggle_shortcuts(&mut self) {
        if self.shortcuts_open {
            self.close_shortcuts();
        } else {
            self.shell.close_overlay();
            self.shell.close_explorer();
            self.shell.close_chat();
            self.shell.close_audio();
            self.shortcuts_open = true;
            println!(
                "aether-shell: shortcuts overlay opened ({} bindings)",
                self.key_profile.entries().len()
            );
            self.mark_full_damage();
        }
    }

    fn close_shortcuts(&mut self) {
        if self.shortcuts_open {
            self.shortcuts_open = false;
            self.mark_full_damage();
        }
    }

    pub(crate) fn shortcuts_rect(&self) -> AetherRect {
        let bounds = self
            .shell
            .primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| AetherRect::new(0, 0, 1280, 800));
        let width = 760.min(bounds.width.saturating_sub(32)).max(1);
        let height = 560.min(bounds.height.saturating_sub(32)).max(1);
        AetherRect::new(
            bounds.x + (bounds.width.saturating_sub(width) / 2) as i32,
            bounds.y + (bounds.height.saturating_sub(height) / 2) as i32,
            width,
            height,
        )
    }

    fn mark_shortcuts_damage(&mut self) {
        if self.shortcuts_open {
            let rect = self.shortcuts_rect();
            self.mark_damage_rect(rect);
        }
    }

    fn toggle_audio(&mut self) {
        if self.shell.audio_open() {
            self.shell.close_audio();
            self.mark_full_damage();
            return;
        }
        self.shortcuts_open = false;
        self.close_task_manager();
        self.shell.close_overlay();
        self.shell.close_explorer();
        self.shell.close_chat();
        match self.audio_backend.snapshot() {
            Ok(snapshot) => {
                println!(
                    "aether-shell: audio overlay opened ({} routes, {:?}, preset {})",
                    snapshot.routes.len(),
                    snapshot.source,
                    snapshot.preset.name()
                );
                self.shell.open_audio(snapshot);
            }
            Err(error) => {
                eprintln!("aether-shell: audio unavailable: {error}");
                return;
            }
        }
        self.mark_full_damage();
    }

    fn handle_audio_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let preset: Keysym = keysyms::KEY_p.into();
        if keysym == escape {
            self.shell.close_audio();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.audio_move_selection(-1);
            self.mark_audio_damage();
        } else if keysym == down {
            self.shell.audio_move_selection(1);
            self.mark_audio_damage();
        } else if keysym == enter {
            self.activate_audio();
            self.mark_audio_damage();
        } else if keysym == preset {
            self.cycle_audio_preset();
            self.mark_audio_damage();
        }
    }

    fn activate_audio(&mut self) {
        let label = self
            .shell
            .audio_layer()
            .and_then(|layer| layer.selected_route())
            .map(|route| route.label().to_string())
            .unwrap_or_default();
        let fixture = self
            .shell
            .audio_layer()
            .is_some_and(|layer| layer.is_fixture());
        if let Some(action) = self.shell.audio_activate() {
            match action {
                AudioAction::SetDefault { kind, id } => {
                    if !fixture {
                        if let Err(error) = self.audio_backend.switch(kind, id) {
                            eprintln!("aether-shell: audio switch failed: {error}");
                        }
                    }
                    println!(
                        "aether-shell: audio default {} -> {} ({id})",
                        format!("{kind:?}").to_lowercase(),
                        label
                    );
                }
                AudioAction::SetPreset(preset) => self.apply_audio_preset(preset),
            }
        }
    }

    fn cycle_audio_preset(&mut self) {
        if let Some(AudioAction::SetPreset(preset)) = self.shell.audio_cycle_preset() {
            self.apply_audio_preset(preset);
        }
    }

    fn apply_audio_preset(&self, preset: LatencyPreset) {
        if let Err(error) = self.audio_backend.set_preset(preset) {
            eprintln!("aether-shell: audio preset failed: {error}");
        }
        println!(
            "aether-shell: audio preset -> {} ({})",
            preset.name(),
            preset.quantum()
        );
    }

    fn mark_audio_damage(&mut self) {
        if self.shell.audio_open() {
            let rect = self.shell.audio_rect();
            self.mark_damage_rect(rect);
        }
    }

    fn handle_chat_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        if keysym == escape {
            self.shell.close_chat();
            self.mark_full_damage();
        } else if keysym == up {
            self.shell.chat_scroll_by(-1);
        } else if keysym == down {
            self.shell.chat_scroll_by(1);
        } else if keysym == page_up {
            self.shell.chat_page(-1);
        } else if keysym == page_down {
            self.shell.chat_page(1);
        }
    }

    /// Stream a token into the chat drawer (external producers/tests).
    pub fn push_chat_token(&mut self, token: &str) {
        if !self.shell.chat_open() {
            let _ = self.shell.open_chat();
        }
        self.shell.chat_push_token(token);
        self.mark_chat_damage();
    }

    fn poll_chat_demo(&mut self) {
        let tokens = self
            .chat_demo
            .as_ref()
            .map(|demo| demo.drain())
            .unwrap_or_default();
        if tokens.is_empty() {
            return;
        }
        for token in tokens {
            self.shell.chat_push_token(&token);
        }
        self.mark_chat_damage();
    }

    fn toggle_explorer(&mut self) {
        if self.shell.explorer_open() {
            self.shell.close_explorer();
        } else {
            self.shortcuts_open = false;
            self.close_task_manager();
            self.shell.close_overlay();
            self.shell.close_chat();
            self.shell.close_audio();
            let _ = self.shell.open_explorer();
            if let Some(worker) = self.index_worker.as_mut() {
                let generation = worker.request_explorer();
                println!("aether-shell: explorer index generation {generation}");
            }
        }
        self.mark_full_damage();
    }

    fn handle_explorer_key(&mut self, keysym: Keysym) {
        let escape: Keysym = keysyms::KEY_Escape.into();
        let enter: Keysym = keysyms::KEY_Return.into();
        let up: Keysym = keysyms::KEY_Up.into();
        let down: Keysym = keysyms::KEY_Down.into();
        let page_up: Keysym = keysyms::KEY_Page_Up.into();
        let page_down: Keysym = keysyms::KEY_Page_Down.into();
        let backspace: Keysym = keysyms::KEY_BackSpace.into();
        if keysym == escape {
            self.shell.close_explorer();
            self.mark_full_damage();
            return;
        }
        if keysym == enter {
            if let Some(file) = self.shell.explorer_selected_file() {
                self.open_path(&file.path);
            }
            self.shell.close_explorer();
            self.mark_full_damage();
            return;
        }
        if keysym == up {
            self.shell.explorer_move(-1);
            return;
        }
        if keysym == down {
            self.shell.explorer_move(1);
            return;
        }
        if keysym == page_up {
            self.shell.explorer_page(-1);
            return;
        }
        if keysym == page_down {
            self.shell.explorer_page(1);
            return;
        }
        if keysym == backspace {
            self.shell.explorer_backspace();
            return;
        }
        if let Some(character) = char::from_u32(xkb::keysym_to_utf32(keysym)) {
            if !character.is_control() {
                self.shell.explorer_push_char(character);
            }
        }
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
        if self.index_socket.is_none() || self.shell.overlay().is_none() {
            return;
        }
        let (mode, term) = parse_palette_query(self.shell.palette_query());
        if !matches!(mode, PaletteMode::All | PaletteMode::File) {
            return;
        }
        if term == self.index_query {
            return;
        }
        self.index_query = term.clone();
        if term.is_empty() {
            if let Some(worker) = self.index_worker.as_mut() {
                worker.cancel();
            }
            self.shell.set_indexed_files(Vec::new());
            self.mark_full_damage();
            return;
        }
        if let Some(worker) = self.index_worker.as_mut() {
            let generation = worker.request(term);
            println!("aether-shell: index query generation {generation}");
        }
    }

    /// Drain the latest index result without blocking. Results for superseded
    /// queries are discarded; identical result sets do not recompose.
    pub fn poll_index_results(&mut self) {
        let Some((generation, files)) = self.index_worker.as_ref().and_then(|worker| worker.poll())
        else {
            return;
        };
        let current = self
            .index_worker
            .as_ref()
            .map(|worker| worker.current_generation())
            .unwrap_or(0);
        if !should_apply(generation, current) {
            return;
        }
        if self.shell.files() == files.as_slice() {
            return;
        }
        println!(
            "aether-shell: index results generation {generation}: {} rows",
            files.len()
        );
        self.shell.set_indexed_files(files);
        self.mark_overlay_damage();
    }

    fn activate_palette(&mut self) {
        let Some(action) = self.shell.palette_activate() else {
            return;
        };
        println!("aether-shell-smithay: palette action {action:?}");
        self.mark_full_damage();
        match action {
            PaletteAction::SwitchWorkspace(index) => {
                let output = self.shell.overlay_output().unwrap_or(1);
                self.shell.close_overlay();
                self.switch_workspace_on(output, index);
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
            PaletteAction::OpenTaskManager => {
                self.shell.close_overlay();
                self.open_task_manager();
            }
            PaletteAction::OpenExplorer => {
                self.shell.close_overlay();
                self.toggle_explorer();
            }
            PaletteAction::OpenChat => {
                self.shell.close_overlay();
                self.toggle_chat();
            }
            PaletteAction::OpenAudio => {
                self.shell.close_overlay();
                self.toggle_audio();
            }
            PaletteAction::OpenShortcuts => {
                self.shell.close_overlay();
                self.toggle_shortcuts();
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
            std::env::var("AETHER_TERMINAL").unwrap_or_else(|_| "aether-terminal".to_string());
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
            text_cache: HashMap::new(),
            text_rasterizations: 0,
            font: FontRenderer::discover(),
            chrome_icons: HashMap::new(),
            chrome_panels: HashMap::new(),
            chrome_rounds: HashMap::new(),
            chrome_tops: HashMap::new(),
            chrome_background: HashMap::new(),
            cursor_buffers: HashMap::new(),
            cursor_theme: Some(CursorTheme::discover()),
            client_cursor: None,
            client_cursor_surface: None,
            cursor_hidden: false,
            index_socket: std::env::var("AETHER_INDEX_SOCKET").ok(),
            index_query: String::new(),
            index_worker: None,
            agent_sessions: AgentSessions::bind(None),
            status: StatusSnapshot::default(),
            status_cpu: None,
            chat_demo: None,
            shortcuts_open: false,
            task_manager: None,
            task_monitor: SystemMonitor::default(),
            task_last: std::time::Instant::now(),
            key_profile: KeyProfile::from_env(),
            audio_backend: SystemAudio::new(),
            status_last: std::time::Instant::now(),
            status_enabled: std::env::var("AETHER_STATUS")
                .map(|value| value != "0")
                .unwrap_or(true),
            frame_time_ms: 0.0,
            input_method_popups: Vec::new(),
            last_input_to_damage_ns: 0,
        };

        state.agent_sessions = AgentSessions::bind(
            std::env::var_os("AETHER_AGENT_SOCKET").map(std::path::PathBuf::from),
        );
        if let Some(socket) = state.index_socket.clone() {
            state.index_worker = Some(IndexWorker::spawn(std::path::PathBuf::from(socket)));
        }
        state.status = StatusSnapshot::sample(None, state.agent_sessions.session_count(), 0.0);
        state.status_cpu = crate::status::read_cpu();
        state.status_last = std::time::Instant::now();
        if std::env::var_os("AETHER_CHAT_DEMO").is_some() {
            let _ = state.shell.open_chat();
            state.chat_demo = Some(ChatDemo::spawn());
            state.mark_full_damage();
        }
        state.refresh_launchers();
        let file_count = if state.index_socket.is_some() {
            println!("aether-shell: using index service {:?}", state.index_socket);
            0
        } else {
            let files = file_index();
            let file_count = files.len();
            state.shell.set_files(files);
            println!("aether-shell: indexed {file_count} files");
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
            "aether-shell: output {} {}x{}@{}Hz at {}+{} ({} total)",
            info.name,
            info.rect.width,
            info.rect.height,
            info.refresh_hz,
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
        self.mark_full_damage();
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
        self.poll_index_results();
        self.poll_chat_demo();
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
        let started = self.begin_input();
        let pressed = state == KeyState::Pressed;
        let old_cursor = self.cursor;
        let old_overlay = self.shell.overlay_rect();
        let mut shortcut = false;
        let mut palette_key = false;
        let mut explorer_key = false;
        let mut chat_key_pressed = false;
        let mut audio_key = false;
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
                    shortcut = true;
                    FilterResult::Intercept(())
                } else if state.shortcuts_open {
                    FilterResult::Intercept(())
                } else if state.task_manager_open() {
                    if pressed {
                        state.handle_task_key(sym);
                    }
                    FilterResult::Intercept(())
                } else if state.shell.overlay().is_some() {
                    if pressed {
                        state.handle_palette_key(sym);
                        state.refresh_index_results();
                        palette_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.explorer_open() {
                    if pressed {
                        state.handle_explorer_key(sym);
                        explorer_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.audio_open() {
                    if pressed {
                        state.handle_audio_key(sym);
                        audio_key = true;
                    }
                    FilterResult::Intercept(())
                } else if state.shell.chat_open() {
                    if pressed {
                        state.handle_chat_key(sym);
                        chat_key_pressed = true;
                    }
                    FilterResult::Intercept(())
                } else {
                    FilterResult::Forward
                }
            },
        );
        self.modifiers = keyboard.modifier_state();
        let new_cursor = cursor_for(self);
        self.cursor = new_cursor;
        if new_cursor != old_cursor {
            self.mark_cursor_damage(self.pointer_pos);
        }
        if shortcut {
            // Shortcuts may move/close/raise windows or switch workspaces.
            self.mark_full_damage();
        } else if explorer_key {
            self.mark_explorer_damage();
        } else if chat_key_pressed {
            self.mark_chat_damage();
        } else if audio_key {
            self.mark_audio_damage();
        } else if self.shortcuts_open {
            self.mark_shortcuts_damage();
        } else if self.task_manager_open() {
            self.mark_task_damage();
        } else if palette_key {
            let new_overlay = self.shell.overlay_rect();
            if let Some(rect) = old_overlay {
                self.mark_damage_rect(rect);
            }
            if let Some(rect) = new_overlay {
                if Some(rect) != old_overlay {
                    self.mark_damage_rect(rect);
                }
            } else {
                // Enter/activation closed the palette and may have launched or
                // switched workspace.
                self.mark_full_damage();
            }
        }
        self.end_input(started);
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

    /// Replace the scale used by the backend (winit HiDPI).
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = if scale > 0.0 { scale } else { 1.0 };
    }

    /// Timestamp at the start of input handling. Damage is added by each
    /// handler for exactly what it changed.
    pub fn begin_input(&mut self) -> std::time::Instant {
        std::time::Instant::now()
    }

    /// Damage around a cursor position (old and new positions are both
    /// marked by callers), independent of window content.
    pub fn mark_cursor_damage(&mut self, position: SPoint<f64, Logical>) {
        const RADIUS: i32 = 32;
        let x = position.x.round() as i32;
        let y = position.y.round() as i32;
        self.mark_damage_rect(AetherRect::new(
            x - RADIUS,
            y - RADIUS,
            (RADIUS * 2) as u32,
            (RADIUS * 2) as u32,
        ));
    }

    fn mark_overlay_damage(&mut self) {
        if let Some(rect) = self.shell.overlay_rect() {
            self.mark_damage_rect(rect);
        }
    }

    fn mark_chat_damage(&mut self) {
        if self.shell.chat_open() {
            let rect = self.shell.chat_rect();
            self.mark_damage_rect(rect);
        }
    }

    fn mark_explorer_damage(&mut self) {
        if self.shell.explorer_open() {
            let rect = self.shell.explorer_rect();
            self.mark_damage_rect(rect);
        }
    }

    fn window_rect(&self, id: WindowId) -> Option<AetherRect> {
        self.shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .map(|window| window.rect)
    }

    /// Record input arrival → damage commit latency.
    pub fn end_input(&mut self, start: std::time::Instant) {
        self.last_input_to_damage_ns = start.elapsed().as_nanos() as u64;
    }

    pub fn last_input_to_damage_ns(&self) -> u64 {
        self.last_input_to_damage_ns
    }

    pub fn text_rasterizations(&self) -> u64 {
        self.text_rasterizations
    }

    /// Pixel size for a chrome text style. 0=small, 1=body, 2=title.
    pub fn text_px(scale: u32) -> f32 {
        match scale {
            0 => aether_text::font::SMALL_PX,
            1 => aether_text::font::BODY_PX,
            2 => aether_text::font::TITLE_PX,
            _ => aether_text::font::HEADING_PX,
        }
    }

    pub fn text_width(&self, text: &str, scale: u32) -> u32 {
        match self.font.as_ref() {
            Some(font) => font.measure(text, Self::text_px(scale), scale >= 2).0,
            None => text_width(text, scale.max(1)),
        }
    }

    pub fn font_is_loaded(&self) -> bool {
        self.font.is_some()
    }

    pub fn set_frame_time_ms(&mut self, frame_ms: f32) {
        self.frame_time_ms = frame_ms;
    }

    /// True when the status bar should be sampled and drawn. Fullscreen hides
    /// it and stops polling.
    pub fn status_visible(&self) -> bool {
        self.status_enabled
            && !self
                .shell
                .outputs()
                .iter()
                .all(|output| self.shell.fullscreen_window_on(output.id).is_some())
    }

    pub fn status(&self) -> &StatusSnapshot {
        &self.status
    }

    pub(crate) fn poll_status(&mut self) {
        // Sample only while a frame is already due. A timer-driven sample would
        // itself create damage and break the zero-frame idle gate; values
        // refresh on the next input-driven frame instead.
        if !self.status_visible()
            || !self.needs_frame()
            || self.status_last.elapsed() < SAMPLE_INTERVAL
        {
            return;
        }
        self.status = StatusSnapshot::sample(
            self.status_cpu,
            self.agent_sessions.session_count(),
            self.frame_time_ms,
        );
        self.status_cpu = crate::status::read_cpu();
        self.status_last = std::time::Instant::now();
    }

    /// Whether compositor content changed since the last presented frame.
    pub fn needs_frame(&self) -> bool {
        self.full_damage || !self.damage.is_empty()
    }

    /// Damage for one output in local physical coordinates. `None` means the
    /// output is unaffected by the pending damage and must not be composed.
    /// A `true` first element requests a full-output clear.
    pub fn output_damage(
        &self,
        output: Option<OutputId>,
    ) -> Option<(bool, Vec<Rectangle<i32, Physical>>)> {
        if !self.needs_frame() {
            return None;
        }
        let bounds = match output {
            Some(id) => self.shell.output_bounds(id),
            None => AetherRect::from_size(self.shell.viewport()),
        };
        damage_for_bounds(self.full_damage, &self.damage, bounds)
    }

    /// Mark an output change for deferred repaint (new output, mode change).
    pub fn mark_output_damage(&mut self) {
        self.mark_full_damage();
    }

    pub fn mark_full_damage(&mut self) {
        if std::env::var_os("AETHER_DAMAGE_TRACE").is_some() {
            eprintln!(
                "FULL_DAMAGE at\n{}",
                std::backtrace::Backtrace::force_capture()
            );
        }
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

/// Clip global damage to one output and translate it into local coordinates.
/// Pure so the presenter's damage contract can be unit-tested without EGL.
pub(crate) fn damage_for_bounds(
    full: bool,
    damage: &[Rectangle<i32, Physical>],
    bounds: AetherRect,
) -> Option<(bool, Vec<Rectangle<i32, Physical>>)> {
    let local_size = SSize::from((bounds.width.max(1) as i32, bounds.height.max(1) as i32));
    if full {
        return Some((true, vec![Rectangle::from_size(local_size)]));
    }
    let clip = Rectangle::new(
        SPoint::from((bounds.x, bounds.y)),
        SSize::from((bounds.width.max(1) as i32, bounds.height.max(1) as i32)),
    );
    let mut local = Vec::new();
    for rect in damage {
        if let Some(hit) = rect.intersection(clip) {
            local.push(Rectangle::new(
                SPoint::from((hit.loc.x - bounds.x, hit.loc.y - bounds.y)),
                hit.size,
            ));
        }
    }
    if local.is_empty() {
        None
    } else {
        Some((false, local))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rectangle<i32, Physical> {
        Rectangle::new(SPoint::from((x, y)), SSize::from((width, height)))
    }

    #[test]
    fn damage_is_clipped_and_translated_per_output() {
        let damage = vec![rect(90, 10, 40, 20), rect(400, 10, 10, 10)];
        let bounds = AetherRect::new(100, 0, 100, 100);
        let (full, local) = damage_for_bounds(false, &damage, bounds).unwrap();
        assert!(!full);
        assert_eq!(local, vec![rect(0, 10, 30, 20)]);
    }

    #[test]
    fn unaffected_output_reports_no_damage() {
        let damage = vec![rect(0, 0, 20, 20)];
        let bounds = AetherRect::new(100, 0, 100, 100);
        assert!(damage_for_bounds(false, &damage, bounds).is_none());
    }

    #[test]
    fn full_damage_is_local_to_the_output() {
        let bounds = AetherRect::new(100, 50, 100, 80);
        let (full, local) = damage_for_bounds(true, &[], bounds).unwrap();
        assert!(full);
        assert_eq!(local, vec![rect(0, 0, 100, 80)]);
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;
    use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
    use smithay::utils::Transform;

    fn test_app() -> (Display<App>, App) {
        let dir = std::env::temp_dir().join(format!("aether-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        std::env::remove_var("AETHER_INDEX_SOCKET");
        std::env::remove_var("AETHER_AGENT_SOCKET");
        let display: Display<App> = Display::new().expect("display");
        let (app, _keyboard) = App::new(&display.handle()).expect("app");
        (display, app)
    }

    fn output(name: &str) -> Output {
        let output = Output::new(
            name.to_string(),
            PhysicalProperties {
                size: (300, 200).into(),
                subpixel: Subpixel::Unknown,
                make: "Aether".into(),
                model: name.into(),
            },
        );
        let mode = OutputMode {
            size: (800, 600).into(),
            refresh: 120_000,
        };
        output.set_preferred(mode);
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output
    }

    #[test]
    fn take_damage_drains_once() {
        let (_display, mut app) = test_app();
        app.mark_full_damage();
        assert!(app.needs_frame());
        let (full, damage) = app.take_damage();
        assert!(full);
        assert!(damage.is_empty());
        assert!(!app.needs_frame());
        let (full, damage) = app.take_damage();
        assert!(!full);
        assert!(damage.is_empty());

        app.mark_damage_rect(AetherRect::new(10, 20, 30, 40));
        let (full, damage) = app.take_damage();
        assert!(!full);
        assert_eq!(damage.len(), 1);
        assert!(!app.needs_frame());
    }

    #[test]
    fn add_and_remove_output_updates_the_shell() {
        let (_display, mut app) = test_app();
        app.add_output(
            OutputInfo {
                id: 2,
                name: "right".into(),
                rect: AetherRect::new(1280, 0, 800, 600),
                scale: 1.0,
                refresh_hz: 60,
                physical_size_mm: (0, 0),
                primary: false,
            },
            output("right"),
        );
        assert!(app.shell().outputs().iter().any(|o| o.id == 2));
        assert_eq!(
            app.shell().output_bounds(2),
            AetherRect::new(1280, 0, 800, 600)
        );
        assert!(app.remove_output(2).is_some());
        assert!(!app.shell().outputs().iter().any(|o| o.id == 2));
    }

    #[test]
    fn output_damage_is_clipped_to_each_output() {
        let (_display, mut app) = test_app();
        app.add_output(
            OutputInfo {
                id: 2,
                name: "right".into(),
                rect: AetherRect::new(1280, 0, 800, 600),
                scale: 1.0,
                refresh_hz: 60,
                physical_size_mm: (0, 0),
                primary: false,
            },
            output("right"),
        );
        app.take_damage();
        app.mark_damage_rect(AetherRect::new(1290, 10, 30, 40));
        let (full, damage) = app.output_damage(Some(2)).expect("second output damage");
        assert!(!full);
        assert_eq!(
            damage,
            vec![Rectangle::new((10, 10).into(), (30, 40).into())]
        );
        assert!(app.output_damage(Some(1)).is_none());
    }
}
