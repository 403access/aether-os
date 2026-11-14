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
pub type OutputId = u64;

/// A logical output in the shell's extended desktop.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputInfo {
    pub id: OutputId,
    pub name: String,
    pub rect: Rect,
    pub scale: f64,
    pub primary: bool,
}

/// Top-left of the first workspace indicator.
pub const WORKSPACE_INDICATOR_X: i32 = 10;
/// Top of the workspace indicator strip.
pub const WORKSPACE_INDICATOR_Y: i32 = 10;
/// Width of one indicator bar.
pub const WORKSPACE_INDICATOR_WIDTH: u32 = 66;
/// Height of the coloured bar itself.
pub const WORKSPACE_INDICATOR_BAR_HEIGHT: u32 = 6;
/// Horizontal distance between indicator origins.
pub const WORKSPACE_INDICATOR_SPACING: i32 = 76;
/// Hit height including the label below the bar.
pub const WORKSPACE_INDICATOR_HIT_HEIGHT: u32 = 26;

/// Visible rows in the palette list.
pub const PALETTE_VISIBLE_ROWS: usize = 4;
/// Height of one result row.
pub const PALETTE_ROW_HEIGHT: u32 = 32;
/// Vertical distance between result rows.
pub const PALETTE_ROW_SPACING: u32 = 48;
/// Y offset of the first result row inside the overlay rect.
pub const PALETTE_CONTENT_TOP: i32 = 96;

/// Minimum width when resizing a floating window.
pub const MIN_FLOATING_WIDTH: u32 = 160;
/// Minimum height when resizing a floating window.
pub const MIN_FLOATING_HEIGHT: u32 = 120;
/// Distance from an output edge where a moving window snaps flush.
pub const FLOATING_SNAP_DISTANCE: i32 = 16;

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
    pub output: OutputId,
    pub rect: Rect,
    pub floating: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub focused: bool,
}

/// A command palette result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    pub title: String,
    pub detail: String,
    pub action: PaletteAction,
}

/// What activating a palette entry does. The backend performs side effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteAction {
    SwitchWorkspace(usize),
    FocusWindow(WindowId),
    LaunchApp(usize),
    SpawnTerminal,
    OpenFile(usize),
    Close,
}

/// An application the palette can launch. The backend spawns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteLauncher {
    pub name: String,
    pub detail: String,
    pub icon: Option<String>,
    pub program: String,
    pub args: Vec<String>,
}

/// A file the palette can open. The backend hands the path to an opener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteFile {
    pub name: String,
    pub detail: String,
    pub path: String,
}

/// Edge zones a moving window can snap into on release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapZone {
    LeftHalf,
    RightHalf,
    Maximize,
}

#[derive(Clone, Debug, Default)]
struct PaletteState {
    query: String,
    selected: usize,
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
    output: OutputId,
    surface: SurfaceId,
    floating: bool,
    floating_rect: Option<Rect>,
    maximized: bool,
    fullscreen: bool,
    restore: Option<WindowRestore>,
}

#[derive(Clone, Copy, Debug)]
struct WindowRestore {
    floating: bool,
    floating_rect: Option<Rect>,
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
    outputs: Vec<OutputInfo>,
    output_workspaces: HashMap<OutputId, usize>,
    palette: PaletteState,
    launchers: Vec<PaletteLauncher>,
    files: Vec<PaletteFile>,
    files_prefiltered: bool,
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
            outputs: vec![OutputInfo {
                id: 1,
                name: "output-1".to_string(),
                rect: Rect::from_size(viewport),
                scale: 1.0,
                primary: true,
            }],
            output_workspaces: HashMap::from([(1, 0)]),
            palette: PaletteState::default(),
            launchers: Vec::new(),
            files: Vec::new(),
            files_prefiltered: false,
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

    /// Current master/stack split ratio.
    pub fn master_ratio(&self) -> f32 {
        self.layout.master_ratio
    }

