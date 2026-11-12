//! Backend-neutral shell state.
//!
//! `aether-shell` owns windows, workspaces, focus, tiling and the overlay
//! surface. It produces [`FramePlan`]s for a backend to present. It does not
//! know about Wayland, DRM, GPU APIs or input devices, so it is fully testable
//! on any machine and can be driven by the Smithay backend on the target or by
//! the software preview renderer during development.
//!
//! ```text
//! inputs (keys, IPC) ──> Shell (this crate) ──> FramePlan ──> backend
//!                          │
//!                          └── aether-scene / aether-layout / aether-pacing
//! ```
//!
//! The compositor backend maps Wayland buffers into the scene; the shell only
//! decides which surfaces exist, where they are, and what changed.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use aether_damage::Region;
use aether_geometry::{Point, Rect, Size};
use aether_layout::{tile, LayoutConfig};
use aether_pacing::FramePacer;
use aether_scene::{Layer, Scene, SurfaceId};

pub type WindowId = u64;

#[derive(Clone, Debug)]
pub struct WindowSpec {
    pub title: String,
    pub app_id: String,
    pub preferred_size: Size,
    pub floating: bool,
}

impl WindowSpec {
    pub fn new(title: impl Into<String>, app_id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            app_id: app_id.into(),
            preferred_size: Size::new(900, 700),
            floating: false,
        }
    }
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self::new("window", "application")
    }
}

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub workspace: usize,
    pub rect: Rect,
    pub floating: bool,
    pub focused: bool,
}

#[derive(Clone, Debug)]
pub struct FrameItem {
    pub surface: SurfaceId,
    pub window: Option<WindowId>,
    pub rect: Rect,
    pub layer: Layer,
    pub title: String,
    pub focused: bool,
}

/// What a backend should draw for one frame. `damage` is the region that
/// changed since the previous present.
#[derive(Clone, Debug)]
pub struct FramePlan {
    pub viewport: Size,
    pub damage: Region,
    pub items: Vec<FrameItem>,
    pub focus: Option<WindowId>,
    pub workspace: usize,
    pub presented_ns: u64,
}

#[derive(Clone, Debug)]
struct Window {
    spec: WindowSpec,
    workspace: usize,
    surface: SurfaceId,
    floating: bool,
}

#[derive(Clone, Debug)]
struct Workspace {
    name: String,
    windows: Vec<WindowId>,
}

pub struct Shell {
    scene: Scene,
    layout: LayoutConfig,
    pacer: FramePacer,
    workspaces: Vec<Workspace>,
    windows: HashMap<WindowId, Window>,
    active: usize,
    focus: Option<WindowId>,
    overlay: Option<SurfaceId>,
    viewport: Size,
    next_id: WindowId,
}

impl Shell {
    pub fn new(viewport: Size, layout: LayoutConfig, refresh_hz: u32) -> Self {
        Self {
            scene: Scene::new(viewport),
            layout,
            pacer: FramePacer::new(refresh_hz),
            workspaces: vec![Workspace {
                name: "main".to_string(),
                windows: Vec::new(),
            }],
            windows: HashMap::new(),
            active: 0,
            focus: None,
            overlay: None,
            viewport,
            next_id: 1,
        }
    }

    pub fn viewport(&self) -> Size {
        self.viewport
    }

