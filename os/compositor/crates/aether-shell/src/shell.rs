//! [`Shell`] state, private window records and construction.

use std::collections::HashMap;

use aether_geometry::{Rect, Size};
use aether_layout::LayoutConfig;
use aether_pacing::FramePacer;
use aether_scene::{Scene, SurfaceId};

use crate::audio::AudioLayer;
use crate::chat::ChatState;
use crate::explorer::ExplorerState;
use crate::output::OutputInfo;
use crate::palette::{PaletteFile, PaletteLauncher};
use crate::remote::RemoteLayer;
use crate::theme::{ThemeLayer, ThemeSettings};
use crate::window::WindowSpec;
use crate::{OutputId, WindowId};

mod audio;
mod chat;
mod explorer;
mod floating;
mod focus;
mod frame;
mod layout;
mod outputs;
mod overlay;
mod palette;
mod remote;
mod themes;
mod windows;
mod workspaces;

#[cfg(test)]
mod tests;

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

#[derive(Clone, Debug, Default)]
struct PaletteState {
    query: String,
    selected: usize,
}

pub struct Shell {
    scene: Scene,
    layout: LayoutConfig,
    pacer: FramePacer,
    workspaces: Vec<Workspace>,
    windows: HashMap<WindowId, Window>,
    active: usize,
    focus: Option<WindowId>,
    focus_history: Vec<WindowId>,
    focus_cycle: Option<usize>,
    overlay: Option<SurfaceId>,
    outputs: Vec<OutputInfo>,
    output_workspaces: HashMap<OutputId, usize>,
    overlay_output: Option<OutputId>,
    palette: PaletteState,
    explorer: Option<ExplorerState>,
    chat: Option<ChatState>,
    audio: Option<AudioLayer>,
    remote: Option<RemoteLayer>,
    theme: ThemeSettings,
    themes: Option<ThemeLayer>,
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
            focus_history: Vec::new(),
            focus_cycle: None,
            overlay: None,
            outputs: vec![OutputInfo {
                id: 1,
                name: "output-1".to_string(),
                rect: Rect::from_size(viewport),
                scale: 1.0,
                refresh_hz: 60,
                physical_size_mm: (0, 0),
                primary: true,
            }],
            output_workspaces: HashMap::from([(1, 0)]),
            overlay_output: Some(1),
            palette: PaletteState::default(),
            explorer: None,
            chat: None,
            audio: None,
            remote: None,
            theme: ThemeSettings::default(),
            themes: None,
            launchers: Vec::new(),
            files: Vec::new(),
            files_prefiltered: false,
            viewport,
            next_id: 1,
        }
    }
}
