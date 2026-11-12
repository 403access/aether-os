//! End-to-end exercise of the pure core: layout, scene mutation, damage and
//! frame pacing, without a GPU or a display server.

use aether_compositor::{tile, FramePacer, Layer, LayoutConfig, Point, Rect, Scene, Size};

#[test]
fn a_session_of_window_management() {
    let viewport = Rect::new(0, 0, 1920, 1080);
    let config = LayoutConfig::default();
    let windows = tile(viewport, 3, config);
    assert_eq!(windows.len(), 3);

    let mut scene = Scene::new(Size::new(viewport.width, viewport.height));
    let ids: Vec<_> = windows
        .iter()
        .map(|rect| scene.add(Layer::WINDOW, *rect))
        .collect();
    let palette = scene.add(Layer::OVERLAY, Rect::new(320, 120, 680, 520));
    let status = scene.add(Layer::CHROME, Rect::new(0, 1040, 1920, 40));

    assert_eq!(scene.len(), 5);
    assert_eq!(scene.draw_order().last(), Some(&status));

    // The palette is above the windows and receives the click.
    assert_eq!(scene.hit_test(Point::new(400, 200)), Some(palette));

    // Type a character: only the input row is damaged, clipped to the palette.
    scene.clear_damage();
    assert!(scene.add_damage(palette, Rect::new(300, 100, 800, 48)));
    let damage = scene.commit();
    assert_eq!(damage.len(), 1);
    assert_eq!(damage.area(), 680 * 28); // clipped vertically to the palette header

    // Move a window: old and new rects are damaged.
    let first = ids[0];
    scene.clear_damage();
    assert!(scene.move_to(first, Rect::new(40, 40, 900, 700)));
    assert_eq!(scene.damage().len(), 1); // new rect is inside the old one

    // Hide the palette: its rect is damaged and it stops receiving input.
    scene.clear_damage();
    assert!(scene.set_visible(palette, false));
    assert_eq!(scene.hit_test(Point::new(400, 200)), Some(first));
    assert_eq!(scene.damage().area(), 680 * 520);

    // Pacing: 120 Hz, no queueing, late frames are superseded.
    let mut pacer = FramePacer::with_margin(120, 1_200_000);
    assert!(pacer.should_present(0));
    pacer.on_present(0);
    assert!(!pacer.should_present(7_000_000));
    assert!(pacer.should_present(7_133_333));
    assert_eq!(pacer.missed_deadlines(60_000_000), 7);
}

#[test]
fn tiling_invariants_hold_for_many_counts() {
    let viewport = Rect::new(-100, -50, 1280, 720);
    let config = LayoutConfig::default();

    for count in 0..=12 {
        let rects = tile(viewport, count, config);
        assert_eq!(rects.len(), count);
        for (index, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(index + 1) {
                assert!(a.intersect(*b).is_none());
            }
        }
    }
}
