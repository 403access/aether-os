//! Minimal app-facing composition: create surfaces, lay them out, damage a
//! region, and schedule a frame. No GPU or display server required.
//!
//!   cargo run -p aether-compositor --example hello_scene

use aether_compositor::prelude::*;

fn main() {
    let viewport = Rect::new(0, 0, 1920, 1080);
    let layout = LayoutConfig::default();
    let windows = tile(viewport, 2, layout);
    let mut scene = Scene::new(Size::new(viewport.width, viewport.height));

    let left = scene.add(Layer::WINDOW, windows[0]);
    let right = scene.add(Layer::WINDOW, windows[1]);
    let palette = scene.add(Layer::OVERLAY, Rect::new(620, 140, 680, 480));

    // First frame: the whole initial scene is new.
    scene.clear_damage();
    // A keystroke changes only the palette's input row.
    scene.add_damage(palette, Rect::new(640, 180, 400, 40));
    let (damage_rects, damage_area) = {
        let damage = scene.commit();
        (damage.len(), damage.area())
    };
    println!(
        "surfaces={} damage_rects={} damage_area={}",
        scene.len(),
        damage_rects,
        damage_area
    );
    println!("left={left:?} right={right:?} palette={palette:?}");

    let mut pacer = FramePacer::new(120);
    pacer.on_present(0);
    println!(
        "next_deadline={:?} present_now={}",
        pacer.deadline(),
        pacer.should_present(7_200_000)
    );
}
