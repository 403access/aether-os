use super::*;

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
