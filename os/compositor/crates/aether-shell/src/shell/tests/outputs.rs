use super::*;

#[test]
fn chrome_is_positioned_on_its_own_output() {
    let mut shell = shell();
    assert!(shell.set_outputs(vec![
        OutputInfo {
            id: 1,
            name: "left".into(),
            rect: Rect::new(0, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
    ]));
    assert!(shell.open_overlay_on(2).is_some());
    let overlay = shell.overlay_rect().unwrap();
    assert_eq!(shell.overlay_output(), Some(2));
    assert!(overlay.x >= 800);
    assert!(overlay.right() <= 1824);

    let chip = shell.workspace_indicator_rect_for(2, 0).unwrap();
    assert_eq!(chip.x, 810);
    assert_eq!(chip.y, 10);
    assert_eq!(
        shell.workspace_at_global(Point::new(chip.x + 2, chip.y + 2)),
        Some(0)
    );
    assert_eq!(
        shell.workspace_at_in_output(1, Point::new(chip.x + 2, chip.y + 2)),
        None
    );

    shell.add_workspace("web");
    assert!(shell.set_output_workspace(2, 1));
    shell.close_overlay();
    assert_eq!(shell.overlay_rect(), None);
    assert_eq!(shell.overlay_output(), None);
}

#[test]
fn switching_a_secondary_output_leaves_the_primary_alone() {
    let mut shell = shell();
    shell.add_workspace("web");
    assert!(shell.set_outputs(vec![
        OutputInfo {
            id: 1,
            name: "left".into(),
            rect: Rect::new(0, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
    ]));
    assert!(shell.switch_workspace_on(2, 1));
    assert_eq!(shell.active_workspace(), 0);
    assert_eq!(shell.output_workspace(1), 0);
    assert_eq!(shell.output_workspace(2), 1);
    assert!(shell.switch_workspace_on(1, 1));
    assert_eq!(shell.active_workspace(), 1);
}

#[test]
fn removing_the_overlay_output_recentres_the_palette() {
    let mut shell = shell();
    assert!(shell.set_outputs(vec![
        OutputInfo {
            id: 1,
            name: "left".into(),
            rect: Rect::new(0, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
    ]));
    assert!(shell.open_overlay_on(2).is_some());
    assert!(shell.overlay_rect().unwrap().x >= 800);
    assert!(shell.set_outputs(vec![OutputInfo {
        id: 1,
        name: "left".into(),
        rect: Rect::new(0, 0, 1280, 720),
        scale: 1.0,
        refresh_hz: 60,
        physical_size_mm: (0, 0),
        primary: true,
    }]));
    assert_eq!(shell.overlay_output(), Some(1));
    let overlay = shell.overlay_rect().unwrap();
    assert!(overlay.x < 1280);
    assert!(overlay.right() <= 1280);
}

#[test]
fn secondary_output_tiles_equal_columns() {
    let mut shell = shell();
    assert!(shell.set_outputs(vec![
        OutputInfo {
            id: 1,
            name: "left".into(),
            rect: Rect::new(0, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
    ]));
    let a = shell.add_window_on_output(WindowSpec::new("a", "app"), 2);
    let b = shell.add_window_on_output(WindowSpec::new("b", "app"), 2);
    assert!(shell.windows().iter().any(|window| window.id == a));
    assert!(shell.windows().iter().any(|window| window.id == b));
    let rects: Vec<_> = shell
        .windows()
        .into_iter()
        .filter(|window| window.output == 2)
        .map(|window| window.rect)
        .collect();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].height, rects[1].height);
    assert!((rects[0].width as i32 - rects[1].width as i32).abs() <= 1);
    assert!(rects.iter().all(|rect| rect.x >= 800));
    assert!(rects.iter().all(|rect| rect.right() <= 1824));
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
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
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
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: true,
        },
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: Rect::new(800, 0, 1024, 768),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
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
        refresh_hz: 60,
        physical_size_mm: (0, 0),
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
