//! Linux-only Smithay winit adapter for the Aether shell.
//!
//! This is the first real backend: it opens a native window, maps the shell's
//! [`aether_shell::FramePlan`] to solid-colour render elements and presents at
//! vsync. It is deliberately a standalone crate and is **not** part of the
//! default workspace, so macOS/CI never try to build Smithay.
//!
//! What it does not do yet: accept Wayland clients, render client buffers or
//! route input. Those need the xdg-shell/compositor handlers and are the next
//! step. This slice proves the shell/backend contract on a real GPU/EGL window.

use std::error::Error;
use std::time::Instant;

use aether_geometry::{Rect, Size};
use aether_layout::LayoutConfig;
use aether_scene::Layer;
use aether_shell::{FrameItem, FramePlan, Shell, WindowSpec};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::reexports::calloop::{EventLoop, LoopSignal};
use smithay::utils::{Physical, Point, Rectangle, Size as SSize, Transform};

pub fn run() -> Result<(), Box<dyn Error>> {
    let frames_left = parse_frames();

    let (backend, winit_source) = winit::init::<GlesRenderer>()?;
    let physical = backend.window_size();
    let scale = backend.scale_factor();
    let viewport = logical_size(physical, scale);

    let mut shell = Shell::new(
        viewport,
        LayoutConfig {
            master_ratio: 0.55,
            gap: 10,
            outer_gap: 16,
        },
        120,
    );
    add_demo_windows(&mut shell);
    shell.set_overlay(Some(Rect::new(620, 180, 680, 520)));
    let plan = shell.frame(0).ok_or("initial frame was not scheduled")?;

    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();
    let mut state = State {
        backend,
        shell,
        plan,
        damage_tracker: OutputDamageTracker::new(physical, scale, Transform::Normal),
        start: Instant::now(),
        frames_left,
        loop_signal,
        size: physical,
    };

    event_loop
        .handle()
        .insert_source(winit_source, |event, _, state| {
            if let Err(error) = state.handle(event) {
                eprintln!("aether-shell-smithay: {error}");
                state.loop_signal.stop();
            }
        })?;

    println!(
        "aether-shell-smithay: window {}x{} @ {scale}x, {} windows, {} frame items",
        physical.w,
        physical.h,
        state.shell.window_count(),
        state.plan.items.len()
    );

    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}

struct State {
    backend: WinitGraphicsBackend<GlesRenderer>,
    shell: Shell,
    plan: FramePlan,
    damage_tracker: OutputDamageTracker,
    start: Instant,
    frames_left: Option<u32>,
    loop_signal: LoopSignal,
    size: SSize<i32, Physical>,
}

impl State {
    fn handle(&mut self, event: WinitEvent) -> Result<(), Box<dyn Error>> {
        match event {
            WinitEvent::Resized { size, scale_factor } => {
                self.size = size;
                self.shell.set_viewport(logical_size(size, scale_factor));
                self.damage_tracker =
                    OutputDamageTracker::new(size, scale_factor, Transform::Normal);
                if let Some(plan) = self.shell.frame(elapsed_ns(self.start)) {
                    self.plan = plan;
                }
                self.backend.window().request_redraw();
            }
            WinitEvent::Redraw => self.draw()?,
            WinitEvent::CloseRequested => self.loop_signal.stop(),
            WinitEvent::Focus(_) | WinitEvent::Input(_) => {}
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(plan) = self.shell.frame(elapsed_ns(self.start)) {
            self.plan = plan;
        }

        // Solid-colour stand-ins for now; the next slice replaces these with
        // compositor surface render elements. The order follows draw order.
        let buffers: Vec<SolidColorBuffer> = self
            .plan
            .items
            .iter()
            .map(|item| {
                SolidColorBuffer::new(
                    (item.rect.width as i32, item.rect.height as i32),
                    color(item),
                )
            })
            .collect();

        let elements: Vec<SolidColorRenderElement> = self
            .plan
            .items
            .iter()
            .zip(buffers.iter())
            .map(|(item, buffer)| {
                SolidColorRenderElement::from_buffer(
                    buffer,
                    Point::from((item.rect.x, item.rect.y)),
                    1.0f64,
                    1.0,
                    Kind::Unspecified,
                )
            })
            // RenderElement lists are consumed front-to-back by the damage
            // tracker; our plan is bottom-to-top, so reverse it.
            .rev()
            .collect();

        {
            let (renderer, mut framebuffer) = self.backend.bind()?;
            self.damage_tracker.render_output(
                renderer,
                &mut framebuffer,
                0,
                &elements,
                [0.01, 0.06, 0.08, 1.0],
            )?;
        }

        let damage = Rectangle::from_size(self.size);
        self.backend.submit(Some(&[damage]))?;

        if let Some(remaining) = self.frames_left {
            if remaining <= 1 {
                self.loop_signal.stop();
            } else {
                self.frames_left = Some(remaining - 1);
            }
        }

        self.backend.window().request_redraw();
        Ok(())
    }
}

fn add_demo_windows(shell: &mut Shell) {
    let windows = [
        ("compositor.rs", "aether.shell", Size::new(1000, 800)),
        ("indexer.rs", "aether.index", Size::new(760, 600)),
        ("Chat · DeepSeek", "aether.chat", Size::new(600, 700)),
        ("Files", "aether.files", Size::new(700, 520)),
    ];
    for (title, app_id, size) in windows {
        let mut spec = WindowSpec::new(title, app_id);
        spec.preferred_size = size;
        shell.add_window(spec);
    }
}

fn color(item: &FrameItem) -> [f32; 4] {
    if item.layer == Layer::OVERLAY {
        [0.05, 0.24, 0.29, 1.0]
    } else if item.focused {
        [0.06, 0.22, 0.27, 1.0]
    } else {
        [0.04, 0.15, 0.2, 1.0]
    }
}

fn logical_size(physical: SSize<i32, Physical>, scale: f64) -> Size {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Size::new(
        ((physical.w as f64) / scale).round().max(1.0) as u32,
        ((physical.h as f64) / scale).round().max(1.0) as u32,
    )
}

fn elapsed_ns(start: Instant) -> u64 {
    start.elapsed().as_nanos() as u64
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
