//! Smithay handler implementations that wire protocol requests into [`App`].

use aether_geometry::{Point as AetherPoint, Rect as AetherRect, Size as AetherSize};
use aether_shell::WindowSpec;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::input::pointer::CursorImageStatus;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::utils::{Logical, Rectangle, Serial};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    with_states, CompositorClientState, CompositorHandler, CompositorState, Damage,
    SurfaceAttributes,
};
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface as InputMethodPopupSurface};
use smithay::wayland::output::OutputHandler;
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
use smithay::wayland::text_input::TextInputSeat;

use super::damage::buffer_damage_to_logical;

use super::input::{cursor_for, pointer_shape};
use super::App;

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
            // The title lives in compositor-drawn chrome, so nothing in the
            // client's buffer commit necessarily repaints it. Damage the frame
            // area (the window rect) when the title changes.
            if let Some(rect) = self.window_rect(id) {
                self.mark_window_damage(rect);
            }
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
            // The frame outlives the client buffer and retiling moves the
            // remaining windows, so repaint the output even when focus did
            // not change.
            self.mark_full_damage();
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
        let Some(content) = self.window_content_rect(window.id) else {
            self.mark_full_damage();
            return;
        };

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
            self.mark_window_damage(content);
            return;
        }
        for damage in damage {
            match damage {
                Damage::Surface(rect) => {
                    if rect.size.w <= 0 || rect.size.h <= 0 {
                        continue;
                    }
                    let rect = AetherRect::new(
                        content.x + rect.loc.x,
                        content.y + rect.loc.y,
                        rect.size.w as u32,
                        rect.size.h as u32,
                    );
                    if let Some(clipped) = rect.intersect(content) {
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
                            content.x + local.x,
                            content.y + local.y,
                            local.width,
                            local.height,
                        );
                        if let Some(clipped) = rect.intersect(content) {
                            self.mark_damage_rect(clipped);
                        }
                    } else {
                        self.mark_window_damage(content);
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
            .and_then(|id| self.window_content_rect(id))
            .map(|rect| {
                Rectangle::new(
                    (rect.x, rect.y).into(),
                    (rect.width as i32, rect.height as i32).into(),
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

fn toplevel_title(surface: &ToplevelSurface) -> Option<String> {
    with_states(surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok())
            .and_then(|attributes| attributes.title.clone())
    })
}

#[derive(Default)]
pub(crate) struct ClientState {
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
