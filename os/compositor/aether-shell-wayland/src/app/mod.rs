//! Backend-agnostic Wayland state for the Aether shell.
//!
//! `App` owns the Wayland display objects, seat, shell policy, client window
//! map and render composition. Backends (winit and native DRM) feed it input
//! and a renderer; it does not know how pixels reach the display.
//!
//! The implementation is split by responsibility: protocol handlers live in
//! `handlers`, input routing and the keymap in `input`, overlay surfaces in
//! `overlays`, workers and launchers in `agents`, `chat_demo`, `index_worker`
//! and `launcher`, sampling in `status`, damage bookkeeping in `damage`, and
//! window/output policy in `windows` and `outputs`.

mod agents;
mod chat_bridge;
mod chat_demo;
mod control;
mod damage;
mod handlers;
pub(crate) mod index_worker;
mod input;
mod launcher;
mod outputs;
pub(crate) mod overlays;
pub(crate) mod status;
pub(crate) mod system;
pub(crate) mod system_info;
pub(crate) mod text_style;
pub(crate) mod tray;
mod windows;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use aether_audio::SystemAudio;
use aether_cache::BudgetCache;
use aether_cursor::{CursorShape, CursorTheme};
use aether_geometry::Size as AetherSize;
use aether_layout::LayoutConfig;
use aether_shell::{OutputId, Shell, WindowId};
use aether_system::SystemMonitor;
use aether_text::FontRenderer;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::input::keyboard::{KeyboardHandle, ModifiersState};
use smithay::input::{Seat, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{Client, Display, DisplayHandle, ListeningSocket};
use smithay::utils::{Physical, Rectangle};
use smithay::wayland::compositor::{
    with_surface_tree_downward, CompositorState, SurfaceAttributes, TraversalAction,
};
use smithay::wayland::input_method::{
    InputMethodManagerState, PopupSurface as InputMethodPopupSurface,
};
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::selection::primary_selection::PrimarySelectionState;
use smithay::wayland::selection::wlr_data_control::DataControlState;
use smithay::wayland::shell::xdg::{ToplevelSurface, XdgShellState};
use smithay::wayland::shm::ShmState;
use smithay::wayland::text_input::TextInputManagerState;

use self::agents::AgentSessions;
use self::chat_bridge::ChatBridge;
use self::chat_demo::ChatDemo;
use self::control::ControlServer;
use self::index_worker::IndexWorker;
use self::input::{KeyProfile, TitleDrag};
use self::overlays::calendar::CalendarState;
use self::overlays::menubar::MenuId;
use self::overlays::sandboxes::SandboxPanelState;
use self::overlays::task_manager::TaskManagerState;
use self::status::StatusSnapshot;
use self::system::SystemState;

use self::handlers::ClientState;
use self::input::{WindowDrag, WindowResize};
use self::launcher::file_index;
use self::outputs::OutputEntry;

pub(crate) use self::windows::WINDOW_FRAME_HEIGHT;

pub(crate) struct CursorCacheEntry {
    pub(crate) buffer: MemoryRenderBuffer,
    pub(crate) hotspot_x: i32,
    pub(crate) hotspot_y: i32,
}

/// Cache key for rasterized rounded panels: size, radius, fill and border.
pub(crate) type PanelCacheKey = (u32, u32, u32, [u8; 4], Option<[u8; 4]>);

/// Byte budgets for compositor chrome buffers, in CPU-side RGBA bytes. GPU
/// textures mirror these allocations, so the effective cost is roughly twice
/// the budget. Values are sized to hold the live overlay set with headroom
/// while refusing to retain history from earlier interactions.
const TEXT_CACHE_BYTES: usize = 8 * 1024 * 1024;
const ICON_CACHE_BYTES: usize = 1024 * 1024;
const PANEL_CACHE_BYTES: usize = 6 * 1024 * 1024;
const ROUND_CACHE_BYTES: usize = 2 * 1024 * 1024;
const TOP_CACHE_BYTES: usize = 1024 * 1024;
/// Backgrounds are one per active output size. The budget covers two QHD
/// outputs (or one 4K plus a secondary) and evicts resize history.
const BACKGROUND_CACHE_BYTES: usize = 48 * 1024 * 1024;

pub(crate) type GradientKey = (u32, u32, u32, [u8; 4], [u8; 4]);

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
    title_drag: Option<TitleDrag>,
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
    pub(crate) text_cache: BudgetCache<(String, u32, [u8; 4], bool, i32), MemoryRenderBuffer>,
    pub(crate) text_rasterizations: u64,
    pub(crate) font: Option<FontRenderer>,
    pub(crate) chrome_icons: BudgetCache<(String, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_panels: BudgetCache<PanelCacheKey, Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_rounds: BudgetCache<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_outlines: BudgetCache<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_shadows: BudgetCache<(u32, u32, u32, bool), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_gradients: BudgetCache<GradientKey, Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_waves: BudgetCache<(u32, u32, u32, u32, u8), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_tops: BudgetCache<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_bottoms: BudgetCache<(u32, u32, u32, [u8; 4]), Arc<MemoryRenderBuffer>>,
    pub(crate) chrome_background:
        BudgetCache<(u32, u32), (crate::render::BackgroundStyle, Arc<MemoryRenderBuffer>)>,
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
    status_cpu: Option<status::CpuTimes>,
    chat_demo: Option<ChatDemo>,
    chat_bridge: ChatBridge,
    control: Option<ControlServer>,
    pub(crate) shortcuts_open: bool,
    /// Open menu bar popover, if any.
    pub(crate) menu_open: Option<MenuId>,
    /// Open Calendar app, if any.
    pub(crate) calendar: Option<CalendarState>,
    /// First-party app window (About, Network, Battery, ...).
    pub(crate) system: SystemState,
    /// Dock item under the pointer, for the hover lift and label.
    pub(crate) dock_hover: Option<overlays::dock::DockAction>,
    /// The demo ships its dock disabled; Settings → Desktop & Dock can turn it
    /// on. `AETHER_SHOW_DOCK` forces it on for captures that need the dock.
    pub(crate) dock_enabled: bool,
    /// Last tide-frame tick, so the rim animates at ~12 fps.
    wave_last: Option<std::time::Instant>,
    /// Cached size and mtime for indexed paths shown by the explorer.
    pub(crate) file_meta: HashMap<String, (u64, Option<u64>)>,
    pub(crate) task_manager: Option<TaskManagerState>,
    pub(crate) sandboxes: Option<SandboxPanelState>,
    pub(crate) sandbox_last: std::time::Instant,
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
                        top_gap: 38,
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
            title_drag: None,
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
            text_cache: BudgetCache::new(TEXT_CACHE_BYTES),
            text_rasterizations: 0,
            font: FontRenderer::discover(),
            chrome_icons: BudgetCache::new(ICON_CACHE_BYTES),
            chrome_panels: BudgetCache::new(PANEL_CACHE_BYTES),
            chrome_rounds: BudgetCache::new(ROUND_CACHE_BYTES),
            chrome_outlines: BudgetCache::new(ROUND_CACHE_BYTES),
            chrome_shadows: BudgetCache::new(PANEL_CACHE_BYTES),
            chrome_gradients: BudgetCache::new(ROUND_CACHE_BYTES),
            chrome_waves: BudgetCache::new(PANEL_CACHE_BYTES),
            chrome_tops: BudgetCache::new(TOP_CACHE_BYTES),
            chrome_bottoms: BudgetCache::new(TOP_CACHE_BYTES),
            chrome_background: BudgetCache::new(BACKGROUND_CACHE_BYTES),
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
            chat_bridge: ChatBridge::bind(None),
            control: None,
            shortcuts_open: false,
            menu_open: None,
            calendar: None,
            system: SystemState::new(),
            dock_hover: None,
            dock_enabled: false,
            wave_last: None,
            file_meta: HashMap::new(),
            task_manager: None,
            sandboxes: None,
            sandbox_last: std::time::Instant::now(),
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

        let theme = aether_shell::theme::ThemeSettings::load();
        println!(
            "aether-shell: theme {} · {} · {}",
            theme.mode.label(),
            theme.tone.label(),
            theme.background.label()
        );
        state.shell.set_theme_settings(theme);

        state.agent_sessions = AgentSessions::bind(
            std::env::var_os("AETHER_AGENT_SOCKET").map(std::path::PathBuf::from),
        );
        state.chat_bridge =
            ChatBridge::bind(std::env::var_os("AETHER_CHAT_SOCKET").map(std::path::PathBuf::from));
        state.control = ControlServer::bind(
            std::env::var_os("AETHER_CONTROL_SOCKET").map(std::path::PathBuf::from),
        );
        if let Some(socket) = state.index_socket.clone() {
            state.index_worker = Some(IndexWorker::spawn(std::path::PathBuf::from(socket)));
        }
        state.status = StatusSnapshot::sample(None, state.agent_sessions.session_count(), 0.0);
        state.status_cpu = status::read_cpu();
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
        self.poll_chat_bridge();
        self.poll_sandboxes();
        self.pump_control();
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
        // Zero is meaningful: parity captures compare an empty desktop.
        let count = std::env::var("AETHER_TEST_CLIENT_COUNT")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
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

    /// Replace the scale used by the backend (winit HiDPI).
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = if scale > 0.0 { scale } else { 1.0 };
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
