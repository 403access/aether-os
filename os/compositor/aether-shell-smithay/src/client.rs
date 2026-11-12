//! Client-capable Wayland compositor on Smithay's winit backend.
//!
//! Adapted from Smithay's MIT-licensed `examples/minimal.rs` (v0.7.0), then
//! reduced/annotated for Aether. It accepts xdg-shell toplevels backed by shm
//! buffers, forwards keyboard input to the focused surface and presents client
//! surfaces in a real window.
//!
//! This is the first client slice. Mapping to `aether-shell`'s FramePlan,
//! resizing/configure policy, pointer hit-testing and udev/DRM are next.

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use aether_geometry::{Point as AetherPoint, Rect as AetherRect, Size as AetherSize};
use aether_layout::LayoutConfig;
use aether_shell::{Shell, WindowId, WindowSpec};
use smithay::backend::input::{
    AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyboardKeyEvent, PointerButtonEvent,
};
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::{draw_render_elements, on_commit_buffer_handler};
use smithay::backend::renderer::{Color32F, Frame, Renderer};
use smithay::backend::winit::{self, WinitEvent};
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{Client, Display, ListeningSocket};
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::utils::{Physical, Rectangle, Serial, Size as SSize, Transform, SERIAL_COUNTER};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    with_surface_tree_downward, CompositorClientState, CompositorHandler, CompositorState,
    SurfaceAttributes, TraversalAction,
};
use smithay::wayland::selection::data_device::{
    ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
};
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};
use smithay::wayland::shm::{ShmHandler, ShmState};

