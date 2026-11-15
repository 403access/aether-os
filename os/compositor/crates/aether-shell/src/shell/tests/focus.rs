use super::*;

fn shell() -> Shell {
    Shell::new(Size::new(800, 600), LayoutConfig::default(), 120)
}

fn window(shell: &mut Shell, title: &str) -> WindowId {
    shell.add_window(WindowSpec::new(title, "test.app"))
}

#[test]
fn mru_cycles_history_without_reordering_during_the_cycle() {
    let mut shell = shell();
    let a = window(&mut shell, "a");
    let b = window(&mut shell, "b");
    let c = window(&mut shell, "c");
    assert!(shell.focus_window(a));
    assert!(shell.focus_window(b));
    assert!(shell.focus_window(c));
    assert_eq!(shell.focus_mru(1), Some(b));
    assert_eq!(shell.focus_mru(1), Some(a));
    assert_eq!(shell.focus_mru(1), Some(c));
    assert_eq!(shell.focus_mru(-1), Some(a));
    // A direct focus resets the cycle anchor.
    assert!(shell.focus_window(b));
    assert_eq!(shell.focus_mru(1), Some(c));
}

#[test]
fn closing_a_window_removes_it_from_the_stack() {
    let mut shell = shell();
    let a = window(&mut shell, "a");
    let b = window(&mut shell, "b");
    assert!(shell.focus_window(a));
    assert!(shell.focus_window(b));
    shell.close_window(b);
    assert_eq!(shell.focus_mru(1), Some(a));
}
