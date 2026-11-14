//! Native single-process KMS/DRM backend for Aether.
//!
//! One process owns the session, DRM devices, libinput seat and the Wayland
//! display. It drives `DrmCompositor` page-flips per connected output, scans
//! connectors for hotplug, and shares all protocol/composition logic with the
//! winit adapter through `aether-shell-wayland`.
//!
//! Boot it from a VT with seat access (logind or seatd):
//!
//!   aether-shell-drm [--frames N]
//!
//! `AETHER_DRM_DEVICE` forces a specific `/dev/dri/cardN` path.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

use aether_geometry::Rect as AetherRect;
use aether_shell::{OutputId, OutputInfo};
use aether_shell_wayland::{AetherRenderElement, App};
use calloop::EventLoop;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::allocator::{Format, Fourcc};
use smithay::backend::drm::compositor::{FrameError, FrameFlags, RenderFrameError};
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::drm::output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements};
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, DrmNode};
use smithay::backend::egl::{EGLContext, EGLDisplay};
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, Event, InputEvent, KeyboardKeyEvent, PointerAxisEvent,
    PointerButtonEvent, PointerMotionEvent,
};
use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SessionEvent, Session};
use smithay::backend::udev::{primary_gpu, UdevBackend, UdevEvent};
use smithay::input::keyboard::KeyboardHandle;
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay::reexports::drm::control::{connector, crtc, Device as ControlDevice, ModeTypeFlags};
use smithay::reexports::input::Libinput;
use smithay::reexports::rustix::fs::OFlags;
use smithay::reexports::wayland_server::Display;
use smithay::utils::{DeviceFd, Size as SSize, Transform};

type DrmAllocator = GbmAllocator<DrmDeviceFd>;
type DrmExporter = GbmFramebufferExporter<DrmDeviceFd>;
type AetherDrmOutput = DrmOutput<DrmAllocator, DrmExporter, (), DrmDeviceFd>;
type AetherDrmManager = DrmOutputManager<DrmAllocator, DrmExporter, (), DrmDeviceFd>;

/// Preferred scan-out formats, best quality first.
const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];

struct DeviceBackend {
    renderer: GlesRenderer,
    manager: AetherDrmManager,
    surfaces: HashMap<crtc::Handle, OutputSurface>,
    notifier: RegistrationToken,
}

struct OutputSurface {
    id: OutputId,
    connector: connector::Handle,
    drm_output: AetherDrmOutput,
    dirty: bool,
}

struct DrmState {
    app: App,
    display: Display<App>,
    session: LibSeatSession,
    libinput: Libinput,
    keyboard: KeyboardHandle<App>,
    handle: LoopHandle<'static, DrmState>,
    devices: HashMap<DrmNode, DeviceBackend>,
    next_output_id: OutputId,
    frames: u32,
    max_frames: Option<u32>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let max_frames = parse_frames();
    let mut event_loop = EventLoop::try_new()?;
    let display: Display<App> = Display::new()?;
    let (app, keyboard) = App::new(&display.handle())?;

    let session = match LibSeatSession::new() {
        Ok((session, notifier)) => {
            let handle = event_loop.handle();
            handle.insert_source(
                notifier,
                |event, &mut (), state: &mut DrmState| match event {
                    SessionEvent::PauseSession => {
                        println!("aether-shell-drm: session paused");
                        state.libinput.suspend();
                        for device in state.devices.values_mut() {
                            device.manager.pause();
                        }
                    }
                    SessionEvent::ActivateSession => {
                        println!("aether-shell-drm: session activated");
                        if let Err(error) = state.libinput.resume() {
                            eprintln!("aether-shell-drm: libinput resume failed: {error:?}");
                        }
                        for device in state.devices.values_mut() {
                            if let Err(error) = device.manager.activate(false) {
                                eprintln!("aether-shell-drm: DRM activate failed: {error:?}");
                            }
                            for surface in device.surfaces.values_mut() {
                                surface.dirty = true;
                            }
                        }
                    }
                },
            )?;
            session
        }
        Err(error) => {
            eprintln!("aether-shell-drm: no seat session ({error}); use the cage fallback on a VT");
            return Err(error.into());
        }
    };

    let seat_name = session.seat();
    println!("aether-shell-drm: seat {seat_name}");

