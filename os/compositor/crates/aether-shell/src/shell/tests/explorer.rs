use super::*;

fn shell_with_files(count: usize) -> Shell {
    let mut shell = Shell::new(Size::new(1280, 800), LayoutConfig::default(), 120);
    shell.set_files(
        (0..count)
            .map(|index| PaletteFile {
                name: format!("file-{index:05}.txt"),
                detail: "file".to_string(),
                path: format!("/tmp/file-{index:05}.txt"),
            })
            .collect(),
    );
    shell.open_explorer().unwrap();
    shell
}

#[test]
fn virtualises_ten_thousand_rows() {
    let mut shell = shell_with_files(10_000);
    let visible = shell.explorer_visible_rows();
    // The floating explorer window is 610 px tall at 1280 x 800, which leaves
    // eleven 30 px rows below the app bar, pane header and column header.
    assert!(visible >= 11);
    assert_eq!(shell.visible_explorer_rows().len(), visible);
    assert_eq!(shell.explorer_match_count(), 10_000);
    let last = visible as isize - 1;
    assert!(shell.explorer_move(last));
    assert_eq!(shell.explorer_selected(), last as usize);
    assert!(shell.explorer_move(1));
    assert_eq!(shell.explorer_scroll(), 1);
    assert!(shell.explorer_page(1));
    assert!(shell.explorer_scroll() > 1);
    assert!(shell.explorer_page(-1));
    assert!(shell.explorer_move(-1000));
    assert!(shell.explorer_selected() < 10_000);
    assert_eq!(shell.visible_explorer_rows().len(), visible);
}

#[test]
fn filters_without_rebuilding_the_visible_list() {
    let mut shell = shell_with_files(10_000);
    for character in "file-0000".chars() {
        assert!(shell.explorer_push_char(character));
    }
    assert!(shell.explorer_match_count() >= 10);
    assert!(shell.visible_explorer_rows().len() <= shell.explorer_visible_rows());
    assert!(shell.explorer_selected_file().is_some());
    let before = shell.explorer_match_count();
    assert!(shell.explorer_backspace());
    assert!(shell.explorer_match_count() >= before);
}

#[test]
#[ignore = "perf: run with --release --ignored and record p95"]
fn explorer_scroll_10k_p95_bench() {
    let mut shell = shell_with_files(10_000);
    let mut samples = Vec::new();
    for index in 0..200 {
        let started = std::time::Instant::now();
        shell.explorer_page(if index % 2 == 0 { 1 } else { -1 });
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = samples[(samples.len() - 1) * 95 / 100];
    println!("explorer_10k_p95_ms={p95:.3}");
    assert!(p95 < 1.0, "p95 {p95}ms exceeds 1ms");
}

#[test]
fn row_hit_testing_and_activation() {
    let mut shell = shell_with_files(100);
    let rect = shell.explorer_pane_rect();
    let point = Point::new(
        rect.x + 20,
        shell.explorer_rows_top() + design::EXPLORER_ROW_HEIGHT as i32 * 3 + 4,
    );
    assert_eq!(shell.explorer_select_at(point), Some(3));
    let file = shell.explorer_selected_file().unwrap();
    assert_eq!(file.name, "file-00003.txt");
}