    /// Update the master/stack split. Returns `false` when unchanged.
    pub fn set_master_ratio(&mut self, ratio: f32) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        let ratio = ratio.clamp(0.2, 0.8);
        if (self.layout.master_ratio - ratio).abs() < 0.0001 {
            return false;
        }
        self.layout.master_ratio = ratio;
        self.relayout();
        true
    }

    /// Draggable divider between the master window and the first stack column.
    /// `None` unless the active workspace has at least two tiled windows.
    pub fn master_gap_rect(&self) -> Option<Rect> {
        let output = self.primary_output()?.clone();
        let ids = &self.workspaces[self.output_workspace(output.id)].windows;
        let tiled = ids
            .iter()
            .filter(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| !window.floating && window.output == output.id)
            })
            .count();
        if tiled < 2 {
            return None;
        }
        let rects = tile(output.rect, tiled, self.layout);
        let master = rects[0];
        let stack = rects[1];
        let gap_start = master.x + master.width as i32;
        Some(Rect::new(
            gap_start,
            master.y,
            (stack.x - gap_start).max(0) as u32,
            master.height,
        ))
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Update the title used for window lists and the palette.
    pub fn set_window_title(&mut self, id: WindowId, title: impl Into<String>) -> bool {
        match self.windows.get_mut(&id) {
            Some(window) => {
                window.spec.title = title.into();
                true
            }
            None => false,
        }
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

    /// Picker hit area for a workspace: the bar plus its label.
    pub fn workspace_indicator_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.workspaces.len() {
            return None;
        }
        Some(Rect::new(
            WORKSPACE_INDICATOR_X + index as i32 * WORKSPACE_INDICATOR_SPACING,
            WORKSPACE_INDICATOR_Y,
            WORKSPACE_INDICATOR_WIDTH,
            WORKSPACE_INDICATOR_HIT_HEIGHT,
        ))
    }

    /// Workspace whose indicator (bar or label) contains `point`.
    pub fn workspace_at(&self, point: Point) -> Option<usize> {
        (0..self.workspaces.len()).find(|&index| {
            self.workspace_indicator_rect(index)
                .is_some_and(|rect| rect.contains(point))
        })
    }

    pub fn add_workspace(&mut self, name: impl Into<String>) -> usize {
        self.workspaces.push(Workspace {
            name: name.into(),
            windows: Vec::new(),
        });
        self.workspaces.len() - 1
    }

    /// Add a window directly on an output's currently visible workspace.
    /// New clients never appear on a workspace that output is not showing.
    pub fn add_window_on_output(&mut self, spec: WindowSpec, output: OutputId) -> WindowId {
        if !self.outputs.iter().any(|candidate| candidate.id == output) {
            return self.add_window(spec);
        }
        let target = self.output_workspace(output);
        let id = self.add_window(spec);
        if let Some(window) = self.windows.get_mut(&id) {
            let previous = window.workspace;
            if previous != target {
                if let Some(workspace) = self.workspaces.get_mut(previous) {
                    workspace.windows.retain(|candidate| *candidate != id);
                }
                if let Some(workspace) = self.workspaces.get_mut(target) {
                    workspace.windows.push(id);
                }
                window.workspace = target;
            }
            window.output = output;
        }
        self.relayout();
        id
    }

    pub fn add_window(&mut self, spec: WindowSpec) -> WindowId {
        let id = self.next_id;
        self.next_id += 1;

        let size = self.clamp_size(spec.preferred_size);
        let floating = spec.floating;
        let rect = Rect::new(0, 0, size.width, size.height);
        let surface = self.scene.add(Layer::WINDOW, rect);
        let workspace = self.active;
        let output = self.primary_output().map(|output| output.id).unwrap_or(1);

        self.windows.insert(
            id,
            Window {
                spec,
                workspace,
                output,
                surface,
                floating,
                floating_rect: None,
                maximized: false,
                fullscreen: false,
                restore: None,
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
        let output_workspace = self.output_workspace(window.output);
        if window.workspace != output_workspace {
            return false;
        }
        self.focus = Some(id);
        let surface = window.surface;
        self.scene.raise(surface);
        true
    }

    pub fn focus_next(&mut self) -> Option<WindowId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        let ids = self.visible_windows_on(primary);
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
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        let ids = self.visible_windows_on(primary);
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

        self.active = index;
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.output_workspaces.insert(primary, index);
        self.focus = self.last_window_on_workspace(primary, index);
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

        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        if target != self.output_workspace(output) {
            if self.focus == Some(id) {
                self.focus = self.visible_windows_on(output).last().copied();
            }
        } else {
            self.focus = Some(id);
            self.scene.raise(surface);
        }
        self.relayout();
        true
    }

    pub fn set_floating(&mut self, id: WindowId, floating: bool) -> bool {
        let Some((surface, current_floating, preferred)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.floating, window.spec.preferred_size))
        else {
            return false;
        };
        if current_floating == floating {
            return true;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let output_bounds = self.output_bounds(output);
        let floating_rect = if floating {
            if current_floating {
                self.scene.get(surface).map(|surface| surface.rect)
            } else {
                Some(self.floating_rect(0, preferred, output_bounds))
            }
        } else {
            None
        };
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = floating;
            window.maximized = false;
            window.restore = None;
            window.floating_rect = floating_rect;
        }
        self.relayout();
        true
    }

    /// Start an interactive move. Promotes a tiled window to its preferred
    /// floating size, centred on the pointer, and returns the new rect.
    pub fn begin_floating_move(&mut self, id: WindowId, pointer: Point) -> Option<Rect> {
        let window = self.windows.get(&id)?;
        if window.maximized {
            return None;
        }
        let surface = window.surface;
        let floating = window.floating;
        let preferred = window.spec.preferred_size;
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let rect = if floating {
            self.scene.get(surface)?.rect
        } else {
            let bounds = self
                .output_bounds(output)
                .inset(self.layout.outer_gap as i32);
            let width = preferred.width.min(bounds.width).max(1);
            let height = preferred.height.min(bounds.height).max(1);
            self.clamp_rect_to(
                Rect::new(
                    pointer.x - width as i32 / 2,
                    pointer.y - height as i32 / 2,
                    width,
                    height,
                ),
                self.output_bounds(output),
            )
        };
        let window = self.windows.get_mut(&id)?;
        window.floating = true;
        window.maximized = false;
        window.restore = None;
        window.floating_rect = Some(rect);
        self.relayout();
        Some(rect)
    }

    pub fn toggle_floating(&mut self, id: WindowId) -> bool {
        let floating = self.windows.get(&id).map(|w| w.floating);
        match floating {
            Some(current) => self.set_floating(id, !current),
            None => false,
        }
    }

    pub fn is_maximized(&self, id: WindowId) -> bool {
        self.windows.get(&id).is_some_and(|window| window.maximized)
    }

    pub fn is_fullscreen(&self, id: WindowId) -> bool {
        self.windows
            .get(&id)
            .is_some_and(|window| window.fullscreen)
    }

    /// Fullscreen window on the active workspace, if any.
    pub fn fullscreen_window(&self) -> Option<WindowId> {
        let primary = self.primary_output().map(|output| output.id).unwrap_or(1);
        self.visible_windows_on(primary)
            .into_iter()
            .find(|id| self.is_fullscreen(*id))
    }

    /// Toggle a window between its previous state and full-output fullscreen.
    pub fn toggle_fullscreen(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let (surface, fullscreen, floating, floating_rect) = (
            window.surface,
            window.fullscreen,
            window.floating,
            window.floating_rect,
        );

        if fullscreen {
            if let Some(window) = self.windows.get_mut(&id) {
                window.fullscreen = false;
                if let Some(restore) = window.restore.take() {
                    window.floating = restore.floating;
                    window.floating_rect = restore.floating_rect;
                    window.maximized = false;
                }
            }
        } else {
            let current = self.scene.get(surface).map(|surface| surface.rect);
            if let Some(window) = self.windows.get_mut(&id) {
                window.restore = Some(WindowRestore {
                    floating,
                    floating_rect,
                });
                window.fullscreen = true;
                window.maximized = false;
                window.floating = false;
                window.floating_rect = current;
            }
        }
        self.relayout();
        true
    }

    /// Toggle a window between its floating/tiled rect and a full viewport.
    pub fn toggle_maximized(&mut self, id: WindowId) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        let (surface, maximized, floating, floating_rect) = (
            window.surface,
            window.maximized,
            window.floating,
            window.floating_rect,
        );

        if maximized {
            if let Some(window) = self.windows.get_mut(&id) {
                window.maximized = false;
                if let Some(restore) = window.restore.take() {
                    window.floating = restore.floating;
                    window.floating_rect = restore.floating_rect;
                } else {
                    window.floating = false;
                    window.floating_rect = None;
                }
            }
        } else {
            let current = self.scene.get(surface).map(|surface| surface.rect);
            if let Some(window) = self.windows.get_mut(&id) {
                window.restore = Some(WindowRestore {
                    floating,
                    floating_rect,
                });
                window.maximized = true;
                window.floating = false;
                window.floating_rect = current;
            }
        }
        self.relayout();
        true
    }

    /// Move a window on screen, promoting it to floating if necessary.
    pub fn move_floating(&mut self, id: WindowId, position: Point) -> bool {
        let Some((surface, blocked)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.maximized || window.fullscreen))
        else {
            return false;
        };
        if blocked {
            return false;
        }
        let Some(current) = self.scene.get(surface).map(|surface| surface.rect) else {
            return false;
        };
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let bounds = self.output_bounds(output);
        let moved = self.clamp_rect_to(
            Rect::new(position.x, position.y, current.width, current.height),
            bounds,
        );
        let moved = self.snap_floating(moved, bounds);
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.floating_rect = Some(moved);
        }
        self.relayout();
        true
    }

    /// Start an interactive resize. Promotes a tiled window to floating while
    /// keeping its current rect, and returns that rect.
    pub fn begin_floating_resize(&mut self, id: WindowId) -> Option<Rect> {
        let window = self.windows.get(&id)?;
        if window.maximized || window.fullscreen {
            return None;
        }
        let surface = window.surface;
        let rect = self.scene.get(surface)?.rect;
        let window = self.windows.get_mut(&id)?;
        window.floating = true;
        window.maximized = false;
        window.restore = None;
        window.floating_rect = Some(rect);
        self.relayout();
        Some(rect)
    }

    /// Snap a window to a half or maximize it. Halves stay floating so they can
    /// be moved or resized afterwards.
    pub fn snap_window(&mut self, id: WindowId, zone: SnapZone) -> bool {
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        if window.maximized {
            return false;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let viewport = self.output_bounds(output);
        if zone == SnapZone::Maximize {
            let was_maximized = self.is_maximized(id);
            return if was_maximized {
                true
            } else {
                self.toggle_maximized(id)
            };
        }
        let half = viewport.width / 2;
        let rect = match zone {
            SnapZone::LeftHalf => Rect::new(viewport.x, viewport.y, half, viewport.height),
            SnapZone::RightHalf => Rect::new(
                viewport.x + half as i32,
                viewport.y,
                viewport.width - half,
                viewport.height,
            ),
            SnapZone::Maximize => unreachable!(),
        };
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.maximized = false;
            window.restore = None;
            window.floating_rect = Some(rect);
        }
        self.relayout();
        true
    }

    /// Resize a floating window, enforcing the minimum size and output bounds.
    pub fn resize_floating(&mut self, id: WindowId, rect: Rect) -> bool {
        let Some((surface, blocked)) = self
            .windows
            .get(&id)
            .map(|window| (window.surface, window.maximized || window.fullscreen))
        else {
            return false;
        };
        if blocked {
            return false;
        }
        let output = self
            .windows
            .get(&id)
            .map(|window| window.output)
            .unwrap_or(1);
        let bounds = self.output_bounds(output);
        let width = rect
            .width
            .max(MIN_FLOATING_WIDTH.min(bounds.width.max(1)))
            .min(bounds.width.max(1));
        let height = rect
            .height
            .max(MIN_FLOATING_HEIGHT.min(bounds.height.max(1)))
            .min(bounds.height.max(1));
        let resized = self.clamp_rect_to(Rect::new(rect.x, rect.y, width, height), bounds);
        if let Some(window) = self.windows.get_mut(&id) {
            window.floating = true;
            window.floating_rect = Some(resized);
        }
        if self.scene.get(surface).is_some() {
            self.relayout();
        }
        true
    }

    fn snap_floating(&self, rect: Rect, bounds: Rect) -> Rect {
        let mut x = rect.x;
        let mut y = rect.y;
        if x - bounds.x <= FLOATING_SNAP_DISTANCE {
            x = bounds.x;
        } else if (bounds.right() - (x as i64 + rect.width as i64)) <= FLOATING_SNAP_DISTANCE as i64
        {
            x = (bounds.width as i32 - rect.width as i32).max(0);
        }
        if y - bounds.y <= FLOATING_SNAP_DISTANCE {
            y = bounds.y;
        } else if (bounds.bottom() - (y as i64 + rect.height as i64))
            <= FLOATING_SNAP_DISTANCE as i64
        {
            y = (bounds.height as i32 - rect.height as i32).max(0);
        }
        Rect::new(x, y, rect.width, rect.height)
    }

    /// Open the overlay with a default centred rect and reset the query.
    pub fn open_overlay(&mut self) -> Option<SurfaceId> {
        self.palette = PaletteState::default();
        let width = 680.min(self.viewport.width.saturating_sub(64)).max(1);
        let height = 420.min(self.viewport.height.saturating_sub(64)).max(1);
        let x = ((self.viewport.width.saturating_sub(width)) / 2) as i32;
        let y = ((self.viewport.height.saturating_sub(height)) / 2) as i32;
        self.set_overlay(Some(Rect::new(x, y, width, height)))
    }

    /// Close the overlay and clear the palette query.
    pub fn close_overlay(&mut self) -> Option<SurfaceId> {
        self.palette = PaletteState::default();
        self.set_overlay(None)
    }

    /// Toggle the overlay. Returns whether it is now open.
    pub fn toggle_overlay(&mut self) -> bool {
        if self.overlay.is_some() {
            self.close_overlay();
            false
        } else {
            self.open_overlay();
            true
        }
    }

    /// Replace the application launchers shown in the palette.
    pub fn set_launchers(&mut self, launchers: Vec<PaletteLauncher>) {
        self.launchers = launchers;
        self.palette.selected = 0;
    }

    pub fn launchers(&self) -> &[PaletteLauncher] {
        &self.launchers
    }

    /// Clone a launcher by its palette index.
    pub fn launcher(&self, index: usize) -> Option<PaletteLauncher> {
        self.launchers.get(index).cloned()
    }

    /// Replace the filename index shown for non-empty queries.
    pub fn set_files(&mut self, files: Vec<PaletteFile>) {
        self.files = files;
        self.files_prefiltered = false;
        self.palette.selected = 0;
    }

    /// Replace the file list with results already matched by an external
    /// indexer. They are shown as-is for non-empty queries instead of being
    /// re-filtered by filename.
    pub fn set_indexed_files(&mut self, files: Vec<PaletteFile>) {
        self.files = files;
        self.files_prefiltered = true;
        self.palette.selected = 0;
    }

    pub fn files(&self) -> &[PaletteFile] {
        &self.files
    }

    /// Clone a file by its palette index.
    pub fn file(&self, index: usize) -> Option<PaletteFile> {
        self.files.get(index).cloned()
    }

    pub fn palette_query(&self) -> &str {
        &self.palette.query
    }

    /// Filtered palette entries. Workspaces are always available; the close
    /// command is last. Matching is case-insensitive on title, then detail.
    pub fn palette_entries(&self) -> Vec<PaletteEntry> {
        let query = self.palette.query.trim().to_lowercase();
        let mut scored: Vec<(u8, usize, PaletteEntry)> = Vec::new();

        for (index, workspace) in self.workspaces.iter().enumerate() {
            let detail = format!("workspace {}", index + 1);
            if let Some(score) = palette_score(&query, &workspace.name) {
                scored.push((
                    score,
                    index,
                    PaletteEntry {
                        title: workspace.name.clone(),
                        detail,
                        action: PaletteAction::SwitchWorkspace(index),
                    },
                ));
            }
        }

        let launcher_offset = self.workspaces.len();
        for (index, launcher) in self.launchers.iter().enumerate() {
            if let Some(score) = palette_score(&query, &launcher.name) {
                scored.push((
                    score,
                    launcher_offset + index,
                    PaletteEntry {
                        title: launcher.name.clone(),
                        detail: launcher.detail.clone(),
                        action: PaletteAction::LaunchApp(index),
                    },
                ));
            }
        }

        let command_offset = launcher_offset + self.launchers.len();
        let new_terminal = PaletteEntry {
            title: "New terminal".to_string(),
            detail: "launch".to_string(),
            action: PaletteAction::SpawnTerminal,
        };
        if let Some(score) = palette_score(&query, &new_terminal.title) {
            scored.push((score, command_offset, new_terminal));
        }

        let file_offset = command_offset + 1;
        if !query.is_empty() {
            let mut hits: Vec<(u8, usize, PaletteEntry)> = if self.files_prefiltered {
                self.files
                    .iter()
                    .enumerate()
                    .map(|(index, file)| {
                        (
                            0,
                            index,
                            PaletteEntry {
                                title: file.name.clone(),
                                detail: file.detail.clone(),
                                action: PaletteAction::OpenFile(index),
                            },
                        )
                    })
                    .collect()
            } else {
                self.files
                    .iter()
                    .enumerate()
                    .filter_map(|(index, file)| {
                        palette_score(&query, &file.name).map(|score| {
                            (
                                score,
                                index,
                                PaletteEntry {
                                    title: file.name.clone(),
                                    detail: file.detail.clone(),
                                    action: PaletteAction::OpenFile(index),
                                },
                            )
                        })
                    })
                    .collect()
            };
            hits.sort_by_key(|(score, index, _)| (*score, *index));
            for (rank, (score, _, entry)) in hits.into_iter().take(8).enumerate() {
                scored.push((score, file_offset + rank, entry));
            }
        }

        let window_offset = file_offset + 8;
        for (offset, window) in self.windows().into_iter().enumerate() {
            let detail = format!("window {} - workspace {}", window.id, window.workspace + 1);
            if let Some(score) = palette_score(&query, &window.title) {
                scored.push((
                    score,
                    window_offset + offset,
                    PaletteEntry {
                        title: window.title,
                        detail,
                        action: PaletteAction::FocusWindow(window.id),
                    },
                ));
            }
        }

        let close = PaletteEntry {
            title: "Close palette".to_string(),
            detail: "Esc".to_string(),
            action: PaletteAction::Close,
        };
        if let Some(score) = palette_score(&query, &close.title) {
            scored.push((score, window_offset + self.windows.len(), close));
        }

        scored.sort_by_key(|(score, order, _)| (*score, *order));
        scored.into_iter().map(|(_, _, entry)| entry).collect()
    }

    /// 0-based index into [`Shell::palette_entries`].
    pub fn palette_selected(&self) -> usize {
        let len = self.palette_entries().len();
        if len == 0 {
            0
        } else {
            self.palette.selected.min(len - 1)
        }
    }

    /// Select an entry by index. Returns whether the index is valid.
    pub fn palette_select(&mut self, index: usize) -> bool {
        if index >= self.palette_entries().len() {
            return false;
        }
        self.palette.selected = index;
        true
    }

    /// First entry shown in the four-row result window. Sliding the window
    /// keeps the selection visible without an explicit scroll state.
    pub fn palette_visible_start(&self) -> usize {
        let len = self.palette_entries().len();
        if len <= PALETTE_VISIBLE_ROWS {
            0
        } else {
            self.palette_selected()
                .saturating_sub(PALETTE_VISIBLE_ROWS - 1)
                .min(len - PALETTE_VISIBLE_ROWS)
        }
    }

    /// Screen rect of a visible palette row (0-based within the window).
    pub fn palette_row_rect(&self, row: usize) -> Option<Rect> {
        if row >= PALETTE_VISIBLE_ROWS {
            return None;
        }
        let overlay = self.overlay_rect()?;
        Some(Rect::new(
            overlay.x + 16,
            overlay.y + PALETTE_CONTENT_TOP + row as i32 * PALETTE_ROW_SPACING as i32,
            overlay.width.saturating_sub(32),
            PALETTE_ROW_HEIGHT,
        ))
    }

    /// Entry index under a screen point, accounting for the sliding window.
    pub fn palette_row_at(&self, point: Point) -> Option<usize> {
        let overlay = self.overlay_rect()?;
        let content_left = overlay.x + 16;
        let content_right = overlay.x + overlay.width as i32 - 16;
        if point.x < content_left || point.x >= content_right {
            return None;
        }
        let relative = point.y - overlay.y - PALETTE_CONTENT_TOP;
        if relative < 0 {
            return None;
        }
        let row = relative as usize / PALETTE_ROW_SPACING as usize;
        if row >= PALETTE_VISIBLE_ROWS {
            return None;
        }
        let index = self.palette_visible_start() + row;
        (index < self.palette_entries().len()).then_some(index)
    }

    /// Append a printable character to the query. Returns whether it changed.
    pub fn palette_push_char(&mut self, character: char) -> bool {
        if character.is_control() || self.palette.query.chars().count() >= 48 {
            return false;
        }
        self.palette.query.push(character);
        self.palette.selected = 0;
        true
    }

    /// Remove the last query character. Returns whether it changed.
    pub fn palette_backspace(&mut self) -> bool {
        let changed = self.palette.query.pop().is_some();
        if changed {
            self.palette.selected = 0;
        }
        changed
    }

    /// Move the selection by `delta`, wrapping at the ends.
    pub fn palette_move_selection(&mut self, delta: i32) -> bool {
        let len = self.palette_entries().len();
        if len < 2 || delta == 0 {
            return false;
        }
        let current = self.palette_selected() as i32;
        self.palette.selected = (current + delta).rem_euclid(len as i32) as usize;
        true
    }

    /// Activate the selected entry. The caller performs the returned action.
    pub fn palette_activate(&self) -> Option<PaletteAction> {
        self.palette_entries()
            .get(self.palette_selected())
            .map(|entry| entry.action)
    }

    /// Create or update the single overlay surface (palette/chat). Passing
    /// `None` removes it.
    pub fn set_overlay(&mut self, rect: Option<Rect>) -> Option<SurfaceId> {
        match (rect, self.overlay) {
            (Some(rect), Some(surface)) => {
                self.scene
                    .move_to(surface, self.clamp_rect_to(rect, self.ui_bounds()));
                Some(surface)
            }
            (Some(rect), None) => {
                let surface = self
                    .scene
                    .add(Layer::OVERLAY, self.clamp_rect_to(rect, self.ui_bounds()));
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

    /// Replace the output list. The viewport becomes the bounding box of all
    /// outputs, and a primary output is guaranteed.
    pub fn set_outputs(&mut self, mut outputs: Vec<OutputInfo>) -> bool {
        if outputs.is_empty() {
            return false;
        }
        if !outputs.iter().any(|output| output.primary) {
            outputs[0].primary = true;
        }
        let width = outputs
            .iter()
            .map(|output| output.rect.right().max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let height = outputs
            .iter()
            .map(|output| output.rect.bottom().max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let primary = outputs
            .iter()
            .find(|output| output.primary)
            .map(|output| output.id)
            .unwrap_or(outputs[0].id);
        self.output_workspaces
            .retain(|id, _| outputs.iter().any(|output| output.id == *id));
        for output in &outputs {
            self.output_workspaces.entry(output.id).or_insert(0);
        }
        if let Some(workspace) = self.output_workspaces.get_mut(&primary) {
            *workspace = self.active;
        }
        let valid: Vec<OutputId> = outputs.iter().map(|output| output.id).collect();
        for window in self.windows.values_mut() {
            if !valid.contains(&window.output) {
                window.output = primary;
            }
        }
        self.outputs = outputs;
        self.viewport = Size::new(width, height);
        self.scene.resize_viewport(self.viewport);
        self.relayout();
        true
    }

    pub fn outputs(&self) -> &[OutputInfo] {
        &self.outputs
    }

    pub fn primary_output(&self) -> Option<&OutputInfo> {
        self.outputs
            .iter()
            .find(|output| output.primary)
            .or_else(|| self.outputs.first())
    }

    pub fn output_at(&self, point: Point) -> Option<OutputId> {
        self.outputs
            .iter()
            .find(|output| output.rect.contains(point))
            .map(|output| output.id)
    }

    pub fn set_viewport(&mut self, viewport: Size) {
        if viewport == self.viewport {
            return;
        }
        self.viewport = viewport;
        if let Some(primary) = self.outputs.iter_mut().find(|output| output.primary) {
            primary.rect = Rect::from_size(viewport);
        } else if let Some(first) = self.outputs.first_mut() {
            first.rect = Rect::from_size(viewport);
            first.primary = true;
        }
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

    /// Window ids in compositor draw order, topmost first.
    pub fn window_stack(&self) -> Vec<WindowId> {
        self.scene
            .draw_order()
            .into_iter()
            .rev()
            .filter_map(|surface| {
                self.windows
                    .iter()
                    .find_map(|(id, window)| (window.surface == surface).then_some(*id))
            })
            .collect()
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
                output: window.output,
                rect: self
                    .scene
                    .get(window.surface)
                    .map(|surface| surface.rect)
                    .unwrap_or_default(),
                floating: window.floating,
                maximized: window.maximized,
                fullscreen: window.fullscreen,
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
        let outputs = self.outputs.clone();
        let mut visible_focus = false;

        for output in &outputs {
            let workspace = self.output_workspace(output.id);
            let ids = self.workspaces[workspace].windows.clone();

            let tiled: Vec<(WindowId, SurfaceId)> = ids
                .iter()
                .filter_map(|id| {
                    let window = self.windows.get(id)?;
                    (window.output == output.id
                        && !window.floating
                        && !window.maximized
                        && !window.fullscreen)
                        .then_some((*id, window.surface))
                })
                .collect();
            let rects = tile(output.rect, tiled.len(), self.layout);
            for ((_, surface), rect) in tiled.iter().zip(rects) {
                self.scene.move_to(*surface, rect);
            }

            let floating: Vec<(SurfaceId, Option<Rect>, Size, bool)> = ids
                .iter()
                .filter_map(|id| {
                    let window = self.windows.get(id)?;
                    (window.output == output.id
                        && (window.floating || window.maximized || window.fullscreen))
                        .then_some((
                            window.surface,
                            window.floating_rect,
                            window.spec.preferred_size,
                            window.maximized || window.fullscreen,
                        ))
                })
                .collect();
            for (index, (surface, explicit, size, maximized)) in floating.into_iter().enumerate() {
                let rect = if maximized {
                    output.rect
                } else if let Some(rect) = explicit {
                    self.clamp_rect_to(rect, output.rect)
                } else {
                    self.floating_rect(index, size, output.rect)
                };
                self.scene.move_to(surface, rect);
            }

            if let Some(focus) = self.focus {
                if self.windows.get(&focus).is_some_and(|window| {
                    window.output == output.id && window.workspace == workspace
                }) {
                    visible_focus = true;
                }
            }
        }

        // Per-output workspaces: a window is visible only on its own output,
        // and only while that output shows the window's workspace.
        for (id, window) in &self.windows {
            let visible = self.outputs.iter().any(|output| output.id == window.output)
                && window.workspace == self.output_workspace(window.output);
            self.scene.set_visible(window.surface, visible);
            if visible && self.focus == Some(*id) {
                visible_focus = true;
            }
        }

        if let Some(focus) = self.focus {
            if let Some(window) = self.windows.get(&focus) {
                if visible_focus {
                    self.scene.raise(window.surface);
                }
            }
        }
    }

    fn floating_rect(&self, index: usize, size: Size, output_bounds: Rect) -> Rect {
        let bounds = output_bounds.inset(self.layout.outer_gap as i32);
        if bounds.is_empty() {
            return Rect::new(bounds.x, bounds.y, 0, 0);
        }
        let width = size.width.min(bounds.width).max(1);
        let height = size.height.min(bounds.height).max(1);
        let offset = (index as i32) * 32;
        let x = bounds.x + ((bounds.width - width) / 2) as i32 + offset;
        let y = bounds.y + ((bounds.height - height) / 2) as i32 + offset;
        self.clamp_rect_to(Rect::new(x, y, width, height), output_bounds)
    }

    fn clamp_rect_to(&self, rect: Rect, bounds: Rect) -> Rect {
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
        let bounds = self.output_bounds(self.primary_output().map(|o| o.id).unwrap_or(1));
        Size::new(
            size.width.clamp(1, bounds.width.max(1)),
            size.height.clamp(1, bounds.height.max(1)),
        )
    }

    /// Bounds of the primary output used for shell chrome (palette/indicators).
    fn ui_bounds(&self) -> Rect {
        self.primary_output()
            .map(|output| output.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport))
    }

    /// Bounds of one output in logical coordinates.
    pub fn output_bounds(&self, output: OutputId) -> Rect {
        self.outputs
            .iter()
            .find(|candidate| candidate.id == output)
            .map(|candidate| candidate.rect)
            .unwrap_or_else(|| Rect::from_size(self.viewport))
    }

    /// Active workspace shown on an output.
    pub fn output_workspace(&self, output: OutputId) -> usize {
        self.output_workspaces
            .get(&output)
            .copied()
            .unwrap_or(0)
            .min(self.workspaces.len().saturating_sub(1))
    }

    /// Assign a workspace to an output. This is per-output: the primary
    /// output's assignment also drives `active_workspace`.
    pub fn set_output_workspace(&mut self, output: OutputId, workspace: usize) -> bool {
        if workspace >= self.workspaces.len()
            || !self.outputs.iter().any(|candidate| candidate.id == output)
        {
            return false;
        }
        if self.output_workspace(output) == workspace {
            return false;
        }
        self.output_workspaces.insert(output, workspace);
        if self
            .primary_output()
            .is_some_and(|primary| primary.id == output)
        {
            self.active = workspace;
        }
        self.focus = self.last_window_on_workspace(output, workspace);
        self.relayout();
        true
    }

    /// Move a window to another output, keeping its workspace assignment.
    pub fn set_window_output(&mut self, id: WindowId, output: OutputId) -> bool {
        if !self.outputs.iter().any(|candidate| candidate.id == output) {
            return false;
        }
        let Some(window) = self.windows.get(&id) else {
            return false;
        };
        if window.output == output {
            return false;
        }
        if let Some(window) = self.windows.get_mut(&id) {
            window.output = output;
        }
        self.relayout();
        true
    }

    /// Last window on an output's workspace, used to pick focus before the
    /// scene has been relaid out.
    fn last_window_on_workspace(&self, output: OutputId, workspace: usize) -> Option<WindowId> {
        self.workspaces
            .get(workspace)?
            .windows
            .iter()
            .rev()
            .find(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| window.output == output)
            })
            .copied()
    }

    /// Visible window ids on an output, topmost first.
    pub fn visible_windows_on(&self, output: OutputId) -> Vec<WindowId> {
        let workspace = self.output_workspace(output);
        self.window_stack()
            .into_iter()
            .filter(|id| {
                self.windows
                    .get(id)
                    .is_some_and(|window| window.output == output && window.workspace == workspace)
            })
            .collect()
    }
}

fn palette_score(query: &str, title: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    let title = title.to_lowercase();
    if title == query {
        Some(0)
    } else if title.starts_with(query) {
        Some(1)
    } else if title.contains(query) {
        Some(2)
    } else {
        None
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
    fn moving_a_window_promotes_it_and_keeps_its_position() {
        let mut shell = shell();
        let first = shell.add_window(WindowSpec {
            title: "first".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: true,
        });
        assert!(shell.move_floating(first, Point::new(120, 80)));

        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .expect("first window");
        assert!(info.floating);
        assert_eq!(info.rect.x, 120);
        assert_eq!(info.rect.y, 80);

        // Tiling another window must not recenter the moved one.
        window(&mut shell, "second");
        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        assert_eq!((info.rect.x, info.rect.y), (120, 80));

        shell.add_workspace("other");
        assert!(shell.switch_workspace(1));
        assert!(shell.switch_workspace(0));
        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        assert_eq!((info.rect.x, info.rect.y), (120, 80));
    }

    #[test]
    fn beginning_a_floating_move_uses_the_preferred_size() {
        let mut shell = shell();
        let id = shell.add_window(WindowSpec {
            title: "draggable".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: false,
        });
        let rect = shell
            .begin_floating_move(id, Point::new(400, 300))
            .expect("drag rect");
        assert_eq!(rect.size(), Size::new(320, 200));
        assert!(rect.contains(Point::new(400, 300)));
        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .unwrap();
        assert!(info.floating);
        assert_eq!(info.rect, rect);
    }

    #[test]
    fn snapping_places_windows_in_halves() {
        let mut shell = shell();
        let id = shell.add_window(WindowSpec {
            title: "snappy".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: true,
        });

        assert!(shell.snap_window(id, SnapZone::LeftHalf));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!(info.rect, Rect::new(0, 0, 400, 600));
        assert!(info.floating);
        assert!(!info.maximized);

        assert!(shell.snap_window(id, SnapZone::RightHalf));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!(info.rect, Rect::new(400, 0, 400, 600));

        assert!(shell.snap_window(id, SnapZone::Maximize));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!(info.rect, Rect::from_size(shell.viewport()));
        assert!(info.maximized);

        assert!(!shell.snap_window(id, SnapZone::LeftHalf));
        assert!(!shell.snap_window(999, SnapZone::LeftHalf));
    }

    #[test]
    fn floating_resize_enforces_minimum_and_bounds() {
        let mut shell = shell();
        let id = shell.add_window(WindowSpec {
            title: "resizable".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: true,
        });
        assert!(shell.resize_floating(id, Rect::new(100, 100, 500, 400)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!(info.rect, Rect::new(100, 100, 500, 400));

        assert!(shell.resize_floating(id, Rect::new(100, 100, 10, 10)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!(info.rect.width, MIN_FLOATING_WIDTH);
        assert_eq!(info.rect.height, MIN_FLOATING_HEIGHT);

        assert!(shell.resize_floating(id, Rect::new(700, 500, 500, 400)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert!(info.rect.right() <= shell.viewport().width as i64);
        assert!(info.rect.bottom() <= shell.viewport().height as i64);
    }

    #[test]
    fn beginning_a_floating_resize_promotes_a_tile() {
        let mut shell = shell();
        let id = window(&mut shell, "tile");
        let before = shell
            .windows()
            .into_iter()
            .find(|w| w.id == id)
            .unwrap()
            .rect;
        let rect = shell.begin_floating_resize(id).expect("resize rect");
        assert_eq!(rect, before);
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert!(info.floating);
        assert!(!info.maximized);
    }

    #[test]
    fn floating_move_snaps_to_output_edges() {
        let mut shell = shell();
        let id = shell.add_window(WindowSpec {
            title: "snap".into(),
            app_id: "test".into(),
            preferred_size: Size::new(320, 200),
            floating: true,
        });
        assert!(shell.move_floating(id, Point::new(5, 7)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!((info.rect.x, info.rect.y), (0, 0));

        assert!(shell.move_floating(id, Point::new(950, 550)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!((info.rect.x, info.rect.y), (480, 400));

        assert!(shell.move_floating(id, Point::new(200, 150)));
        let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!((info.rect.x, info.rect.y), (200, 150));
    }

    #[test]
    fn fullscreen_covers_the_viewport_and_restores_the_layout() {
        let mut shell = shell();
        let first = window(&mut shell, "first");
        let second = window(&mut shell, "second");
        let before = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap()
            .rect;
        let before_second = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap()
            .rect;
        assert!(shell.toggle_fullscreen(first));
        assert!(shell.is_fullscreen(first));
        assert_eq!(shell.fullscreen_window(), Some(first));
        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        assert_eq!(info.rect, Rect::from_size(shell.viewport()));
        assert!(!shell.move_floating(first, Point::new(10, 10)));

        assert!(shell.toggle_fullscreen(first));
        assert!(!shell.is_fullscreen(first));
        assert_eq!(shell.fullscreen_window(), None);
        let info = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        assert_eq!(info.rect, before);
        let other = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap();
        assert_eq!(other.rect, before_second);
        assert!(!shell.toggle_fullscreen(999));
    }

    #[test]
    fn maximize_covers_the_viewport_and_restores_the_layout() {
        let mut shell = shell();
        let first = window(&mut shell, "first");
        let second = window(&mut shell, "second");
        let before_first = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap()
            .rect;
        let before_second = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap()
            .rect;

        assert!(shell.toggle_maximized(first));
        assert!(shell.is_maximized(first));
        let maximized = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        assert_eq!(maximized.rect, Rect::from_size(shell.viewport()));
        let remaining = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap();
        assert_eq!(remaining.rect, Rect::from_size(shell.viewport()).inset(8));
        assert!(!shell.move_floating(first, Point::new(10, 10)));

        assert!(shell.toggle_maximized(first));
        assert!(!shell.is_maximized(first));
        let restored_first = shell
            .windows()
            .into_iter()
            .find(|window| window.id == first)
            .unwrap();
        let restored_second = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap();
        assert_eq!(restored_first.rect, before_first);
        assert_eq!(restored_second.rect, before_second);
        assert!(!shell.toggle_maximized(999));
    }

    #[test]
    fn new_windows_use_their_outputs_workspace() {
        let mut shell = shell();
        shell.add_workspace("web");
        assert!(shell.set_outputs(vec![
            OutputInfo {
                id: 1,
                name: "left".into(),
                rect: Rect::new(0, 0, 800, 600),
                scale: 1.0,
                primary: true,
            },
            OutputInfo {
                id: 2,
                name: "right".into(),
                rect: Rect::new(800, 0, 1024, 768),
                scale: 1.0,
                primary: false,
            },
        ]));
        assert!(shell.set_output_workspace(2, 1));
        let id = shell.add_window_on_output(WindowSpec::new("two", "app"), 2);
        let window = shell
            .windows()
            .into_iter()
            .find(|window| window.id == id)
            .unwrap();
        assert_eq!(window.output, 2);
        assert_eq!(window.workspace, 1);
        assert!(shell.visible_windows_on(2).contains(&id));
        assert!(!shell.visible_windows_on(1).contains(&id));
        assert!(window.rect.x >= 800);
    }

    #[test]
    fn multiple_outputs_extend_the_desktop_and_hotplug_reflows() {
        let mut shell = shell();
        let first = window(&mut shell, "first");
        let second = window(&mut shell, "second");
        assert_eq!(shell.outputs().len(), 1);
        assert_eq!(
            shell.primary_output().unwrap().rect,
            Rect::new(0, 0, 800, 600)
        );

        assert!(shell.set_outputs(vec![
            OutputInfo {
                id: 1,
                name: "left".into(),
                rect: Rect::new(0, 0, 800, 600),
                scale: 1.0,
                primary: true,
            },
            OutputInfo {
                id: 2,
                name: "right".into(),
                rect: Rect::new(800, 0, 1024, 768),
                scale: 1.0,
                primary: false,
            },
        ]));
        assert_eq!(shell.viewport(), Size::new(1824, 768));
        assert_eq!(shell.output_at(Point::new(1700, 400)), Some(2));
        assert_eq!(shell.output_at(Point::new(1900, 400)), None);

        // Existing windows stay on the primary output and re-tile inside it.
        let windows = shell.windows();
        assert!(windows
            .iter()
            .filter(|window| window.id == first || window.id == second)
            .all(|window| window.output == 1 && window.rect.right() <= 800));
        assert_eq!(
            windows
                .iter()
                .find(|window| window.id == first)
                .unwrap()
                .rect
                .height,
            584
        );

        // A window placed on the second output tiles inside its own bounds.
        let third = window(&mut shell, "third");
        assert!(shell.set_window_output(third, 2));
        let third_rect = shell
            .windows()
            .into_iter()
            .find(|window| window.id == third)
            .unwrap()
            .rect;
        assert!(third_rect.x >= 800);
        assert!(third_rect.right() <= 1824);

        // Per-output workspaces: output 2 can show a different workspace.
        shell.add_workspace("web");
        assert!(shell.set_output_workspace(2, 1));
        assert_eq!(shell.output_workspace(1), 0);
        assert_eq!(shell.output_workspace(2), 1);
        assert!(!shell.visible_windows_on(2).contains(&third));
        assert!(shell.set_output_workspace(2, 0));
        assert!(shell.visible_windows_on(2).contains(&third));

        assert!(shell.set_outputs(vec![OutputInfo {
            id: 1,
            name: "only".into(),
            rect: Rect::new(0, 0, 1280, 720),
            scale: 1.0,
            primary: true,
        }]));
        assert_eq!(shell.viewport(), Size::new(1280, 720));
        assert_eq!(shell.outputs()[0].name, "only");
        let second_rect = shell
            .windows()
            .into_iter()
            .find(|window| window.id == second)
            .unwrap()
            .rect;
        assert_eq!(second_rect.bottom(), 712);
        // Orphaned output-2 windows move back to the surviving output.
        assert_eq!(
            shell
                .windows()
                .iter()
                .find(|w| w.id == third)
                .unwrap()
                .output,
            1
        );
        assert!(!shell.set_outputs(Vec::new()));
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
    fn palette_filters_workspaces_and_close() {
        let mut shell = shell();
        shell.add_workspace("code");
        shell.add_workspace("web");

        let titles: Vec<String> = shell
            .palette_entries()
            .into_iter()
            .map(|entry| entry.title)
            .collect();
        assert_eq!(
            titles,
            ["main", "code", "web", "New terminal", "Close palette"]
        );

        assert!(shell.palette_push_char('c'));
        let titles: Vec<String> = shell
            .palette_entries()
            .into_iter()
            .map(|entry| entry.title)
            .collect();
        assert_eq!(titles, ["code", "Close palette"]);

        assert!(shell.palette_push_char('o'));
        let titles: Vec<String> = shell
            .palette_entries()
            .into_iter()
            .map(|entry| entry.title)
            .collect();
        assert_eq!(titles, ["code"]);

        assert!(shell.palette_push_char('d'));
        assert_eq!(shell.palette_entries().len(), 1);

        assert!(shell.palette_push_char('z'));
        assert!(shell.palette_entries().is_empty());
        assert!(shell.palette_activate().is_none());

        assert!(shell.palette_backspace());
        assert_eq!(shell.palette_entries().len(), 1);
    }

    #[test]
    fn window_stack_is_topmost_first() {
        let mut shell = shell();
        let first = window(&mut shell, "first");
        let second = window(&mut shell, "second");
        assert_eq!(shell.window_stack(), vec![second, first]);
        assert!(shell.focus_window(first));
        assert_eq!(shell.window_stack(), vec![first, second]);
    }

    #[test]
    fn window_titles_update_and_reach_the_palette() {
        let mut shell = shell();
        let id = window(&mut shell, "initial");
        assert!(shell.set_window_title(id, "Terminal"));
        assert_eq!(
            shell
                .windows()
                .into_iter()
                .find(|w| w.id == id)
                .unwrap()
                .title,
            "Terminal"
        );
        assert!(shell
            .palette_entries()
            .iter()
            .any(|entry| entry.title == "Terminal"));
        assert!(!shell.set_window_title(999, "missing"));
    }

    #[test]
    fn prefiltered_index_results_skip_name_matching() {
        let mut shell = shell();
        shell.set_indexed_files(vec![PaletteFile {
            name: "notes-alpha.txt".into(),
            detail: "indexed".into(),
            path: "/home/user/notes-alpha.txt".into(),
        }]);
        for character in "magic".chars() {
            assert!(shell.palette_push_char(character));
        }
        let entries = shell.palette_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "notes-alpha.txt");
        assert_eq!(entries[0].action, PaletteAction::OpenFile(0));
    }

    #[test]
    fn palette_searches_files_only_with_a_query() {
        let mut shell = shell();
        shell.set_files(vec![
            PaletteFile {
                name: "notes-alpha.txt".into(),
                detail: "notes".into(),
                path: "/home/user/notes-alpha.txt".into(),
            },
            PaletteFile {
                name: "beta.md".into(),
                detail: String::new(),
                path: "/home/user/beta.md".into(),
            },
        ]);

        assert!(shell
            .palette_entries()
            .iter()
            .all(|entry| entry.title != "notes-alpha.txt"));

        for character in "alpha".chars() {
            assert!(shell.palette_push_char(character));
        }
        let entries = shell.palette_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "notes-alpha.txt");
        assert_eq!(entries[0].action, PaletteAction::OpenFile(0));
        let file = shell.file(0).expect("file");
        assert_eq!(file.path, "/home/user/notes-alpha.txt");
        assert!(shell.file(2).is_none());
    }

    #[test]
    fn palette_lists_application_launchers() {
        let mut shell = shell();
        shell.set_launchers(vec![
            PaletteLauncher {
                name: "Example Editor".into(),
                detail: "application".into(),
                icon: Some("/tmp/editor.png".into()),
                program: "example-editor".into(),
                args: vec!["--new".into()],
            },
            PaletteLauncher {
                name: "Browser".into(),
                detail: "application".into(),
                icon: None,
                program: "browser".into(),
                args: Vec::new(),
            },
        ]);

        let titles: Vec<String> = shell
            .palette_entries()
            .into_iter()
            .map(|entry| entry.title)
            .collect();
        assert_eq!(
            titles,
            [
                "main",
                "Example Editor",
                "Browser",
                "New terminal",
                "Close palette"
            ]
        );

        for character in "edit".chars() {
            assert!(shell.palette_push_char(character));
        }
        let entries = shell.palette_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, PaletteAction::LaunchApp(0));
        let launcher = shell.launcher(0).expect("launcher");
        assert_eq!(launcher.icon.as_deref(), Some("/tmp/editor.png"));
        assert_eq!(launcher.program, "example-editor");
        assert_eq!(launcher.args, ["--new"]);
        assert!(shell.launcher(2).is_none());
    }

    #[test]
    fn palette_exposes_the_terminal_launcher() {
        let mut shell = shell();
        assert!(shell.palette_push_char('n'));
        assert!(shell.palette_push_char('e'));
        assert!(shell.palette_push_char('w'));
        let entries = shell.palette_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "New terminal");
        assert_eq!(entries[0].action, PaletteAction::SpawnTerminal);
    }

    #[test]
    fn palette_lists_and_filters_windows() {
        let mut shell = shell();
        shell.add_workspace("code");
        let terminal = window(&mut shell, "Terminal");
        let editor = window(&mut shell, "Editor");

        let entries = shell.palette_entries();
        assert!(entries.iter().any(|entry| entry.title == "Terminal"));
        assert!(entries.iter().any(|entry| entry.title == "Editor"));

        assert!(shell.palette_push_char('e'));
        assert!(shell.palette_push_char('d'));
        assert!(shell.palette_push_char('i'));
        assert!(shell.palette_push_char('t'));
        let entries = shell.palette_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Editor");
        assert_eq!(entries[0].action, PaletteAction::FocusWindow(editor));
        assert_ne!(entries[0].action, PaletteAction::FocusWindow(terminal));
    }

    #[test]
    fn palette_rows_slide_with_selection() {
        let mut shell = shell();
        for title in ["one", "two", "three", "four", "five"] {
            window(&mut shell, title);
        }
        shell.open_overlay();

        assert_eq!(shell.palette_entries().len(), 8);
        assert_eq!(shell.palette_visible_start(), 0);
        let first = shell.palette_row_rect(0).expect("row 0");
        assert_eq!(first.x, 76);
        assert_eq!(first.y, 90 + 96);
        assert_eq!(
            shell.palette_row_at(Point::new(first.x + 2, first.y + 2)),
            Some(0)
        );
        assert_eq!(shell.palette_row_at(Point::new(20, first.y + 2)), None);

        for _ in 0..4 {
            shell.palette_move_selection(1);
        }
        assert_eq!(shell.palette_selected(), 4);
        assert_eq!(shell.palette_visible_start(), 1);
        assert_eq!(
            shell.palette_row_at(Point::new(first.x + 2, first.y + 2)),
            Some(1)
        );
        assert!(shell.palette_select(7));
        assert_eq!(shell.palette_selected(), 7);
        assert!(!shell.palette_select(8));
    }

    #[test]
    fn palette_selection_wraps_and_activates() {
        let mut shell = shell();
        shell.add_workspace("code");
        shell.open_overlay();

        assert_eq!(shell.palette_selected(), 0);
        assert!(shell.palette_move_selection(-1));
        assert_eq!(shell.palette_selected(), 3);
        assert_eq!(shell.palette_activate(), Some(PaletteAction::Close));

        assert!(shell.palette_move_selection(1));
        assert_eq!(shell.palette_selected(), 0);
        assert_eq!(
            shell.palette_activate(),
            Some(PaletteAction::SwitchWorkspace(0))
        );

        assert!(shell.palette_push_char('c'));
        assert_eq!(shell.palette_selected(), 0);
        assert_eq!(
            shell.palette_activate(),
            Some(PaletteAction::SwitchWorkspace(1))
        );
    }

    #[test]
    fn palette_query_resets_when_reopened() {
        let mut shell = shell();
        assert!(shell.toggle_overlay());
        assert!(shell.palette_push_char('w'));
        assert_eq!(shell.palette_query(), "w");
        assert!(!shell.toggle_overlay());
        assert_eq!(shell.palette_query(), "");
        assert!(shell.toggle_overlay());
        assert_eq!(shell.palette_query(), "");
        assert_eq!(shell.overlay_rect(), Some(Rect::new(60, 90, 680, 420)));
    }

    #[test]
    fn master_ratio_updates_the_divider() {
        let mut shell = shell();
        window(&mut shell, "one");
        window(&mut shell, "two");

        assert!((shell.master_ratio() - 0.5).abs() < f32::EPSILON);
        let before = shell.master_gap_rect().expect("two tiled windows");
        assert!(before.width > 0);

        assert!(shell.set_master_ratio(0.7));
        assert!((shell.master_ratio() - 0.7).abs() < f32::EPSILON);
        let after = shell.master_gap_rect().expect("two tiled windows");
        assert!(after.x > before.x, "divider moved right");

        assert!(shell.set_master_ratio(5.0));
        assert!((shell.master_ratio() - 0.8).abs() < f32::EPSILON);
        assert!(shell.set_master_ratio(-1.0));
        assert!((shell.master_ratio() - 0.2).abs() < f32::EPSILON);
        assert!(!shell.set_master_ratio(0.2));
        assert!(!shell.set_master_ratio(f32::NAN));
    }

    #[test]
    fn master_divider_requires_two_tiled_windows() {
        let mut shell = shell();
        assert_eq!(shell.master_gap_rect(), None);

        window(&mut shell, "one");
        assert_eq!(shell.master_gap_rect(), None);

        let floating = window(&mut shell, "two");
        shell.set_floating(floating, true);
        assert_eq!(shell.master_gap_rect(), None);

        window(&mut shell, "three");
        let gap = shell.master_gap_rect().expect("two tiled windows");
        assert!(gap.width > 0);
        assert!(gap.height > 0);
    }

    #[test]
    fn workspace_picker_hit_tests_bars_and_labels() {
        let mut shell = shell();
        shell.add_workspace("web");

        assert_eq!(
            shell.workspace_indicator_rect(0),
            Some(Rect::new(10, 10, 66, 26))
        );
        assert_eq!(shell.workspace_at(Point::new(12, 12)), Some(0));
        assert_eq!(shell.workspace_at(Point::new(20, 30)), Some(0));

        let second = shell.workspace_indicator_rect(1).expect("second indicator");
        assert_eq!(
            shell.workspace_at(Point::new(second.x + 2, second.y + 2)),
            Some(1)
        );
        assert_eq!(shell.workspace_at(Point::new(500, 500)), None);
        assert_eq!(shell.workspace_indicator_rect(2), None);
    }

    #[test]
    fn overlay_rect_tracks_the_overlay_surface() {
        let mut shell = shell();
        assert_eq!(shell.overlay_rect(), None);
        let rect = Rect::new(100, 80, 400, 300);
        shell.set_overlay(Some(rect));
        assert_eq!(shell.overlay_rect(), Some(rect));
        shell.set_overlay(None);
        assert_eq!(shell.overlay_rect(), None);
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