    pub fn layout(&self) -> LayoutConfig {
        self.layout
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    pub fn active_workspace(&self) -> usize {
        self.active
    }

    pub fn workspace_names(&self) -> Vec<String> {
        self.workspaces.iter().map(|ws| ws.name.clone()).collect()
    }

    pub fn add_workspace(&mut self, name: impl Into<String>) -> usize {
        self.workspaces.push(Workspace {
            name: name.into(),
            windows: Vec::new(),
        });
        self.workspaces.len() - 1
    }

    pub fn add_window(&mut self, spec: WindowSpec) -> WindowId {
        let id = self.next_id;
        self.next_id += 1;

        let size = self.clamp_size(spec.preferred_size);
        let floating = spec.floating;
        let rect = Rect::new(0, 0, size.width, size.height);
        let surface = self.scene.add(Layer::WINDOW, rect);
        let workspace = self.active;

        self.windows.insert(
            id,
            Window {
                spec,
                workspace,
                surface,
                floating,
            },
        );
        self.workspaces[workspace].windows.push(id);
        self.focus = Some(id);
        self.relayout();
        id
    }

    pub fn close_window(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.remove(&id) else {
            return false;
        };
        self.scene.remove(window.surface);
        if let Some(workspace) = self.workspaces.get_mut(window.workspace) {
            workspace.windows.retain(|candidate| *candidate != id);
        }
        if self.focus == Some(id) {
            self.focus = self.workspaces[self.active].windows.last().copied();
        }
        self.relayout();
        true
    }

    pub fn focus_window(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        if window.workspace != self.active {
            return false;
        }
        self.focus = Some(id);
        let surface = window.surface;
        self.scene.raise(surface);
        true
    }

    pub fn focus_next(&mut self) -> Option<WindowId> {
        let ids = self.workspaces[self.active].windows.clone();
        if ids.is_empty() {
            self.focus = None;
            return None;
        }
        let next = match self.focus.and_then(|f| ids.iter().position(|id| *id == f)) {
            Some(index) => (index + 1) % ids.len(),
            None => 0,
        };
        self.focus_window(ids[next]);
        Some(ids[next])
    }

    pub fn focus_previous(&mut self) -> Option<WindowId> {
        let ids = self.workspaces[self.active].windows.clone();
        if ids.is_empty() {
            self.focus = None;
            return None;
        }
        let previous = match self.focus.and_then(|f| ids.iter().position(|id| *id == f)) {
            Some(0) => ids.len() - 1,
            Some(index) => index - 1,
            None => ids.len() - 1,
        };
        self.focus_window(ids[previous]);
        Some(ids[previous])
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus
    }

    pub fn switch_workspace(&mut self, index: usize) -> bool {
        if index >= self.workspaces.len() || index == self.active {
            return false;
        }

        let outgoing = self.workspaces[self.active].windows.clone();
        for id in outgoing {
            if let Some(window) = self.windows.get(&id) {
                self.scene.set_visible(window.surface, false);
            }
        }

        self.active = index;
        let incoming = self.workspaces[index].windows.clone();
        for id in &incoming {
            if let Some(window) = self.windows.get(id) {
                self.scene.set_visible(window.surface, true);
            }
        }

        self.focus = incoming.last().copied();
        self.relayout();
        true
    }

    pub fn move_window_to_workspace(&mut self, id: WindowId, target: usize) -> bool {
        if target >= self.workspaces.len() {
            return false;
        }
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let current = window.workspace;
        if current == target {
            return false;
        }
        let surface = window.surface;

        self.workspaces[current]
            .windows
            .retain(|candidate| *candidate != id);
        self.workspaces[target].windows.push(id);
        if let Some(window) = self.windows.get_mut(&id) {
            window.workspace = target;
        }

        if target != self.active {
            self.scene.set_visible(surface, false);
            if self.focus == Some(id) {
                self.focus = self.workspaces[self.active].windows.last().copied();
            }
        } else {
            self.scene.set_visible(surface, true);
            self.focus = Some(id);
            self.scene.raise(surface);
        }
        self.relayout();
        true
    }

    pub fn set_floating(&mut self, id: WindowId, floating: bool) -> bool {
        let Some(window) = self.windows.get_mut(&id) else {
            return false;
        };
        if window.floating == floating {
            return true;
        }
        window.floating = floating;
        self.relayout();
        true
    }

    pub fn toggle_floating(&mut self, id: WindowId) -> bool {
        let floating = self.windows.get(&id).map(|w| w.floating);
        match floating {
            Some(current) => self.set_floating(id, !current),
            None => false,
        }
    }

    /// Create or update the single overlay surface (palette/chat). Passing
    /// `None` removes it.
    pub fn set_overlay(&mut self, rect: Option<Rect>) -> Option<SurfaceId> {
        match (rect, self.overlay) {
            (Some(rect), Some(surface)) => {
                self.scene.move_to(surface, self.clamp_rect(rect));
                Some(surface)
            }
            (Some(rect), None) => {
                let surface = self.scene.add(Layer::OVERLAY, self.clamp_rect(rect));
                self.overlay = Some(surface);
                Some(surface)
            }
            (None, Some(surface)) => {
                self.scene.remove(surface);
                self.overlay = None;
                None
            }
            (None, None) => None,
        }
    }

    pub fn overlay(&self) -> Option<SurfaceId> {
        self.overlay
    }

    pub fn overlay_rect(&self) -> Option<Rect> {
        self.overlay
            .and_then(|surface| self.scene.get(surface).map(|surface| surface.rect))
    }

    /// Simulate content damage for a window (text append, resize of app
    /// content). Clipped to the surface by the scene.
    pub fn damage_window(&mut self, id: WindowId, rect: Rect) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        self.scene.add_damage(window.surface, rect)
    }