    println!("aether-shell-drm: creating libinput context");
    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());
    libinput_context
        .udev_assign_seat(&seat_name)
        .map_err(|error| format!("libinput seat assignment failed: {error:?}"))?;
    println!("aether-shell-drm: libinput ready");
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    let handle = event_loop.handle();
    let mut state = DrmState {
        app,
        display,
        session,
        libinput: libinput_context,
        keyboard: keyboard.clone(),
        handle,
        devices: HashMap::new(),
        next_output_id: 1,
        frames: 0,
        max_frames,
    };

    let udev_backend = UdevBackend::new(&seat_name)?;
    let initial: Vec<(u64, std::path::PathBuf)> = udev_backend
        .device_list()
        .map(|(id, path)| (id, path.to_path_buf()))
        .collect();

    event_loop.handle().insert_source(
        udev_backend,
        |event, _, state: &mut DrmState| match event {
            UdevEvent::Added { device_id, path } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.add_device(node, &path);
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.scan_outputs(node);
                }
            }
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.remove_device(node);
                }
            }
        },
    )?;

    event_loop
        .handle()
        .insert_source(libinput_backend, |event, _, state: &mut DrmState| {
            state.input(event)
        })?;

    // Initialize the primary GPU first so display-only nodes can fall back.
    let primary = primary_gpu(&seat_name)
        .ok()
        .flatten()
        .and_then(|path| DrmNode::from_path(path).ok());
    let mut ordered: Vec<(u64, std::path::PathBuf)> = initial;
    if let Some(primary) = primary {
        ordered.sort_by_key(|(_, path)| {
            DrmNode::from_path(path)
                .map(|node| node != primary)
                .unwrap_or(true)
        });
    }
    // logind can take a moment to activate the tty session and grant DRM
    // access, especially on cold boot. Retry briefly before giving up.
    for attempt in 0..10 {
        for (_, path) in &ordered {
            if let Ok(node) = DrmNode::from_path(path) {
                state.add_device(node, path);
            }
        }
        if !state.devices.is_empty() {
            break;
        }
        if attempt < 9 {
            println!("aether-shell-drm: no usable DRM device yet; retrying");
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    if state.devices.is_empty() {
        return Err("no usable DRM device (no seat access or no GPU)".into());
    }

    state.app.flush_focus(&state.keyboard);
    state.mark_all_dirty();
    state.render_all();
    state.app.spawn_test_clients();

    let start = std::time::Instant::now();
    loop {
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut state)?;
        state.app.poll_clients(&mut state.display)?;
        state.app.flush_focus(&state.keyboard);
        if state.app.needs_frame() {
            state.mark_all_dirty();
        }
        let presented = state.render_all();
        state.app.configure_all();
        if presented {
            state
                .app
                .send_frame_callbacks(start.elapsed().as_millis() as u32);
        }

        state.frames += 1;
        if state.max_frames.is_some_and(|limit| state.frames >= limit) {
            return Ok(());
        }
    }
}

impl DrmState {
    fn add_device(&mut self, node: DrmNode, path: &Path) {
        if self.devices.contains_key(&node) {
            return;
        }
        match self.open_device(node, path) {
            Ok(device) => {
                println!("aether-shell-drm: initialized {}", path.display());
                self.devices.insert(node, device);
                self.scan_outputs(node);
            }
            Err(error) => eprintln!("aether-shell-drm: skipping {} ({error})", path.display()),
        }
    }

    fn open_device(&mut self, node: DrmNode, path: &Path) -> Result<DeviceBackend, Box<dyn Error>> {
        println!("aether-shell-drm: opening {}", path.display());
        let file = match self.session.open(path, OFlags::RDWR | OFlags::CLOEXEC) {
            Ok(file) => file,
            Err(error) => {
                eprintln!(
                    "aether-shell-drm: libseat open {} failed ({error}); trying direct open",
                    path.display()
                );
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)?
                    .into()
            }
        };
        let fd = DrmDeviceFd::new(DeviceFd::from(file));
        let (drm, notifier) = DrmDevice::new(fd.clone(), true)?;
        println!("aether-shell-drm: DRM device ready");
        if let Ok(resources) = drm.resource_handles() {
            for handle in resources.connectors() {
                if let Ok(info) = drm.get_connector(*handle, false) {
                    println!(
                        "aether-shell-drm: probe connector {} state {:?} modes {}",
                        info,
                        info.state(),
                        info.modes().len()
                    );
                }
            }
        }

