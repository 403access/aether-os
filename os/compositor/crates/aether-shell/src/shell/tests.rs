use crate::*;
use aether_geometry::{Point, Rect, Size};
use aether_layout::LayoutConfig;
use aether_scene::Layer;

fn shell() -> Shell {
    Shell::new(Size::new(800, 600), LayoutConfig::default(), 120)
}

fn window(shell: &mut Shell, title: &str) -> WindowId {
    shell.add_window(WindowSpec::new(title, "test.app"))
}

mod chat;
mod explorer;
mod focus;
mod frame;
mod outputs;
mod palette;
mod parity;
mod remote;
mod themes;
mod windows;
