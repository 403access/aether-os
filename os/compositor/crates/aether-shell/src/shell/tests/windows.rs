use super::*;

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
    let widths: Vec<i32> = windows
        .iter()
        .map(|window| window.rect.width as i32)
        .collect();
    assert!(widths.iter().max().unwrap() - widths.iter().min().unwrap() <= 2);
    let mut xs: Vec<i32> = windows.iter().map(|window| window.rect.x).collect();
    xs.sort_unstable();
    xs.dedup();
    assert_eq!(xs.len(), 4, "one equal column per window");
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
fn floating_windows_stay_below_the_top_gap() {
    let mut shell = Shell::new(
        Size::new(800, 600),
        LayoutConfig {
            gap: 6,
            outer_gap: 8,
            top_gap: 48,
        },
        120,
    );
    let id = shell.add_window(WindowSpec {
        title: "big".into(),
        app_id: "test".into(),
        preferred_size: Size::new(640, 500),
        floating: true,
    });
    let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
    assert!(
        info.rect.y >= 8 + 48,
        "floating window overlaps the top dock: {:?}",
        info.rect
    );
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
    assert_eq!(info.rect, Rect::new(8, 8, 392, 584));
    assert!(info.floating);
    assert!(!info.maximized);

    assert!(shell.snap_window(id, SnapZone::RightHalf));
    let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
    assert_eq!(info.rect, Rect::new(400, 8, 392, 584));

    assert!(shell.snap_window(id, SnapZone::Maximize));
    let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
    assert_eq!(
        info.rect,
        aether_layout::work_area(Rect::from_size(shell.viewport()), shell.layout())
    );
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
fn beginning_a_title_move_keeps_the_tile_rect() {
    let mut shell = shell();
    let id = window(&mut shell, "tile");
    let before = shell
        .windows()
        .into_iter()
        .find(|w| w.id == id)
        .unwrap()
        .rect;
    let rect = shell.begin_window_move(id).expect("move rect");
    assert_eq!(rect, before);
    let info = shell.windows().into_iter().find(|w| w.id == id).unwrap();
    assert_eq!(info.rect, before);
    assert!(info.floating);

    // A floating window keeps the exact pointer offset while it moves.
    let small = shell.add_window(WindowSpec {
        title: "small".into(),
        app_id: "test".into(),
        preferred_size: Size::new(320, 200),
        floating: true,
    });
    let start = shell
        .windows()
        .into_iter()
        .find(|w| w.id == small)
        .unwrap()
        .rect;
    assert!(shell.begin_window_move(small).is_some());
    assert!(shell.move_floating(small, Point::new(start.x + 40, start.y + 24)));
    let moved = shell.windows().into_iter().find(|w| w.id == small).unwrap();
    assert_eq!((moved.rect.x, moved.rect.y), (start.x + 40, start.y + 24));
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
fn maximize_covers_the_work_area_and_restores_the_layout() {
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
    assert_eq!(
        maximized.rect,
        aether_layout::work_area(Rect::from_size(shell.viewport()), shell.layout())
    );
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
fn window_stack_is_topmost_first() {
    let mut shell = shell();
    let first = window(&mut shell, "first");
    let second = window(&mut shell, "second");
    assert_eq!(shell.window_stack(), vec![second, first]);
    assert!(shell.focus_window(first));
    assert_eq!(shell.window_stack(), vec![first, second]);
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
