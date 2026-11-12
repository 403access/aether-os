//! Demo binary for the scene core. It exercises the parts that do not need a
//! GPU and prints a short report; useful as a smoke test on the target machine.
//!
//!   cargo run --release --bin aether-scene-demo

use aether_scene::{tile, FramePacer, Layer, LayoutConfig, Point, Rect, Scene, Size};

fn main() {
    let viewport = Rect::new(0, 0, 2560, 1440);
    let config = LayoutConfig::default();
    let rects = tile(viewport, 4, config);

    println!("Aether scene model v{}", aether_scene::SCENE_MODEL_VERSION);
    println!("viewport {viewport:?}");
    println!("tiling (4 windows, gap {} px):", config.gap);
    for (index, rect) in rects.iter().enumerate() {
        println!(
            "  window {index}: x={} y={} w={} h={}",
            rect.x, rect.y, rect.width, rect.height
        );
    }

    let mut scene = Scene::new(Size::new(viewport.width, viewport.height));
    let ids: Vec<_> = rects
        .iter()
        .map(|rect| scene.add(Layer::WINDOW, *rect))
        .collect();
    let overlay = scene.add(Layer::OVERLAY, Rect::new(380, 120, 760, 560));
    let chrome = scene.add(Layer::CHROME, Rect::new(0, 1400, 2560, 40));

    println!("surfaces: {}", scene.len());
    println!("draw order: {:?}", scene.draw_order());
    println!(
        "hit test (400, 200) -> {:?}",
        scene.hit_test(Point::new(400, 200))
    );

    scene.clear_damage();
    scene.add_damage(overlay, Rect::new(400, 150, 420, 120));
    let damage = scene.commit();
    println!(
        "content damage: {} rect(s), {} px²",
        damage.len(),
        damage.area()
    );

    if let Some(first) = ids.first() {
        scene.move_to(*first, Rect::new(64, 64, 800, 600));
        println!(
            "after move: {} rect(s), {} px²",
            scene.damage().len(),
            scene.damage().area()
        );
    }

    let mut pacer = FramePacer::new(120);
    pacer.on_present(0);
    println!(
        "pacing: {} ns interval, present at 7.0 ms = {}, missed at 30 ms = {}",
        pacer.interval_ns(),
        pacer.should_present(7_000_000),
        pacer.missed_deadlines(30_000_000)
    );
    println!("chrome surface: {chrome:?}");
}
