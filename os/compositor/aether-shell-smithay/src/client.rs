//! Client-capable Wayland compositor on Smithay's winit backend.
//!
//! The protocol state, input policy and render composition live in
//! `aether-shell-wayland`. This file only owns the winit window: it converts
//! winit events into shell input and presents composed frames.

use std::error::Error;
use std::time::Instant;

use aether_cursor::CursorShape;
use aether_geometry::{Rect as AetherRect, Size as AetherSize};
use aether_shell::OutputInfo;
use aether_shell_wayland::{append_stat, damage_area, App};
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, Event, InputEvent, KeyboardKeyEvent, PointerAxisEvent,
    PointerButtonEvent,
};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::{Frame, Renderer};
use smithay::backend::winit::{self, WinitEvent};
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::wayland_server::Display;
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::reexports::winit::window::WindowAttributes;
use smithay::utils::{Physical, Rectangle, Size as SSize, Transform};

pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let max_frames = parse_frames();
    let mut display: Display<App> = Display::new()?;
    let (mut state, keyboard) = App::new(&display.handle())?;

    let (mut backend, mut winit) = winit::init_from_attributes::<GlesRenderer>(
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(1280.0, 800.0))
            .with_title("Aether OS"),
    )?;
    backend.window().set_cursor_visible(false);

    let start_time = Instant::now();
    let physical = backend.window_size();
    let scale = backend.scale_factor();
    let logical = logical_size(physical, scale);
    state.shell_mut().set_viewport(logical);

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
        size: (logical.width as i32, logical.height as i32).into(),
        refresh: 60_000,
    };
    output.set_preferred(mode);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );
    state.add_output(
        OutputInfo {
            id: 1,
            name: "Aether-1".to_string(),
            rect: AetherRect::from_size(logical),
            scale,
            refresh_hz: backend
                .window()
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz())
                .map(|milli| (milli / 1000).max(1))
                .unwrap_or(60),
            physical_size_mm: backend
                .window()
                .current_monitor()
                .map(|monitor| monitor.size())
                .map(|size| (size.width, size.height))
                .unwrap_or((0, 0)),
            primary: true,
        },
        output,
    );
    state.configure_all();
    if let Some(output) = state.shell().primary_output() {
        if output.refresh_hz < 120 {
            println!(
                "aether-shell-smithay: host output refreshes at {} Hz, below the 120 Hz P2 gate",
                output.refresh_hz
            );
        }
    }

    if args.iter().any(|arg| arg == "--overlay") || std::env::var("AETHER_OVERLAY").is_ok() {
        state.toggle_overlay();
        println!(
            "aether-shell-smithay: overlay enabled: {:?}",
            state.shell().overlay_rect()
        );
    }

    let mut frames = 0u32;
    let mut applied_cursor = CursorShape::Arrow;
    let stats_path = std::env::var_os("AETHER_FRAME_STATS").map(std::path::PathBuf::from);
    let mut last_present: Option<Instant> = None;
    println!("aether-shell-smithay: compositor on winit window");
    state.spawn_test_clients();

    loop {
        let status = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, scale_factor } => {
                state.mark_full_damage();
                let logical = logical_size(size, scale_factor);
                state.shell_mut().set_viewport(logical);
                state.configure_all();
            }
            WinitEvent::Focus(_) => {}
            WinitEvent::Input(InputEvent::Keyboard { event }) => {
                state.keyboard_key(
                    &keyboard,
                    event.key_code(),
                    event.state(),
                    event.time_msec(),
                );
            }
            WinitEvent::Input(InputEvent::PointerMotionAbsolute { event }) => {
                let logical = state.shell().viewport();
                let position = event.position_transformed(SSize::from((
                    logical.width as i32,
                    logical.height as i32,
                )));
                state.pointer_motion(position, event.time_msec());
            }
            WinitEvent::Input(InputEvent::PointerAxis { event }) => {
                let source = event.source();
                let horizontal = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.0
                });
                let vertical = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.0
                });
                state.pointer_axis(horizontal, vertical, source, event.time_msec());
            }
            WinitEvent::Input(InputEvent::PointerButton { event }) => {
                state.pointer_button(event.button_code(), event.state(), event.time_msec());
            }
            WinitEvent::CloseRequested => {
                state.request_close();
            }
            _ => {}
        });

        if matches!(status, PumpStatus::Exit(_)) || state.close_requested() {
            return Ok(());
        }

        state.flush_focus(&keyboard);
        if state.cursor() != applied_cursor {
            applied_cursor = state.cursor();
            println!("aether-shell-smithay: cursor -> {applied_cursor:?}");
        }

        if let Some((full, damage)) = state.output_damage(None) {
            let size = backend.window_size();
            let full_rect = Rectangle::from_size(size);
            let damage_label = if full {
                "full".to_string()
            } else {
                format!("partial {damage:?}")
            };
            let damage: Vec<Rectangle<i32, Physical>> = if full {
                vec![full_rect]
            } else {
                damage
                    .iter()
                    .filter_map(|rect| rect.intersection(full_rect))
                    .collect()
            };

            if !damage.is_empty() {
                {
                    let (renderer, mut framebuffer) = backend.bind()?;
                    let composed = state.compose(renderer, None);
                    let mut frame =
                        renderer.render(&mut framebuffer, size, Transform::Flipped180)?;
                    frame.clear(composed.clear, &damage)?;
                    draw_render_elements(&mut frame, 1.0, &composed.elements, &damage)?;
                    let _ = frame.finish()?;
                    let now = Instant::now();
                    let interval = last_present
                        .map(|last| now.duration_since(last).as_nanos() as u64)
                        .unwrap_or(0);
                    last_present = Some(now);
                    if interval > 0 && interval < 200_000_000 {
                        state.set_frame_time_ms(interval as f32 / 1_000_000.0);
                    }
                    if let Some(path) = stats_path.as_deref() {
                        append_stat(
                            path,
                            &format!(
                                "{{\"kind\":\"present\",\"output\":1,\"width\":{},\"height\":{},\"refresh_hz\":60,\"interval_ns\":{},\"missed\":0,\"queue_depth\":0,\"max_queue_depth\":0,\"input_to_damage_ns\":{},\"damage_area\":{},\"cleared_area\":{}}}",
                                size.w,
                                size.h,
                                interval,
                                state.last_input_to_damage_ns(),
                                damage_area(&composed.damage),
                                damage_area(&damage),
                            ),
                        );
                    }
                }
                state.send_frame_callbacks(start_time.elapsed().as_millis() as u32);
                backend.submit(Some(&damage))?;
                if std::env::var_os("AETHER_FRAME_LOG").is_some() {
                    println!("aether-shell-smithay: frame damage={damage_label}");
                }
            }
            let _ = state.take_damage();
        }

        state.poll_clients(&mut display)?;
        state.spawn_test_clients();
        state.configure_all();

        frames += 1;
        if max_frames.is_some_and(|limit| frames >= limit) {
            return Ok(());
        }
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

fn logical_size(physical: SSize<i32, Physical>, scale: f64) -> AetherSize {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    AetherSize::new(
        ((physical.w as f64) / scale).round().max(1.0) as u32,
        ((physical.h as f64) / scale).round().max(1.0) as u32,
    )
}
