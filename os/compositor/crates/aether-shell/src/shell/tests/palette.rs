use super::*;

#[test]
fn palette_filters_workspaces_and_close() {
    let mut shell = shell();
    shell.add_workspace("code");
    shell.add_workspace("web");

    let titles: Vec<String> = shell
        .palette_entries()
        .into_iter()
        .map(|entry| entry.title)
        .collect();
    assert_eq!(
        titles,
        [
            "main",
            "code",
            "web",
            "New terminal",
            "Files",
            "Chat",
            "Audio",
            "Agents",
            "Themes",
            "Keyboard shortcuts",
            "Task manager",
            "Calendar",
            "Close palette"
        ]
    );

    assert!(shell.palette_push_char('c'));
    let titles: Vec<String> = shell
        .palette_entries()
        .into_iter()
        .map(|entry| entry.title)
        .collect();
    assert_eq!(
        titles,
        [
            "code",
            "Chat",
            "Calendar",
            "Close palette",
            "Keyboard shortcuts"
        ]
    );

    assert!(shell.palette_push_char('o'));
    let titles: Vec<String> = shell
        .palette_entries()
        .into_iter()
        .map(|entry| entry.title)
        .collect();
    // "co" is a prefix of neither, but is a subsequence of both titles.
    assert_eq!(titles, ["code", "Close palette"]);

    assert!(shell.palette_push_char('d'));
    assert_eq!(shell.palette_entries().len(), 1);

    assert!(shell.palette_push_char('z'));
    assert!(shell.palette_entries().is_empty());
    assert!(shell.palette_activate().is_none());

    assert!(shell.palette_backspace());
    assert_eq!(shell.palette_entries().len(), 1);
}

#[test]
fn window_titles_update_and_reach_the_palette() {
    let mut shell = shell();
    let id = window(&mut shell, "initial");
    assert!(shell.set_window_title(id, "Terminal"));
    assert_eq!(
        shell
            .windows()
            .into_iter()
            .find(|w| w.id == id)
            .unwrap()
            .title,
        "Terminal"
    );
    assert!(shell
        .palette_entries()
        .iter()
        .any(|entry| entry.title == "Terminal"));
    assert!(!shell.set_window_title(999, "missing"));
}

#[test]
fn prefiltered_index_results_skip_name_matching() {
    let mut shell = shell();
    shell.set_indexed_files(vec![PaletteFile {
        name: "notes-alpha.txt".into(),
        detail: "indexed".into(),
        path: "/home/user/notes-alpha.txt".into(),
    }]);
    for character in "magic".chars() {
        assert!(shell.palette_push_char(character));
    }
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "notes-alpha.txt");
    assert_eq!(entries[0].action, PaletteAction::OpenFile(0));
}

#[test]
fn palette_searches_files_only_with_a_query() {
    let mut shell = shell();
    shell.set_files(vec![
        PaletteFile {
            name: "notes-alpha.txt".into(),
            detail: "notes".into(),
            path: "/home/user/notes-alpha.txt".into(),
        },
        PaletteFile {
            name: "beta.md".into(),
            detail: String::new(),
            path: "/home/user/beta.md".into(),
        },
    ]);

    assert!(shell
        .palette_entries()
        .iter()
        .all(|entry| entry.title != "notes-alpha.txt"));

    for character in "alpha".chars() {
        assert!(shell.palette_push_char(character));
    }
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "notes-alpha.txt");
    assert_eq!(entries[0].action, PaletteAction::OpenFile(0));
    let file = shell.file(0).expect("file");
    assert_eq!(file.path, "/home/user/notes-alpha.txt");
    assert!(shell.file(2).is_none());
}

#[test]
fn palette_lists_application_launchers() {
    let mut shell = shell();
    shell.set_launchers(vec![
        PaletteLauncher {
            name: "Example Editor".into(),
            detail: "application".into(),
            icon: Some("/tmp/editor.png".into()),
            program: "example-editor".into(),
            args: vec!["--new".into()],
        },
        PaletteLauncher {
            name: "Browser".into(),
            detail: "application".into(),
            icon: None,
            program: "browser".into(),
            args: Vec::new(),
        },
    ]);

    let titles: Vec<String> = shell
        .palette_entries()
        .into_iter()
        .map(|entry| entry.title)
        .collect();
    assert_eq!(
        titles,
        [
            "main",
            "Example Editor",
            "Browser",
            "New terminal",
            "Files",
            "Chat",
            "Audio",
            "Agents",
            "Themes",
            "Keyboard shortcuts",
            "Task manager",
            "Calendar",
            "Close palette"
        ]
    );

    for character in "edit".chars() {
        assert!(shell.palette_push_char(character));
    }
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PaletteAction::LaunchApp(0));
    let launcher = shell.launcher(0).expect("launcher");
    assert_eq!(launcher.icon.as_deref(), Some("/tmp/editor.png"));
    assert_eq!(launcher.program, "example-editor");
    assert_eq!(launcher.args, ["--new"]);
    assert!(shell.launcher(2).is_none());
}