pub fn run() -> Result<(), Box<dyn Error>> {
    let max_frames = parse_frames();
    let mut display: Display<App> = Display::new()?;
    let dh = display.handle();

    let compositor_state = CompositorState::new::<App>(&dh);
    let shm_state = ShmState::new::<App>(&dh, vec![]);
    let mut seat_state = SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "aether");

    let mut state = App {
        compositor_state,
        xdg_shell_state: XdgShellState::new::<App>(&dh),
        shm_state,
        seat_state,
        data_device_state: DataDeviceState::new::<App>(&dh),
        seat,
        shell: Shell::new(
            AetherSize::new(1280, 800),
            LayoutConfig {
                master_ratio: 0.55,
                gap: 10,
                outer_gap: 16,
            },
            120,
        ),
        windows: HashMap::new(),
        configured: HashMap::new(),
        focus: None,
        close_requested: false,
        window_size: SSize::from((1280, 800)),
        scale: 1.0,
        pointer_pos: smithay::utils::Point::from((0.0, 0.0)),
    };

    let listener = ListeningSocket::bind_auto("aether", 0..8)?;
    let socket_name = listener
        .socket_name()
        .and_then(|name| name.to_str())
        .unwrap_or("aether-0")
        .to_string();
    let mut clients = Vec::new();

    let (mut backend, mut winit) = winit::init::<GlesRenderer>()?;
    let start_time = std::time::Instant::now();
    state.window_size = backend.window_size();
    state.scale = backend.scale_factor();
    state
        .shell
        .set_viewport(logical_size(state.window_size, state.scale));
    state.configure_all();
    let keyboard = state
        .seat
        .add_keyboard(Default::default(), 200, 200)
        .map_err(|_| "failed to create keyboard")?;

    let mut spawned_client = false;
    let mut frames = 0u32;
    let mut applied_focus: Option<WlSurface> = None;

    println!(
        "aether-shell-smithay: compositor on WAYLAND_DISPLAY={socket_name} (focus with pointer)"
    );

    loop {
        let status = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, scale_factor } => {
                state.window_size = size;
                state.scale = scale_factor;
                state.shell.set_viewport(logical_size(size, scale_factor));
                state.configure_all();
            }
            WinitEvent::Focus(_) => {}
            WinitEvent::Input(InputEvent::Keyboard { event }) => {
                keyboard.input::<(), _>(
                    &mut state,
                    event.key_code(),
                    event.state(),
                    0.into(),
                    0,
                    |_, _, _| FilterResult::Forward,
                );
            }
            WinitEvent::Input(InputEvent::PointerMotionAbsolute { event }) => {
                let logical = state.shell.viewport();
                let position = event.position_transformed(SSize::from((
                    logical.width as i32,
                    logical.height as i32,
                )));
                state.pointer_pos = position;
                let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
                if let Some(id) = state.shell.window_at(point) {
                    if state.shell.focus_window(id) {
                        state.sync_focus();
                    }
                    if let Some(rect) = state
                        .shell
                        .windows()
                        .into_iter()
                        .find(|window| window.id == id)
                        .map(|window| window.rect)
                    {
                        let local = smithay::utils::Point::from((
                            position.x - rect.x as f64,
                            position.y - rect.y as f64,
                        ));
                        let surface = state
                            .windows
                            .get(&id)
                            .map(|toplevel| toplevel.wl_surface().clone());
                        let serial = SERIAL_COUNTER.next_serial();
                        let time = Event::time_msec(&event);
                        let pointer = state.seat.get_pointer().expect("seat pointer");
                        pointer.motion(
                            &mut state,
                            surface.map(|surface| (surface, local)),
                            &MotionEvent {
                                location: local,
                                serial,
                                time,
                            },
                        );
                        pointer.frame(&mut state);
                    }
                }
            }
            WinitEvent::Input(InputEvent::PointerButton { event }) => {
                let position = state.pointer_pos;
                let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
                if event.state() == ButtonState::Pressed {
                    if let Some(id) = state.shell.window_at(point) {
                        state.shell.focus_window(id);
                        state.sync_focus();
                    }
                }
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let pointer = state.seat.get_pointer().expect("seat pointer");
                pointer.button(
                    &mut state,
                    &ButtonEvent {
                        button: event.button_code(),
                        state: event.state(),
                        serial,
                        time,
                    },
                );
                pointer.frame(&mut state);
            }
            WinitEvent::CloseRequested => {
                state.close_requested = true;
            }
            _ => {}
        });

        if matches!(status, PumpStatus::Exit(_)) || state.close_requested {
            return Ok(());
        }

        // Apply focus changes once per loop to keep the borrow simple.
        if state.focus != applied_focus {
            let focus = state.focus.clone();
            keyboard.set_focus(&mut state, focus.clone(), 0.into());
            applied_focus = focus;
        }

        let size = backend.window_size();
        let damage = Rectangle::from_size(size);

        {
            let (renderer, mut framebuffer) = backend.bind()?;
            let rects: HashMap<WindowId, AetherRect> = state
                .shell
                .windows()
                .into_iter()
                .filter(|window| window.workspace == state.shell.active_workspace())
                .map(|window| (window.id, window.rect))
                .collect();
            let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = state
                .windows
                .iter()
                .filter_map(|(id, toplevel)| rects.get(id).map(|rect| (rect, toplevel)))
                .flat_map(|(rect, toplevel)| {
                    render_elements_from_surface_tree(
                        renderer,
                        toplevel.wl_surface(),
                        (rect.x, rect.y),
                        1.0,
                        1.0,
                        Kind::Unspecified,
                    )
                })
                .collect();

            let mut frame = renderer.render(&mut framebuffer, size, Transform::Flipped180)?;
            frame.clear(Color32F::new(0.01, 0.06, 0.08, 1.0), &[damage])?;
            // Client surfaces are painted over the shell background.
            if !elements.is_empty() {
                draw_render_elements(&mut frame, 1.0, &elements, &[damage])?;
            }
            let _ = frame.finish()?;
        }

        for surface in state.xdg_shell_state.toplevel_surfaces() {
            send_frames_surface_tree(
                surface.wl_surface(),
                start_time.elapsed().as_millis() as u32,
            );
        }

        if let Some(stream) = listener.accept()? {
            let client = display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))?;
            println!("aether-shell-smithay: client connected");
            clients.push(client);
        }

        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;
        backend.submit(Some(&[damage]))?;

        if !spawned_client {
            if let Ok(path) = std::env::var("AETHER_TEST_CLIENT") {
                let count = std::env::var("AETHER_TEST_CLIENT_COUNT")
                    .ok()
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(1)
                    .max(1);
                for _ in 0..count {
                    let mut command = std::process::Command::new(&path);
                    command.env("WAYLAND_DISPLAY", &socket_name);
                    match command.spawn() {
                        Ok(child) => {
                            println!(
                                "aether-shell-smithay: spawned test client (pid {})",
                                child.id()
                            );
                        }
                        Err(error) => {
                            eprintln!("aether-shell-smithay: test client failed: {error}");
                        }
                    }
                }
                spawned_client = true;
            }
        }

        frames += 1;
        if max_frames.is_some_and(|limit| frames >= limit) {
            return Ok(());
        }
        state.configure_all();
    }
}

fn parse_frames() -> Option<u32> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--frames" {
            return args.next().and_then(|value| value.parse().ok());
        }
    }
    None
}

impl BufferHandler for App {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for App {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let mut spec = WindowSpec::new("Wayland client", "wayland.client");
        spec.preferred_size = AetherSize::new(800, 600);
        let id = self.shell.add_window(spec);
        self.windows.insert(id, surface.clone());
        self.focus = Some(surface.wl_surface().clone());
        self.configure_window(id);
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
    }
}

impl ShmHandler for App {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for App {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

struct App {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,
    shell: Shell,
    windows: HashMap<WindowId, ToplevelSurface>,
    configured: HashMap<WindowId, (i32, i32)>,
    focus: Option<WlSurface>,
    close_requested: bool,
    window_size: SSize<i32, Physical>,
    scale: f64,
    pointer_pos: smithay::utils::Point<f64, smithay::utils::Logical>,
}

impl App {
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
        if self.configured.get(&id) == Some(&size) {
            return;
        }
        self.configured.insert(id, size);
        toplevel.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.size = Some(SSize::from(size));
        });
        toplevel.send_configure();
    }

    fn configure_all(&mut self) {
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

fn logical_size(physical: SSize<i32, Physical>, scale: f64) -> AetherSize {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    AetherSize::new(
        ((physical.w as f64) / scale).round().max(1.0) as u32,
        ((physical.h as f64) / scale).round().max(1.0) as u32,
    )
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
