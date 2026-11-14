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
use aether_shell_wayland::App;
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
            primary: true,
        },
        output,
    );
    state.configure_all();

    if args.iter().any(|arg| arg == "--overlay") || std::env::var("AETHER_OVERLAY").is_ok() {
        state.toggle_overlay();
        println!(
            "aether-shell-smithay: overlay enabled: {:?}",
            state.shell().overlay_rect()
        );
    }

    let mut frames = 0u32;
    let mut applied_cursor = CursorShape::Arrow;
    println!("aether-shell-smithay: compositor on winit window");
    state.spawn_test_clients();

    loop {
        let status = winit.dispatch_new_events(|event| {
            state.mark_full_damage();
            match event {
                WinitEvent::Resized { size, scale_factor } => {
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
            }
        });

        if matches!(status, PumpStatus::Exit(_)) || state.close_requested() {
            return Ok(());
        }

        state.flush_focus(&keyboard);
        if state.cursor() != applied_cursor {
            applied_cursor = state.cursor();
            println!("aether-shell-smithay: cursor -> {applied_cursor:?}");
        }

        let (full, damage) = state.take_damage();
        if full || !damage.is_empty() {
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
                }
                state.send_frame_callbacks(start_time.elapsed().as_millis() as u32);
                backend.submit(Some(&damage))?;
                if std::env::var_os("AETHER_FRAME_LOG").is_some() {
                    println!("aether-shell-smithay: frame damage={damage_label}");
                }
            }
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