#[test]
fn palette_exposes_the_terminal_launcher() {
    let mut shell = shell();
    assert!(shell.palette_push_char('n'));
    assert!(shell.palette_push_char('e'));
    assert!(shell.palette_push_char('w'));
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "New terminal");
    assert_eq!(entries[0].action, PaletteAction::SpawnTerminal);
}

#[test]
fn palette_lists_and_filters_windows() {
    let mut shell = shell();
    shell.add_workspace("code");
    let terminal = window(&mut shell, "Terminal");
    let editor = window(&mut shell, "Editor");

    let entries = shell.palette_entries();
    assert!(entries.iter().any(|entry| entry.title == "Terminal"));
    assert!(entries.iter().any(|entry| entry.title == "Editor"));

    assert!(shell.palette_push_char('e'));
    assert!(shell.palette_push_char('d'));
    assert!(shell.palette_push_char('i'));
    assert!(shell.palette_push_char('t'));
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Editor");
    assert_eq!(entries[0].action, PaletteAction::FocusWindow(editor));
    assert_ne!(entries[0].action, PaletteAction::FocusWindow(terminal));
}

#[test]
fn palette_rows_slide_with_selection() {
    let mut shell = shell();
    for title in ["one", "two", "three", "four", "five"] {
        window(&mut shell, title);
    }
    shell.open_overlay();

    assert_eq!(shell.palette_entries().len(), 16);
    assert_eq!(shell.palette_visible_start(), 0);
    let first = shell.palette_row_rect(0).expect("row 0");
    assert_eq!(first.x, 70);
    assert_eq!(first.y, 90 + PALETTE_CONTENT_TOP);
    assert_eq!(
        shell.palette_row_at(Point::new(first.x + 2, first.y + 2)),
        Some(0)
    );
    assert_eq!(shell.palette_row_at(Point::new(20, first.y + 2)), None);

    for _ in 0..6 {
        shell.palette_move_selection(1);
    }
    assert_eq!(shell.palette_selected(), 6);
    assert_eq!(shell.palette_visible_start(), 1);
    assert_eq!(
        shell.palette_row_at(Point::new(first.x + 2, first.y + 2)),
        Some(1)
    );
    assert!(shell.palette_select(7));
    assert_eq!(shell.palette_selected(), 7);
    assert!(!shell.palette_select(16));
}

#[test]
fn palette_selection_wraps_and_activates() {
    let mut shell = shell();
    shell.add_workspace("code");
    shell.open_overlay();

    assert_eq!(shell.palette_selected(), 0);
    assert!(shell.palette_move_selection(-1));
    assert_eq!(shell.palette_selected(), 11);
    assert_eq!(shell.palette_activate(), Some(PaletteAction::Close));

    assert!(shell.palette_move_selection(1));
    assert_eq!(shell.palette_selected(), 0);
    assert_eq!(
        shell.palette_activate(),
        Some(PaletteAction::SwitchWorkspace(0))
    );

    assert!(shell.palette_push_char('c'));
    assert_eq!(shell.palette_selected(), 0);
    assert_eq!(
        shell.palette_activate(),
        Some(PaletteAction::SwitchWorkspace(1))
    );
}

#[test]
fn palette_query_resets_when_reopened() {
    let mut shell = shell();
    assert!(shell.toggle_overlay());
    assert!(shell.palette_push_char('w'));
    assert_eq!(shell.palette_query(), "w");
    assert!(!shell.toggle_overlay());
    assert_eq!(shell.palette_query(), "");
    assert!(shell.toggle_overlay());
    assert_eq!(shell.palette_query(), "");
    assert_eq!(shell.overlay_rect(), Some(Rect::new(60, 90, 680, 420)));
}

#[test]
fn overlay_rect_tracks_the_overlay_surface() {
    let mut shell = shell();
    assert_eq!(shell.overlay_rect(), None);
    let rect = Rect::new(100, 80, 400, 300);
    shell.set_overlay(Some(rect));
    assert_eq!(shell.overlay_rect(), Some(rect));
    shell.set_overlay(None);
    assert_eq!(shell.overlay_rect(), None);
}