    pub fn set_viewport(&mut self, viewport: Size) {
        if viewport == self.viewport {
            return;
        }
        self.viewport = viewport;
        self.scene.resize_viewport(viewport);
        self.relayout();
    }

    pub fn window_at(&self, point: Point) -> Option<WindowId> {
        let surface = self.scene.hit_test(point)?;
        if Some(surface) == self.overlay {
            return None;
        }
        self.windows
            .iter()
            .find_map(|(id, window)| (window.surface == surface).then_some(*id))
    }

    pub fn windows(&self) -> Vec<WindowInfo> {
        let mut infos: Vec<WindowInfo> = self
            .windows
            .iter()
            .map(|(id, window)| WindowInfo {
                id: *id,
                title: window.spec.title.clone(),
                app_id: window.spec.app_id.clone(),
                workspace: window.workspace,
                rect: self
                    .scene
                    .get(window.surface)
                    .map(|surface| surface.rect)
                    .unwrap_or_default(),
                floating: window.floating,
                focused: self.focus == Some(*id),
            })
            .collect();
        infos.sort_by_key(|window| window.id);
        infos
    }

    pub fn next_deadline(&self) -> Option<u64> {
        self.pacer.deadline()
    }

    /// Produce a frame if the pacer is due and something changed. The caller
    /// hands the plan to a backend (Smithay, software preview, remote encoder)
    /// and clears it by presenting.
    pub fn frame(&mut self, now_ns: u64) -> Option<FramePlan> {
        if !self.pacer.should_present(now_ns) {
            return None;
        }

        let damage = self.scene.commit().clone();
        if damage.is_empty() {
            return None;
        }
        // The frame is the presentation point: the damage is now owned by the
        // plan, so the scene starts the next frame clean.
        self.scene.clear_damage();
        self.pacer.on_present(now_ns);

        let mut items = Vec::new();
        for surface_id in self.scene.draw_order() {
            let Some(surface) = self.scene.get(surface_id) else {
                continue;
            };
            let window = self
                .windows
                .iter()
                .find_map(|(id, window)| (window.surface == surface_id).then_some(*id));
            let (title, focused) = match window {
                Some(id) => (
                    self.windows
                        .get(&id)
                        .map(|window| window.spec.title.clone())
                        .unwrap_or_default(),
                    self.focus == Some(id),
                ),
                None if Some(surface_id) == self.overlay => ("Command Palette".to_string(), false),
                None => ("surface".to_string(), false),
            };
            items.push(FrameItem {
                surface: surface_id,
                window,
                rect: surface.rect,
                layer: surface.layer,
                title,
                focused,
            });
        }

        Some(FramePlan {
            viewport: self.viewport,
            damage,
            items,
            focus: self.focus,
            workspace: self.active,
            presented_ns: now_ns,
        })
    }