        let gbm = GbmDevice::new(fd.clone())?;
        println!("aether-shell-drm: GBM device ready");
        let egl_display = unsafe { EGLDisplay::new(gbm.clone())? };
        println!("aether-shell-drm: EGL display ready");
        let egl_context = EGLContext::new(&egl_display)?;
        println!("aether-shell-drm: EGL context ready");
        let renderer = unsafe { GlesRenderer::new(egl_context)? };
        println!("aether-shell-drm: GLES renderer ready");

        let render_formats: Vec<Format> = renderer
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .copied()
            .collect();

        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), None);
        let manager: AetherDrmManager = DrmOutputManager::new(
            drm,
            allocator,
            exporter,
            Some(gbm),
            SUPPORTED_FORMATS.iter().copied(),
            render_formats,
        );

        let device_node = node;
        let notifier = self.handle.insert_source(
            notifier,
            move |event, _metadata, state: &mut DrmState| match event {
                DrmEvent::VBlank(crtc) => state.frame_submitted(device_node, crtc),
                DrmEvent::Error(error) => {
                    eprintln!("aether-shell-drm: DRM event error: {error}")
                }
            },
        )?;

        Ok(DeviceBackend {
            renderer,
            manager,
            surfaces: HashMap::new(),
            notifier,
        })
    }

    fn remove_device(&mut self, node: DrmNode) {
        let ids: Vec<(connector::Handle, OutputId)> = self
            .devices
            .get(&node)
            .map(|device| {
                device
                    .surfaces
                    .values()
                    .map(|surface| (surface.connector, surface.id))
                    .collect()
            })
            .unwrap_or_default();
        for (connector, id) in ids {
            self.remove_output(node, connector, id);
        }
        if let Some(mut device) = self.devices.remove(&node) {
            self.handle.remove(device.notifier);
            device.surfaces.clear();
            println!("aether-shell-drm: removed DRM device");
        }
    }

    fn scan_outputs(&mut self, node: DrmNode) {
        let Some(device) = self.devices.get(&node) else {
            return;
        };
        let used: Vec<crtc::Handle> = device.surfaces.keys().copied().collect();
        let connected = match enumerate_connectors(device, &used) {
            Ok(connected) => connected,
            Err(error) => {
                eprintln!("aether-shell-drm: connector scan failed: {error}");
                return;
            }
        };
        println!(
            "aether-shell-drm: connector scan -> {} usable output(s)",
            connected.len()
        );

        let present: Vec<connector::Handle> = connected
            .iter()
            .map(|candidate| candidate.connector.handle())
            .collect();

        for candidate in connected {
            let already = self.devices.get(&node).is_some_and(|device| {
                device
                    .surfaces
                    .values()
                    .any(|surface| surface.connector == candidate.connector.handle())
            });
            if !already {
                self.add_output(node, candidate);
            }
        }

        let stale: Vec<(connector::Handle, OutputId)> = self
            .devices
            .get(&node)
            .map(|device| {
                device
                    .surfaces
                    .values()
                    .filter(|surface| !present.contains(&surface.connector))
                    .map(|surface| (surface.connector, surface.id))
                    .collect()
            })
            .unwrap_or_default();
        for (connector, id) in stale {
            self.remove_output(node, connector, id);
        }
    }

    fn add_output(&mut self, node: DrmNode, candidate: ConnectorCandidate) {
        let next_id = self.next_output_id;
        let existing_outputs: usize = self
            .devices
            .values()
            .map(|device| device.surfaces.len())
            .sum();
        let max_right = self
            .app
            .shell()
            .outputs()
            .iter()
            .map(|output| output.rect.right())
            .max()
            .unwrap_or(0) as i32;
        let x = output_origin(existing_outputs, max_right);
        let name = candidate.name.clone();
        let (width, height) = candidate.mode.size();

        let output = Output::new(
            name.clone(),
            PhysicalProperties {
                size: {
                    let (width, height) = candidate.connector.size().unwrap_or((0, 0));
                    (width as i32, height as i32).into()
                },
                subpixel: Subpixel::Unknown,
                make: "Aether".to_string(),
                model: name.clone(),
            },
        );
        let mode = OutputMode::from(candidate.mode);
        output.set_preferred(mode);
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((x, 0).into()),
        );

        let info = OutputInfo {
            id: next_id,
            name,
            rect: AetherRect::new(x, 0, width as u32, height as u32),
            scale: 1.0,
            primary: self.app.shell().primary_output().is_none(),
        };

        let DeviceBackend {
            renderer,
            manager,
            surfaces,
            ..
        } = match self.devices.get_mut(&node) {
            Some(device) => device,
            None => return,
        };

        let planes = manager.device().planes(&candidate.crtc).ok();
        println!(
            "aether-shell-drm: enabling {} {}x{} on {:?}",
            candidate.name, width, height, candidate.crtc
        );
        let elements: DrmOutputRenderElements<GlesRenderer, AetherRenderElement> =
            DrmOutputRenderElements::default();
        let drm_output = match manager.initialize_output(
            candidate.crtc,
            candidate.mode,
            &[candidate.connector.handle()],
            &output,
            planes,
            renderer,
            &elements,
        ) {
            Ok(output) => output,
            Err(error) => {
                eprintln!(
                    "aether-shell-drm: cannot enable connector {}: {error}",
                    candidate.connector
                );
                return;
            }
        };

        println!("aether-shell-drm: connector {} enabled", candidate.name);
        surfaces.insert(
            candidate.crtc,
            OutputSurface {
                id: next_id,
                connector: candidate.connector.handle(),
                drm_output,
                dirty: true,
            },
        );
        self.next_output_id += 1;
        self.app.add_output(info, output);
    }

    fn remove_output(&mut self, node: DrmNode, connector: connector::Handle, id: OutputId) {
        if let Some(device) = self.devices.get_mut(&node) {
            let crtc = device
                .surfaces
                .iter()
                .find_map(|(crtc, surface)| (surface.connector == connector).then_some(*crtc));
            if let Some(crtc) = crtc {
                device.surfaces.remove(&crtc);
            }
        }
        self.app.remove_output(id);
    }

    fn frame_submitted(&mut self, node: DrmNode, crtc: crtc::Handle) {
        if let Some(device) = self.devices.get_mut(&node) {
            if let Some(surface) = device.surfaces.get_mut(&crtc) {
                let _ = surface.drm_output.frame_submitted();
                surface.dirty = self.app.needs_frame();
            }
        }
    }

    fn input(&mut self, event: InputEvent<LibinputInputBackend>) {
        match event {
            InputEvent::Keyboard { event } => {
                self.app.keyboard_key(
                    &self.keyboard,
                    event.key_code(),
                    event.state(),
                    event.time_msec(),
                );
                self.app.flush_focus(&self.keyboard);
            }
            InputEvent::PointerMotion { event } => {
                self.app
                    .pointer_motion_relative(event.delta().into(), event.time_msec());
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let current = self.app.pointer_location();
                let point =
                    aether_geometry::Point::new(current.x.round() as i32, current.y.round() as i32);
                let output = self
                    .app
                    .shell()
                    .output_at(point)
                    .or_else(|| self.app.shell().primary_output().map(|output| output.id))
                    .unwrap_or(1);
                let bounds = self.app.shell().output_bounds(output);
                let local = event
                    .position_transformed(SSize::from((bounds.width as i32, bounds.height as i32)));
                self.app.pointer_motion(
                    (bounds.x as f64 + local.x, bounds.y as f64 + local.y).into(),
                    event.time_msec(),
                );
            }
            InputEvent::PointerButton { event } => {
                self.app
                    .pointer_button(event.button_code(), event.state(), event.time_msec());
            }
            InputEvent::PointerAxis { event } => {
                let source = event.source();
                let horizontal = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.0
                });
                let vertical = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.0
                });
                self.app
                    .pointer_axis(horizontal, vertical, source, event.time_msec());
            }
            InputEvent::DeviceAdded { .. } | InputEvent::DeviceRemoved { .. } => {}
            _ => {}
        }
        self.mark_all_dirty();
    }

    fn mark_all_dirty(&mut self) {
        for device in self.devices.values_mut() {
            for surface in device.surfaces.values_mut() {
                surface.dirty = true;
            }
        }
    }

    fn render_all(&mut self) -> bool {
        let DrmState { app, devices, .. } = self;
        let mut presented = false;
        for device in devices.values_mut() {
            let DeviceBackend {
                renderer, surfaces, ..
            } = device;
            for surface in surfaces.values_mut() {
                if !surface.dirty {
                    continue;
                }
                let composed = app.compose(renderer, Some(surface.id));
                let rendered = surface.drm_output.render_frame(
                    renderer,
                    &composed.elements,
                    composed.clear,
                    FrameFlags::DEFAULT,
                );
                match rendered {
                    Ok(frame) => {
                        surface.dirty = false;
                        if frame.is_empty {
                            continue;
                        }
                        if let Err(error) = surface.drm_output.queue_frame(()) {
                            eprintln!("aether-shell-drm: queue_frame failed: {error}");
                        } else {
                            presented = true;
                        }
                    }
                    Err(RenderFrameError::PrepareFrame(FrameError::EmptyFrame)) => {
                        surface.dirty = false;
                    }
                    Err(RenderFrameError::PrepareFrame(FrameError::NoFreeSlotsError)) => {
                        // A flip is still in flight; retry after its VBlank.
                    }
                    Err(error) => {
                        eprintln!("aether-shell-drm: render failed: {error}");
                        surface.dirty = false;
                    }
                }
            }
        }
        let _ = app.take_damage();
        presented
    }
}

