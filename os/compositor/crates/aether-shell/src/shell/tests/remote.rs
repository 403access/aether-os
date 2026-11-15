use super::*;
use crate::remote::{RemoteAction, RemoteLayer};

fn shell() -> Shell {
    Shell::new(Size::new(1440, 900), LayoutConfig::default(), 120)
}

#[test]
fn opens_centred_and_actions_are_hit_testable() {
    let mut shell = shell();
    shell.open_remote(RemoteLayer::demo()).unwrap();
    assert!(shell.remote_open());
    let rect = shell.remote_rect();
    assert!(rect.width >= 600);
    assert_eq!(rect.x, (1440 - rect.width as i32) / 2);
    for index in 0..RemoteAction::ALL.len() {
        let action = shell.remote_action_rect(index).unwrap();
        let center = Point::new(
            action.x + action.width as i32 / 2,
            action.y + action.height as i32 / 2,
        );
        assert_eq!(shell.remote_action_at(center), Some(index));
    }
    assert_eq!(shell.remote_action_at(Point::new(0, 0)), None);
    assert!(!shell.remote_select(3));
}

#[test]
fn actions_toggle_and_stop() {
    let mut shell = shell();
    shell.open_remote(RemoteLayer::demo()).unwrap();
    assert!(shell.remote_move_selection(1));
    assert_eq!(shell.remote_activate(), Some(RemoteAction::ToggleInput));
    assert!(!shell.remote_layer().unwrap().input_enabled());
    assert!(shell.remote_move_selection(1));
    assert_eq!(shell.remote_activate(), Some(RemoteAction::ToggleClipboard));
    assert!(shell.remote_layer().unwrap().clipboard_enabled());
    assert!(shell.remote_status().unwrap().contains("18.4 ms"));
    shell.remote_select(0);
    assert_eq!(shell.remote_activate(), Some(RemoteAction::StopSession));
    assert!(shell.remote_status().is_none());
    shell.close_remote();
    assert!(!shell.remote_open());
}

#[test]
fn opening_another_overlay_closes_remote() {
    let mut shell = shell();
    shell
        .open_remote(RemoteLayer::waiting("K7QM-2H4P"))
        .unwrap();
    shell.open_explorer().unwrap();
    assert!(!shell.remote_open());
    shell.open_remote(RemoteLayer::demo()).unwrap();
    shell.open_chat().unwrap();
    assert!(!shell.remote_open());
    shell.open_remote(RemoteLayer::demo()).unwrap();
    shell.open_overlay();
    assert!(!shell.remote_open());
}
