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
use aether_shell::{Shell, WindowId, WindowSpec, WORKSPACE_INDICATOR_BAR_HEIGHT};
use aether_text::{text_width, TextBitmap};
use smithay::backend::allocator::Fourcc;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputEvent, KeyboardKeyEvent,
    PointerAxisEvent, PointerButtonEvent,
};
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::{draw_render_elements, on_commit_buffer_handler};
use smithay::backend::renderer::{Color32F, Frame, Renderer};
use smithay::backend::winit::{self, WinitEvent};
use smithay::input::keyboard::{keysyms, FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{Client, Display, ListeningSocket};
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::reexports::winit::window::WindowAttributes;
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
    let args: Vec<String> = std::env::args().collect();
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

    let (mut backend, mut winit) = winit::init_from_attributes::<GlesRenderer>(
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(1280.0, 800.0))
            .with_title("Aether OS"),
    )?;
    let start_time = std::time::Instant::now();
    state.window_size = backend.window_size();
    state.scale = backend.scale_factor();
    state
        .shell
        .set_viewport(logical_size(state.window_size, state.scale));
    state.configure_all();
    if args.iter().any(|arg| arg == "--overlay") || std::env::var("AETHER_OVERLAY").is_ok() {
        state.toggle_overlay();
        println!(
            "aether-shell-smithay: overlay enabled: {:?}",
            state.shell.overlay_rect()
        );
    }
    let keyboard = state
        .seat
        .add_keyboard(Default::default(), 200, 200)
        .map_err(|_| "failed to create keyboard")?;
    // Advertise wl_pointer. Without this `get_pointer()` is `None` and any
    // motion/button event over a client panics.
    let _pointer = state.seat.add_pointer();

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
                    |state, modifiers, keysym| {
                        if state.handle_shortcut(modifiers, keysym.modified_sym()) {
                            FilterResult::Intercept(())
                        } else if state.shell.overlay().is_some() {
                            // The overlay is modal: keystrokes do not leak to the
                            // focused client while it is open.
                            FilterResult::Intercept(())
                        } else {
                            FilterResult::Forward
                        }
                    },
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
                let chrome_owns_pointer =
                    state.shell.overlay().is_some() || state.shell.workspace_at(point).is_some();
                if chrome_owns_pointer {
                    // The overlay and workspace picker are compositor-owned
                    // surfaces: never focus or move a client underneath them.
                } else if let Some(id) = state.shell.window_at(point) {
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
            WinitEvent::Input(InputEvent::PointerAxis { event }) => {
                let source = event.source();
                let horizontal = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.0
                });
                let vertical = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.0
                });
                let mut axis = AxisFrame::new(event.time_msec()).source(source);
                if horizontal != 0.0 {
                    axis = axis.value(Axis::Horizontal, horizontal);
                }
                if vertical != 0.0 {
                    axis = axis.value(Axis::Vertical, vertical);
                }
                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        axis = axis.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        axis = axis.stop(Axis::Vertical);
                    }
                }
                let pointer = state.seat.get_pointer().expect("seat pointer");
                pointer.axis(&mut state, axis);
                pointer.frame(&mut state);
            }
            WinitEvent::Input(InputEvent::PointerButton { event }) => {
                let position = state.pointer_pos;
                let point = AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
                let modal = state.shell.overlay().is_some();
                let over_picker = state.shell.workspace_at(point).is_some();
                if event.state() == ButtonState::Pressed {
                    if let Some(workspace) = state.shell.workspace_at(point) {
                        state.switch_workspace(workspace);
                    } else if modal {
                        // Clicking outside the panel dismisses the overlay;
                        // clicks inside are consumed by the compositor.
                        if state
                            .shell
                            .overlay_rect()
                            .is_some_and(|rect| !rect.contains(point))
                        {
                            state.toggle_overlay();
                        }
                    } else if let Some(id) = state.shell.window_at(point) {
                        state.shell.focus_window(id);
                        state.sync_focus();
                    }
                }
                if !modal && !over_picker {
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

            // Compositor-owned chrome above clients: palette structure,
            // workspace indicators, labels and hints. Solid blocks carry the
            // structure; aether-text supplies the glyphs.
            let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
            let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();

            if let Some(rect) = state.shell.overlay_rect() {
                chrome.push((rect, [0.05, 0.24, 0.29, 0.96]));
                chrome.push((
                    AetherRect::new(rect.x, rect.y, rect.width, 36.min(rect.height)),
                    [0.09, 0.42, 0.5, 1.0],
                ));
                labels.push((
                    "Aether Palette".into(),
                    rect.x + 14,
                    rect.y + 10,
                    2,
                    [235, 252, 255, 255],
                ));
                let hint = "Super+Space";
                labels.push((
                    hint.into(),
                    rect.x + rect.width as i32 - text_width(hint, 1) as i32 - 14,
                    rect.y + 14,
                    1,
                    [179, 235, 250, 255],
                ));

                if rect.height > 96 {
                    let input = AetherRect::new(
                        rect.x + 16,
                        rect.y + 52,
                        rect.width.saturating_sub(32),
                        28,
                    );
                    chrome.push((input, [0.03, 0.12, 0.16, 1.0]));
                    labels.push((
                        "Search files, windows, agents...".into(),
                        input.x + 10,
                        input.y + 10,
                        1,
                        [122, 173, 189, 255],
                    ));

                    let rows = [
                        "1  Open documentation",
                        "2  Switch workspace",
                        "3  Start agent session",
                    ];
                    for (row, text) in rows.iter().enumerate() {
                        let y = rect.y + 96 + row as i32 * 48;
                        if y + 32 < rect.y + rect.height as i32 {
                            let row_rect =
                                AetherRect::new(rect.x + 16, y, rect.width.saturating_sub(32), 32);
                            chrome.push((row_rect, [0.05, 0.31, 0.37, 1.0]));
                            labels.push((
                                (*text).into(),
                                row_rect.x + 10,
                                row_rect.y + 12,
                                1,
                                [217, 247, 255, 255],
                            ));
                        }
                    }

                    let footer = "Esc close   Enter run   Super+Space toggle";
                    labels.push((
                        footer.into(),
                        rect.x + 16,
                        rect.y + rect.height as i32 - 22,
                        1,
                        [128, 184, 199, 255],
                    ));
                }
            }

            let names = state.shell.workspace_names();
            for workspace in 0..state.shell.workspace_count() {
                let Some(chip) = state.shell.workspace_indicator_rect(workspace) else {
                    continue;
                };
                let active = workspace == state.shell.active_workspace();
                chrome.push((
                    AetherRect::new(chip.x, chip.y, chip.width, WORKSPACE_INDICATOR_BAR_HEIGHT),
                    if active {
                        [0.22, 0.84, 1.0, 1.0]
                    } else {
                        [0.12, 0.3, 0.35, 0.9]
                    },
                ));
                let name = names.get(workspace).map(String::as_str).unwrap_or("");
                labels.push((
                    format!("{} {name}", workspace + 1),
                    chip.x + 4,
                    chip.y + 12,
                    1,
                    if active {
                        [56, 214, 255, 255]
                    } else {
                        [102, 158, 173, 255]
                    },
                ));
            }

            let text_elements: Vec<MemoryRenderBufferRenderElement<GlesRenderer>> = labels
                .iter()
                .filter_map(|(text, x, y, scale, color)| {
                    let bitmap = TextBitmap::render(text, *scale, *color);
                    if bitmap.is_empty() {
                        return None;
                    }
                    let buffer = MemoryRenderBuffer::from_slice(
                        bitmap.pixels(),
                        Fourcc::Abgr8888,
                        (bitmap.width() as i32, bitmap.height() as i32),
                        1,
                        Transform::Normal,
                        None,
                    );
                    MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        (*x as f64, *y as f64),
                        &buffer,
                        None,
                        None,
                        None,
                        Kind::Unspecified,
                    )
                    .ok()
                })
                .collect();

            let mut frame = renderer.render(&mut framebuffer, size, Transform::Flipped180)?;
            frame.clear(Color32F::new(0.01, 0.06, 0.08, 1.0), &[damage])?;
            // Client surfaces are painted over the shell background.
            if !elements.is_empty() {
                draw_render_elements(&mut frame, 1.0, &elements, &[damage])?;
            }
            if !chrome.is_empty() {
                let buffers: Vec<SolidColorBuffer> = chrome
                    .iter()
                    .map(|(rect, color)| {
                        SolidColorBuffer::new((rect.width as i32, rect.height as i32), *color)
                    })
                    .collect();
                let elements: Vec<SolidColorRenderElement> = buffers
                    .iter()
                    .zip(chrome.iter())
                    .map(|(buffer, (rect, _))| {
                        SolidColorRenderElement::from_buffer(
                            buffer,
                            (rect.x, rect.y),
                            1.0f64,
                            1.0,
                            Kind::Unspecified,
                        )
                    })
                    .collect();
                draw_render_elements::<GlesRenderer, f64, SolidColorRenderElement>(
                    &mut frame,
                    1.0,
                    &elements,
                    &[damage],
                )?;
            }
            if !text_elements.is_empty() {
                draw_render_elements::<
                    GlesRenderer,
                    f64,
                    MemoryRenderBufferRenderElement<GlesRenderer>,
                >(&mut frame, 1.0, &text_elements, &[damage])?;
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
    fn switch_workspace(&mut self, workspace: usize) {
        if self.shell.switch_workspace(workspace) {
            println!("aether-shell-smithay: workspace -> {workspace}");
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
        false
    }

    fn toggle_overlay(&mut self) {
        if self.shell.overlay().is_some() {
            self.shell.set_overlay(None);
        } else {
            let viewport = self.shell.viewport();
            let width = 680.min(viewport.width.saturating_sub(64)).max(1);
            let height = 420.min(viewport.height.saturating_sub(64)).max(1);
            let x = ((viewport.width.saturating_sub(width)) / 2) as i32;
            let y = ((viewport.height.saturating_sub(height)) / 2) as i32;
            self.shell
                .set_overlay(Some(AetherRect::new(x, y, width, height)));
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
