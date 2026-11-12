//! Generate preview frames for the shell on any machine (no GPU/Wayland).
//!
//!   cargo run -p aether-preview --bin aether-shell-preview
//!
//! Writes `artifacts/shell/frame.png` and `artifacts/shell/damage.png`.

use std::path::PathBuf;

use aether_geometry::{Rect, Size};
use aether_layout::LayoutConfig;
use aether_preview::render;
use aether_shell::{Shell, WindowSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut shell = Shell::new(
        Size::new(1920, 1200),
        LayoutConfig {
            master_ratio: 0.55,
            gap: 10,
            outer_gap: 16,
        },
        120,
    );

    let windows = [
        ("compositor.rs", "aether.shell", Size::new(1000, 800)),
        ("indexer.rs", "aether.index", Size::new(760, 600)),
        ("Chat · DeepSeek", "aether.chat", Size::new(600, 700)),
        ("Files", "aether.files", Size::new(700, 520)),
    ];
    let mut ids = Vec::new();
    for (title, app_id, size) in windows {
        let mut spec = WindowSpec::new(title, app_id);
        spec.preferred_size = size;
        ids.push(shell.add_window(spec));
    }
    shell.set_overlay(Some(Rect::new(620, 180, 680, 520)));

    let first = shell.frame(0).ok_or("first frame was not scheduled")?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../artifacts/shell");
    render(&first, false).save_png(&out.join("frame.png"))?;

    if let Some(id) = ids.first().copied() {
        shell.focus_window(id);
        shell.damage_window(id, Rect::new(40, 84, 520, 36));
    }
    let second = shell
        .frame(8_400_000)
        .ok_or("second frame was not scheduled")?;
    render(&second, true).save_png(&out.join("damage.png"))?;

    println!(
        "shell preview: {} windows, workspace {}, {} frame items, {} px damaged",
        shell.window_count(),
        shell.active_workspace() + 1,
        second.items.len(),
        second.damage.area()
    );
    println!(
        "wrote {}/frame.png and {}/damage.png",
        out.display(),
        out.display()
    );
    Ok(())
}