    fn relayout(&mut self) {
        let ids = self.workspaces[self.active].windows.clone();

        let tiled: Vec<(WindowId, SurfaceId)> = ids
            .iter()
            .filter_map(|id| {
                let window = self.windows.get(id)?;
                (!window.floating).then_some((*id, window.surface))
            })
            .collect();
        let rects = tile(Rect::from_size(self.viewport), tiled.len(), self.layout);
        for ((_, surface), rect) in tiled.iter().zip(rects) {
            self.scene.move_to(*surface, rect);
        }

        let floating: Vec<(SurfaceId, Size)> = ids
            .iter()
            .filter_map(|id| {
                let window = self.windows.get(id)?;
                window
                    .floating
                    .then_some((window.surface, window.spec.preferred_size))
            })
            .collect();
        for (index, (surface, size)) in floating.into_iter().enumerate() {
            self.scene.move_to(surface, self.floating_rect(index, size));
        }

        if let Some(focus) = self.focus {
            if let Some(window) = self.windows.get(&focus) {
                self.scene.raise(window.surface);
            }
        }
    }

    fn floating_rect(&self, index: usize, size: Size) -> Rect {
        let bounds = Rect::from_size(self.viewport).inset(self.layout.outer_gap as i32);
        if bounds.is_empty() {
            return Rect::new(bounds.x, bounds.y, 0, 0);
        }
        let width = size.width.min(bounds.width).max(1);
        let height = size.height.min(bounds.height).max(1);
        let offset = (index as i32) * 32;
        let x = bounds.x + ((bounds.width - width) / 2) as i32 + offset;
        let y = bounds.y + ((bounds.height - height) / 2) as i32 + offset;
        self.clamp_rect(Rect::new(x, y, width, height))
    }

    fn clamp_rect(&self, rect: Rect) -> Rect {
        let bounds = Rect::from_size(self.viewport);
        if bounds.is_empty() || rect.is_empty() {
            return rect;
        }
        let width = rect.width.min(bounds.width).max(1);
        let height = rect.height.min(bounds.height).max(1);
        let max_x = (bounds.x + bounds.width as i32 - width as i32).max(bounds.x);
        let max_y = (bounds.y + bounds.height as i32 - height as i32).max(bounds.y);
        Rect::new(
            rect.x.clamp(bounds.x, max_x),
            rect.y.clamp(bounds.y, max_y),
            width,
            height,
        )
    }

