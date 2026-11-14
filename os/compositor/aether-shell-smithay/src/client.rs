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
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use crate::agents::{parse_launchers, AgentSessions};
use crate::damage::buffer_damage_to_logical;
use aether_cursor::{render as render_cursor, CursorShape, CursorTheme};
use aether_geometry::{Point as AetherPoint, Rect as AetherRect, Size as AetherSize};
use aether_layout::LayoutConfig;
use aether_shell::{
    PaletteAction, PaletteFile, PaletteLauncher, Shell, SnapZone, WindowId, WindowSpec,
    MIN_FLOATING_HEIGHT, MIN_FLOATING_WIDTH, PALETTE_VISIBLE_ROWS, WORKSPACE_INDICATOR_BAR_HEIGHT,
};
use aether_text::{text_width, TextBitmap};
use smithay::backend::allocator::Fourcc;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputEvent, KeyState,
    KeyboardKeyEvent, MouseButton, PointerAxisEvent, PointerButtonEvent,
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
use smithay::input::keyboard::{keysyms, xkb, FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, CursorIcon as PointerIcon, CursorImageStatus, MotionEvent,
};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{
    Client, Display, DisplayHandle, ListeningSocket, Resource,
};
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::reexports::winit::window::WindowAttributes;
use smithay::utils::{
    Logical, Physical, Point as SPoint, Rectangle, Serial, Size as SSize, Transform, SERIAL_COUNTER,
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
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::wayland::text_input::{TextInputManagerState, TextInputSeat};

pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let max_frames = parse_frames();
    let mut display: Display<App> = Display::new()?;
    let dh = display.handle();

    let compositor_state = CompositorState::new::<App>(&dh);
    let shm_state = ShmState::new::<App>(&dh, vec![]);
    let mut seat_state = SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "aether");

    let _output_manager_state = OutputManagerState::new_with_xdg_output::<App>(&dh);
    let _text_input_manager_state = TextInputManagerState::new::<App>(&dh);
    let _input_method_manager_state = InputMethodManagerState::new::<App, _>(&dh, |_| true);
    let output = Output::new(
        "Aether-1".to_string(),
        PhysicalProperties {
            size: (340, 190).into(),
            subpixel: Subpixel::Unknown,
            make: "Aether".to_string(),
            model: "Kestrel".to_string(),
        },
    );
    let mode = OutputMode {
        size: (1280, 800).into(),
        refresh: 60_000,
    };
    output.set_preferred(mode);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );
    output.create_global::<App>(&dh);
    println!("aether-shell-smithay: output Aether-1 1280x800@60");

    let mut state = App {
        display_handle: dh.clone(),
        compositor_state,
        xdg_shell_state: XdgShellState::new::<App>(&dh),
        shm_state,
        seat_state,
        data_device_state: DataDeviceState::new::<App>(&dh),
        primary_selection_state: PrimarySelectionState::new::<App>(&dh),
        output,
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
        resize_drag: None,
        window_drag: None,
        window_resize: None,
        pointer_captured: false,
        modifiers: ModifiersState::default(),
        cursor: CursorShape::Arrow,
        socket_name: String::new(),
        full_damage: true,
        damage: Vec::new(),
        icons: HashMap::new(),
        cursor_buffers: HashMap::new(),
        cursor_theme: Some(CursorTheme::discover()),
        client_cursor: None,
        cursor_hidden: false,
        index_socket: std::env::var("AETHER_INDEX_SOCKET").ok(),
        index_query: String::new(),
        agent_sessions: AgentSessions::bind(None),
    };

    state.agent_sessions =
        AgentSessions::bind(std::env::var_os("AETHER_AGENT_SOCKET").map(std::path::PathBuf::from));

    let listener = ListeningSocket::bind_auto("aether", 0..8)?;
    let socket_name = listener
        .socket_name()
        .and_then(|name| name.to_str())
        .unwrap_or("aether-0")
        .to_string();
    let mut clients = Vec::new();
    state.socket_name = socket_name.clone();

    state.refresh_launchers();
    println!(
        "aether-shell-smithay: loaded {} palette launchers",
        state.shell.launchers().len()
    );

    if state.index_socket.is_some() {
        println!(
            "aether-shell-smithay: using index service {:?}",
            state.index_socket
        );
    } else {
        let files = file_index();
        let file_count = files.len();
        state.shell.set_files(files);
        println!("aether-shell-smithay: indexed {file_count} files");
    }

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
    let mut applied_cursor = CursorShape::Arrow;
    backend.window().set_cursor_visible(false);
    let mut rendered_frames = 0u32;

    println!(
        "aether-shell-smithay: compositor on WAYLAND_DISPLAY={socket_name} (focus with pointer)"
    );

    loop {
        let status = winit.dispatch_new_events(|event| {
            state.mark_full_damage();
            match event {
            WinitEvent::Resized { size, scale_factor } => {
                state.window_size = size;
                state.scale = scale_factor;
                state.shell.set_viewport(logical_size(size, scale_factor));
                state.configure_all();
            }
            WinitEvent::Focus(_) => {}
            WinitEvent::Input(InputEvent::Keyboard { event }) => {
                let pressed = event.state() == KeyState::Pressed;
                keyboard.input::<(), _>(
                    &mut state,
                    event.key_code(),
                    event.state(),
                    0.into(),
                    0,
                    |state, modifiers, keysym| {
                        state.modifiers = *modifiers;
                        let sym = keysym.modified_sym();
                        if pressed && state.handle_shortcut(modifiers, sym) {
                            FilterResult::Intercept(())
                        } else if state.shell.overlay().is_some() {
                            // The overlay is modal: key presses edit the palette
                            // query instead of leaking to the focused client.
                            // Releases are consumed without repeating the key.
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
                state.modifiers = keyboard.modifier_state();
                state.cursor = cursor_for(&state);
            }
            WinitEvent::Input(InputEvent::PointerMotionAbsolute { event }) => {
                let logical = state.shell.viewport();
                let position = event.position_transformed(SSize::from((
                    logical.width as i32,
                    logical.height as i32,
                )));
                state.pointer_pos = position;
                state.cursor = cursor_for(&state);

                if let Some(resize) = state.window_resize {
                    let point =
                        AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
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
                    if state.shell.resize_floating(resize.id, rect) {
                        state.configure_all();
                    }
                } else if let Some(drag) = state.window_drag {
                    let x = (position.x - drag.offset_x).round() as i32;
                    let y = (position.y - drag.offset_y).round() as i32;
                    if state.shell.move_floating(drag.id, AetherPoint::new(x, y)) {
                        state.configure_all();
                    }
                } else if let Some(mut drag) = state.resize_drag {
                    // Drag the master/stack divider. Ratio is measured against
                    // the usable inner width (viewport minus outer gaps and gap).
                    let layout = state.shell.layout();
                    let inner = AetherRect::from_size(state.shell.viewport())
                        .inset(layout.outer_gap as i32);
                    let usable = (inner.width as i32 - layout.gap as i32).max(1) as f64;
                    let ratio = ((position.x - inner.x as f64) / usable) as f32;
                    if (ratio - drag.last_ratio).abs() > 0.002 {
                        drag.last_ratio = ratio;
                        state.resize_drag = Some(drag);
                        if state.shell.set_master_ratio(ratio) {
                            state.configure_all();
                        }
                    }
                } else {
                    let point =
                        AetherPoint::new(position.x.round() as i32, position.y.round() as i32);
                    if state.shell.overlay().is_some() {
                        if let Some(index) = state.shell.palette_row_at(point) {
                            state.shell.palette_select(index);
                        }
                    }
                    let chrome_owns_pointer = state.shell.overlay().is_some()
                        || state.shell.workspace_at(point).is_some();
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
                let over_divider = state.shell.master_gap_rect().is_some_and(|gap| {
                    AetherRect::new(gap.x - 5, gap.y, gap.width + 10, gap.height).contains(point)
                });

                if event.state() == ButtonState::Pressed {
                    state.pointer_captured = modal || over_picker;
                    if over_divider && event.button() == Some(MouseButton::Left) {
                        state.resize_drag = Some(ResizeDrag {
                            last_ratio: state.shell.master_ratio(),
                        });
                        state.pointer_captured = true;
                        println!("aether-shell-smithay: resize drag started");
                    } else if let Some(workspace) = state.shell.workspace_at(point) {
                        state.switch_workspace(workspace);
                    } else if modal {
                        // Rows are buttons; clicking elsewhere inside the panel
                        // is consumed and clicking outside dismisses it.
                        if let Some(index) = state.shell.palette_row_at(point) {
                            state.shell.palette_select(index);
                            state.activate_palette();
                        } else if state
                            .shell
                            .overlay_rect()
                            .is_some_and(|rect| !rect.contains(point))
                        {
                            state.toggle_overlay();
                        }
                    } else if state.modifiers.alt
                        && event.button() == Some(MouseButton::Left)
                    {
                        if let Some(id) = state.shell.window_at(point) {
                            state.shell.focus_window(id);
                            state.sync_focus();
                            if let Some(rect) = state
                                .shell
                                .begin_floating_move(id, AetherPoint::new(point.x, point.y))
                            {
                                state.window_drag = Some(WindowDrag {
                                    id,
                                    offset_x: position.x - rect.x as f64,
                                    offset_y: position.y - rect.y as f64,
                                });
                                state.pointer_captured = true;
                                state.configure_all();
                                println!(
                                    "aether-shell-smithay: window drag started id={id} rect={rect:?}"
                                );
                            }
                        }
                    } else if state.modifiers.alt
                        && event.button() == Some(MouseButton::Right)
                    {
                        if let Some(id) = state.shell.window_at(point) {
                            state.shell.focus_window(id);
                            state.sync_focus();
                            if let Some(rect) = state.shell.begin_floating_resize(id) {
                                let corner = resize_corner(rect, point);
                                state.window_resize = Some(WindowResize {
                                    id,
                                    corner,
                                    start_pointer: point,
                                    start_rect: rect,
                                });
                                state.pointer_captured = true;
                                state.configure_all();
                                println!(
                                    "aether-shell-smithay: window resize started id={id} rect={rect:?} corner={corner:?}"
                                );
                            }
                        }
                    } else if let Some(id) = state.shell.window_at(point) {
                        state.shell.focus_window(id);
                        state.sync_focus();
                    }
                }

                let forward = !state.pointer_captured;
                if event.state() == ButtonState::Released {
                    if let Some(resize) = state.window_resize.take() {
                        let rect = state
                            .shell
                            .windows()
                            .into_iter()
                            .find(|window| window.id == resize.id)
                            .map(|window| window.rect);
                        println!(
                            "aether-shell-smithay: window resized id={} rect={rect:?}",
                            resize.id
                        );
                    }
                    if let Some(drag) = state.window_drag.take() {
                        let position = state.pointer_pos;
                        let point = AetherPoint::new(
                            position.x.round() as i32,
                            position.y.round() as i32,
                        );
                        let viewport = state.shell.viewport();
                        let zone = if point.x <= 8 {
                            Some(SnapZone::LeftHalf)
                        } else if point.x >= viewport.width as i32 - 8 {
                            Some(SnapZone::RightHalf)
                        } else if point.y <= 8 {
                            Some(SnapZone::Maximize)
                        } else {
                            None
                        };
                        if let Some(zone) = zone {
                            if state.shell.snap_window(drag.id, zone) {
                                state.configure_all();
                                println!(
                                    "aether-shell-smithay: window snapped id={} zone={zone:?}",
                                    drag.id
                                );
                            }
                        }
                        let rect = state
                            .shell
                            .windows()
                            .into_iter()
                            .find(|window| window.id == drag.id)
                            .map(|window| window.rect);
                        println!(
                            "aether-shell-smithay: window moved id={} rect={rect:?}",
                            drag.id
                        );
                    }
                    if state.resize_drag.take().is_some() {
                        println!(
                            "aether-shell-smithay: master ratio -> {:.2}",
                            state.shell.master_ratio()
                        );
                    }
                    state.pointer_captured = false;
                }

                if forward {
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
                state.cursor = cursor_for(&state);
            }
            WinitEvent::CloseRequested => {
                state.close_requested = true;
            }
            _ => {}
            }
        });

        if matches!(status, PumpStatus::Exit(_)) || state.close_requested {
            return Ok(());
        }

        if state.cursor != applied_cursor {
            println!("aether-shell-smithay: cursor -> {:?}", state.cursor);
            applied_cursor = state.cursor;
        }

        // Apply focus changes once per loop to keep the borrow simple.
        if state.focus != applied_focus {
            let focus = state.focus.clone();
            keyboard.set_focus(&mut state, focus.clone(), 0.into());
            applied_focus = focus;
        }

        let mut frame_damage = None;
        let has_damage = state.full_damage || !state.damage.is_empty();
        if has_damage {
            let size = backend.window_size();
            let full = Rectangle::from_size(size);
            let damage: Vec<Rectangle<i32, Physical>> = if state.full_damage {
                vec![full]
            } else {
                state
                    .damage
                    .iter()
                    .filter_map(|rect| rect.intersection(full))
                    .collect()
            };
            let damage_label = if state.full_damage {
                "full".to_string()
            } else {
                format!("partial {:?}", state.damage)
            };

            {
                let (renderer, mut framebuffer) = backend.bind()?;
                let rects: HashMap<WindowId, AetherRect> = state
                    .shell
                    .windows()
                    .into_iter()
                    .filter(|window| window.workspace == state.shell.active_workspace())
                    .map(|window| (window.id, window.rect))
                    .collect();
                // Render in compositor z-order (topmost first) so raised, floating
                // and maximized windows are not covered by HashMap iteration order.
                let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = state
                    .shell
                    .window_stack()
                    .into_iter()
                    .filter_map(|id| {
                        let rect = rects.get(&id)?;
                        let toplevel = state.windows.get(&id)?;
                        Some((*rect, toplevel))
                    })
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
                let mut icons: Vec<(String, i32, i32)> = Vec::new();

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
                        let query = state.shell.palette_query();
                        if query.is_empty() {
                            labels.push((
                                "Search workspaces...".into(),
                                input.x + 10,
                                input.y + 10,
                                1,
                                [122, 173, 189, 255],
                            ));
                        } else {
                            labels.push((
                                format!("> {query}_"),
                                input.x + 10,
                                input.y + 10,
                                1,
                                [235, 252, 255, 255],
                            ));
                        }

                        let entries = state.shell.palette_entries();
                        let selected = state.shell.palette_selected();
                        if entries.is_empty() {
                            labels.push((
                                "No matching commands".into(),
                                rect.x + 26,
                                rect.y + 106,
                                1,
                                [128, 184, 199, 255],
                            ));
                        }
                        let start = state.shell.palette_visible_start();
                        for (row, entry) in entries
                            .iter()
                            .skip(start)
                            .take(PALETTE_VISIBLE_ROWS)
                            .enumerate()
                        {
                            let y = rect.y + 96 + row as i32 * 48;
                            if y + 32 < rect.y + rect.height as i32 {
                                let row_rect = AetherRect::new(
                                    rect.x + 16,
                                    y,
                                    rect.width.saturating_sub(32),
                                    32,
                                );
                                let active = start + row == selected;
                                chrome.push((
                                    row_rect,
                                    if active {
                                        [0.09, 0.42, 0.5, 1.0]
                                    } else {
                                        [0.05, 0.31, 0.37, 1.0]
                                    },
                                ));
                                if active {
                                    chrome.push((
                                        AetherRect::new(row_rect.x, row_rect.y, 3, row_rect.height),
                                        [0.22, 0.84, 1.0, 1.0],
                                    ));
                                }
                                let icon = match entry.action {
                                    PaletteAction::LaunchApp(index) => state
                                        .shell
                                        .launcher(index)
                                        .and_then(|launcher| launcher.icon),
                                    _ => None,
                                };
                                let title_x = if let Some(icon) = icon {
                                    icons.push((icon, row_rect.x + 10, row_rect.y + 8));
                                    row_rect.x + 34
                                } else {
                                    row_rect.x + 12
                                };
                                labels.push((
                                    entry.title.clone(),
                                    title_x,
                                    row_rect.y + 12,
                                    1,
                                    if active {
                                        [235, 252, 255, 255]
                                    } else {
                                        [217, 247, 255, 255]
                                    },
                                ));
                                labels.push((
                                    entry.detail.clone(),
                                    row_rect.x + row_rect.width as i32
                                        - text_width(&entry.detail, 1) as i32
                                        - 10,
                                    row_rect.y + 12,
                                    1,
                                    [128, 184, 199, 255],
                                ));
                            }
                        }

                        let footer = "Esc close   Enter run   Up/Down select";
                        labels.push((
                            footer.into(),
                            rect.x + 16,
                            rect.y + rect.height as i32 - 22,
                            1,
                            [128, 184, 199, 255],
                        ));
                    }
                }

                if state.shell.fullscreen_window().is_none() {
                    let names = state.shell.workspace_names();
                    for workspace in 0..state.shell.workspace_count() {
                        let Some(chip) = state.shell.workspace_indicator_rect(workspace) else {
                            continue;
                        };
                        let active = workspace == state.shell.active_workspace();
                        chrome.push((
                            AetherRect::new(
                                chip.x,
                                chip.y,
                                chip.width,
                                WORKSPACE_INDICATOR_BAR_HEIGHT,
                            ),
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
                }

                let mut icon_elements: Vec<MemoryRenderBufferRenderElement<GlesRenderer>> =
                    Vec::new();
                for (path, x, y) in &icons {
                    if !state.icons.contains_key(path) {
                        if let Some((width, height, pixels)) = load_icon(path) {
                            let buffer = MemoryRenderBuffer::from_slice(
                                &pixels,
                                Fourcc::Abgr8888,
                                (width as i32, height as i32),
                                1,
                                Transform::Normal,
                                None,
                            );
                            state.icons.insert(path.to_string(), buffer);
                        }
                    }
                    if let Some(buffer) = state.icons.get(path) {
                        if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                            renderer,
                            (*x as f64, *y as f64),
                            buffer,
                            None,
                            None,
                            Some(SSize::from((16, 16))),
                            Kind::Unspecified,
                        ) {
                            icon_elements.push(element);
                        }
                    }
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

                let cursor_shape = state.cursor;
                let position = state.pointer_pos;
                let theme_bitmap = state
                    .cursor_theme
                    .as_ref()
                    .and_then(|theme| theme.load(cursor_shape, 24));
                let entry = state.cursor_buffers.entry(cursor_shape).or_insert_with(|| {
                    let bitmap = theme_bitmap.unwrap_or_else(|| render_cursor(cursor_shape));
                    let buffer = MemoryRenderBuffer::from_slice(
                        &bitmap.pixels,
                        Fourcc::Abgr8888,
                        (bitmap.width as i32, bitmap.height as i32),
                        1,
                        Transform::Normal,
                        None,
                    );
                    CursorCacheEntry {
                        buffer,
                        hotspot_x: bitmap.hotspot_x,
                        hotspot_y: bitmap.hotspot_y,
                    }
                });
                let cursor_element = (!state.cursor_hidden)
                    .then(|| {
                        let x = position.x.round() as i32 - entry.hotspot_x;
                        let y = position.y.round() as i32 - entry.hotspot_y;
                        MemoryRenderBufferRenderElement::from_buffer(
                            renderer,
                            (x as f64, y as f64),
                            &entry.buffer,
                            None,
                            None,
                            None,
                            Kind::Unspecified,
                        )
                        .ok()
                    })
                    .flatten();

                let mut frame = renderer.render(&mut framebuffer, size, Transform::Flipped180)?;
                frame.clear(Color32F::new(0.01, 0.06, 0.08, 1.0), &damage)?;
                // Client surfaces are painted over the shell background.
                if !elements.is_empty() {
                    draw_render_elements(&mut frame, 1.0, &elements, &damage)?;
                }
                if !chrome.is_empty() {
                    // `draw_render_elements` treats the slice as topmost-first and
                    // reverses it internally; our chrome is built bottom-up.
                    let chrome_top_down: Vec<(AetherRect, [f32; 4])> =
                        chrome.iter().rev().copied().collect();
                    let buffers: Vec<SolidColorBuffer> = chrome_top_down
                        .iter()
                        .map(|(rect, color)| {
                            SolidColorBuffer::new((rect.width as i32, rect.height as i32), *color)
                        })
                        .collect();
                    let elements: Vec<SolidColorRenderElement> = buffers
                        .iter()
                        .zip(chrome_top_down.iter())
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
                        &mut frame, 1.0, &elements, &damage,
                    )?;
                }
                if !icon_elements.is_empty() {
                    draw_render_elements::<
                        GlesRenderer,
                        f64,
                        MemoryRenderBufferRenderElement<GlesRenderer>,
                    >(&mut frame, 1.0, &icon_elements, &damage)?;
                }
                if !text_elements.is_empty() {
                    draw_render_elements::<
                        GlesRenderer,
                        f64,
                        MemoryRenderBufferRenderElement<GlesRenderer>,
                    >(&mut frame, 1.0, &text_elements, &damage)?;
                }
                if let Some(cursor) = cursor_element {
                    draw_render_elements::<
                        GlesRenderer,
                        f64,
                        MemoryRenderBufferRenderElement<GlesRenderer>,
                    >(&mut frame, 1.0, &[cursor], &damage)?;
                }
                let _ = frame.finish()?;
            }

            for surface in state.xdg_shell_state.toplevel_surfaces() {
                send_frames_surface_tree(
                    surface.wl_surface(),
                    start_time.elapsed().as_millis() as u32,
                );
            }
            state.full_damage = false;
            state.damage.clear();
            frame_damage = Some(damage);
            rendered_frames += 1;
            if std::env::var_os("AETHER_FRAME_LOG").is_some() {
                println!("aether-shell-smithay: frame {rendered_frames} damage={damage_label}");
            }
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
        if state.agent_sessions.poll() {
            state.refresh_launchers();
            println!(
                "aether-shell-smithay: agent sessions -> {} launchers",
                state.shell.launchers().len()
            );
        }
        if let Some(damage) = frame_damage {
            backend.submit(Some(&damage))?;
        }

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
        self.output.enter(surface.wl_surface());
        let title = toplevel_title(&surface).unwrap_or_else(|| "Wayland client".to_string());
        let mut spec = WindowSpec::new(title, "wayland.client");
        spec.preferred_size = AetherSize::new(800, 600);
        let id = self.shell.add_window(spec);
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
        self.output.leave(surface.wl_surface());
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
        if window.workspace != self.shell.active_workspace() {
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
    fn new_popup(&mut self, _surface: InputMethodPopupSurface) {}
    fn dismiss_popup(&mut self, _surface: InputMethodPopupSurface) {}
    fn popup_repositioned(&mut self, _surface: InputMethodPopupSurface) {}
    fn parent_geometry(&self, _parent: &WlSurface) -> Rectangle<i32, Logical> {
        Rectangle::default()
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
            }
            CursorImageStatus::Named(icon) => {
                self.cursor_hidden = false;
                self.client_cursor = Some(pointer_shape(icon));
            }
            CursorImageStatus::Surface(_) => {
                self.cursor_hidden = false;
                self.client_cursor = None;
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

fn load_icon(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let file = std::fs::File::open(path).ok()?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let data = &buffer[..info.buffer_size()];
    let pixels = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        _ => return None,
    };
    Some((info.width, info.height, pixels))
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

struct App {
    display_handle: DisplayHandle,
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    primary_selection_state: PrimarySelectionState,
    output: Output,
    seat: Seat<Self>,
    shell: Shell,
    windows: HashMap<WindowId, ToplevelSurface>,
    configured: HashMap<WindowId, (i32, i32, bool)>,
    focus: Option<WlSurface>,
    close_requested: bool,
    window_size: SSize<i32, Physical>,
    scale: f64,
    pointer_pos: smithay::utils::Point<f64, smithay::utils::Logical>,
    resize_drag: Option<ResizeDrag>,
    window_drag: Option<WindowDrag>,
    window_resize: Option<WindowResize>,
    pointer_captured: bool,
    modifiers: ModifiersState,
    cursor: CursorShape,
    socket_name: String,
    full_damage: bool,
    damage: Vec<Rectangle<i32, Physical>>,
    icons: HashMap<String, MemoryRenderBuffer>,
    cursor_buffers: HashMap<CursorShape, CursorCacheEntry>,
    cursor_theme: Option<CursorTheme>,
    client_cursor: Option<CursorShape>,
    cursor_hidden: bool,
    index_socket: Option<String>,
    index_query: String,
    agent_sessions: AgentSessions,
}

struct CursorCacheEntry {
    buffer: MemoryRenderBuffer,
    hotspot_x: i32,
    hotspot_y: i32,
}

impl App {
    fn mark_full_damage(&mut self) {
        self.full_damage = true;
        self.damage.clear();
    }

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

    fn toggle_overlay(&mut self) {
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
                let workspace = self
                    .shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == id)
                    .map(|window| window.workspace);
                if let Some(workspace) = workspace {
                    if workspace != self.shell.active_workspace() {
                        self.switch_workspace(workspace);
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
smithay::delegate_primary_selection!(App);
smithay::delegate_output!(App);
smithay::delegate_text_input_manager!(App);
smithay::delegate_input_method_manager!(App);