struct ConnectorCandidate {
    connector: connector::Info,
    crtc: crtc::Handle,
    mode: smithay::reexports::drm::control::Mode,
    name: String,
}

fn enumerate_connectors(
    device: &DeviceBackend,
    used: &[crtc::Handle],
) -> Result<Vec<ConnectorCandidate>, Box<dyn Error>> {
    let drm = device.manager.device();
    let resources = drm.resource_handles()?;
    let mut candidates = Vec::new();
    let mut taken: Vec<crtc::Handle> = used.to_vec();

    for handle in resources.connectors() {
        let connector = drm.get_connector(*handle, false)?;
        println!(
            "aether-shell-drm: connector {} state {:?} modes {}",
            connector,
            connector.state(),
            connector.modes().len()
        );
        if connector.state() != connector::State::Connected {
            continue;
        }
        let Some(mode) = connector
            .modes()
            .iter()
            .copied()
            .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
            .or_else(|| connector.modes().first().copied())
        else {
            continue;
        };

        let mut allowed: Vec<crtc::Handle> = Vec::new();
        for encoder_handle in connector.encoders() {
            let Ok(encoder) = drm.get_encoder(*encoder_handle) else {
                continue;
            };
            for crtc in resources.filter_crtcs(encoder.possible_crtcs()) {
                if !allowed.contains(&crtc) {
                    allowed.push(crtc);
                }
            }
        }
        let Some(crtc) = allowed.into_iter().find(|crtc| !taken.contains(crtc)) else {
            continue;
        };
        taken.push(crtc);
        candidates.push(ConnectorCandidate {
            name: format!(
                "{}-{}",
                connector.interface().as_str(),
                connector.interface_id()
            ),
            connector,
            crtc,
            mode,
        });
    }
    Ok(candidates)
}

/// First output starts at the desktop origin; later outputs are appended to
/// the right of the current extended desktop.
fn output_origin(existing_outputs: usize, max_right: i32) -> i32 {
    if existing_outputs == 0 {
        0
    } else {
        max_right
    }
}

fn parse_frames() -> Option<u32> {
    parse_frames_from(std::env::args().skip(1))
}

fn parse_frames_from(args: impl Iterator<Item = String>) -> Option<u32> {
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--frames" {
            return args.next().and_then(|value| value.parse().ok());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_output_uses_desktop_origin() {
        assert_eq!(output_origin(0, 1280), 0);
        assert_eq!(output_origin(0, 0), 0);
    }

    #[test]
    fn later_outputs_append_to_the_right() {
        assert_eq!(output_origin(1, 1280), 1280);
        assert_eq!(output_origin(2, 3200), 3200);
    }

    #[test]
    fn frames_flag_is_parsed() {
        assert_eq!(
            parse_frames_from(["--frames".to_string(), "120".to_string()].into_iter()),
            Some(120)
        );
        assert_eq!(
            parse_frames_from(["--frames".to_string()].into_iter()),
            None
        );
        assert_eq!(parse_frames_from(std::iter::empty()), None);
    }
}