    fn clamp_size(&self, size: Size) -> Size {
        Size::new(
            size.width.clamp(1, self.viewport.width.max(1)),
            size.height.clamp(1, self.viewport.height.max(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        Shell::new(Size::new(800, 600), LayoutConfig::default(), 120)
    }

    fn window(shell: &mut Shell, title: &str) -> WindowId {
        shell.add_window(WindowSpec::new(title, "test.app"))
    }

    #[test]
    fn windows_tile_without_overlap() {
        let mut shell = shell();
        for title in ["one", "two", "three", "four"] {
            window(&mut shell, title);
        }

        let windows = shell.windows();
        assert_eq!(windows.len(), 4);
        for (index, a) in windows.iter().enumerate() {
            for b in windows.iter().skip(index + 1) {
                assert!(
                    a.rect.intersect(b.rect).is_none(),
                    "overlap: {:?} and {:?}",
                    a.rect,
                    b.rect
                );
            }
            assert_eq!(
                a.rect.intersect(Rect::from_size(shell.viewport())),
                Some(a.rect)
            );
        }
    }

    #[test]
    fn focus_cycles_and_raises() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        let b = window(&mut shell, "b");

        assert_eq!(shell.focus_next(), Some(a));
        let plan = shell.frame(0).expect("frame");
        assert_eq!(plan.items.last().and_then(|item| item.window), Some(a));

        assert_eq!(shell.focus_previous(), Some(b));
        let plan = shell.frame(10_000_000).expect("frame");
        assert_eq!(plan.items.last().and_then(|item| item.window), Some(b));
        assert_eq!(plan.focus, Some(b));
    }

    #[test]
    fn workspace_switch_hides_and_shows_windows() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        let workspace = shell.add_workspace("work");

        assert!(shell.move_window_to_workspace(a, workspace));
        let empty = shell.frame(0).expect("frame");
        assert!(empty.items.is_empty());
        assert_eq!(shell.active_workspace(), 0);

        assert!(shell.switch_workspace(workspace));
        let plan = shell.frame(10_000_000).expect("frame");
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].window, Some(a));
        assert_eq!(plan.workspace, workspace);
    }

    #[test]
    fn closing_a_window_relayouts_the_rest() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        window(&mut shell, "b");

        assert!(shell.close_window(a));
        let windows = shell.windows();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].rect, Rect::from_size(shell.viewport()).inset(8));
        assert!(!shell.close_window(a));
    }

    #[test]
    fn floating_windows_are_centered_and_not_tiled() {
        let mut shell = shell();
        window(&mut shell, "tiled");
        let floating = shell.add_window(WindowSpec {
            title: "palette-host".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: true,
        });

        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == floating)
            .expect("floating window");
        assert!(info.floating);
        assert_eq!(info.rect.width, 320);
        assert_eq!(info.rect.height, 200);
        assert!(info.rect.x > 0 && info.rect.y > 0);
    }

    #[test]
    fn overlay_is_topmost_and_blocks_window_hit_testing() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        let overlay = shell
            .set_overlay(Some(Rect::new(100, 80, 400, 300)))
            .expect("overlay");

        let plan = shell.frame(0).expect("frame");
        let last = plan.items.last().expect("item");
        assert_eq!(last.surface, overlay);
        assert_eq!(last.layer, Layer::OVERLAY);
        assert_eq!(shell.window_at(Point::new(120, 100)), None);

        shell.set_overlay(None);
        let plan = shell.frame(10_000_000).expect("frame");
        assert!(plan.items.iter().all(|item| item.surface != overlay));
        assert_eq!(shell.window_at(Point::new(120, 100)), Some(a));
    }

    #[test]
    fn frame_pacing_is_deadline_based() {
        let mut shell = shell();
        let a = window(&mut shell, "a");

        assert!(shell.frame(0).is_some(), "first frame is immediate");
        assert!(shell.frame(1_000_000).is_none(), "not due yet");

        assert!(shell.damage_window(a, Rect::new(10, 10, 40, 20)));
        assert!(shell.frame(8_400_000).is_some(), "deadline reached");
    }

    #[test]
    fn content_damage_is_clipped_to_the_surface() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        assert!(shell.frame(0).is_some());

        assert!(shell.damage_window(a, Rect::new(-100, -100, 200, 200)));
        let plan = shell.frame(10_000_000).expect("frame");
        let info = shell.windows().into_iter().find(|w| w.id == a).unwrap();
        let clipped = Rect::new(-100, -100, 200, 200)
            .intersect(info.rect)
            .expect("overlap");
        assert_eq!(plan.damage.area(), clipped.area());
    }

    #[test]
    fn viewport_change_relayouts_and_damages() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        assert!(shell.frame(0).is_some());

        shell.set_viewport(Size::new(1024, 768));
        assert_eq!(shell.viewport(), Size::new(1024, 768));
        let plan = shell.frame(10_000_000).expect("frame");
        assert!(!plan.damage.is_empty());

        let info = shell.windows().into_iter().find(|w| w.id == a).unwrap();
        assert!(info.rect.intersect(Rect::from_size(Size::new(1024, 768))) == Some(info.rect));
    }

    #[test]
    fn window_hit_testing_maps_to_ids() {
        let mut shell = shell();
        let a = window(&mut shell, "a");
        let info = shell.windows().into_iter().find(|w| w.id == a).unwrap();
        let point = Point::new(info.rect.x + 5, info.rect.y + 5);
        assert_eq!(shell.window_at(point), Some(a));
        assert_eq!(shell.window_at(Point::new(-10, -10)), None);
    }
}
