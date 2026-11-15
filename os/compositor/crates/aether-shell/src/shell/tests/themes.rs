use super::*;
use crate::remote::RemoteLayer;

fn shell() -> Shell {
    Shell::new(Size::new(1440, 900), LayoutConfig::default(), 120)
}

#[test]
fn opens_centred_with_every_reference_row_hittable() {
    let mut shell = shell();
    shell.open_themes().unwrap();
    assert!(shell.themes_open());
    let rect = shell.theme_rect();
    assert_eq!(rect.width, design::THEME_PANEL_WIDTH);
    assert_eq!(rect.height, design::THEME_PANEL_HEIGHT);
    assert_eq!(rect.x, (1440 - rect.width as i32) / 2);
    assert_eq!(theme::ThemeField::ALL.len(), 11);
    for index in 0..theme::ThemeField::ALL.len() {
        let row = shell.theme_row_rect(index).unwrap();
        let center = Point::new(row.x + row.width as i32 / 2, row.y + row.height as i32 / 2);
        assert_eq!(shell.theme_row_at(center), Some(index));
    }
    assert_eq!(shell.theme_row_at(Point::new(rect.x + 2, rect.y + 2)), None);
    assert!(!shell.theme_select(11));
}

#[test]
fn switches_toggle_and_selections_cycle() {
    let mut shell = shell();
    shell.open_themes().unwrap();
    assert!(shell.theme_select(1));
    assert_eq!(shell.theme_activate(), Some(theme::ThemeAction::Changed));
    assert!(!shell.theme_settings().effects);
    assert!(!shell.theme_layer().unwrap().settings().effects);

    assert!(shell.theme_select(6));
    assert_eq!(shell.theme_adjust(1), Some(theme::ThemeAction::Changed));
    assert_eq!(shell.theme_settings().speed, theme::WaterSpeed::Rapid);
    assert_eq!(
        shell.theme_settings().intensity,
        theme::WaterIntensity::Subtle
    );

    assert!(shell.theme_select(10));
    assert!(shell.theme_activate().is_some());
    assert!(!shell.theme_settings().glass);
}

#[test]
fn reset_and_use_water_update_the_active_theme() {
    let mut shell = shell();
    shell.open_themes().unwrap();
    shell.theme_select(0);
    shell.theme_adjust(1);
    shell.theme_select(10);
    shell.theme_activate();
    assert_ne!(shell.theme_settings().mode, theme::ThemeMode::Water);
    assert!(!shell.theme_settings().glass);
    assert_eq!(
        shell.theme_use_water(),
        Some(theme::ThemeAction::ActivateWater)
    );
    assert!(shell.theme_settings().is_water());
    assert!(!shell.theme_settings().glass);
    assert_eq!(shell.theme_reset(), theme::ThemeAction::Reset);
    assert_eq!(shell.theme_settings(), &theme::ThemeSettings::default());
}

#[test]
fn status_action_and_footer_are_hit_testable() {
    let mut shell = shell();
    shell.open_themes().unwrap();
    assert!(!shell.theme_action_at(Point::new(
        shell.theme_action_rect().x,
        shell.theme_action_rect().y
    )));
    let mut settings = *shell.theme_settings();
    settings.mode = theme::ThemeMode::Dark;
    shell.set_theme_settings(settings);
    let action = shell.theme_action_rect();
    let center = Point::new(action.x + 4, action.y + 4);
    assert!(shell.theme_action_at(center));
    assert_eq!(
        shell.theme_use_water(),
        Some(theme::ThemeAction::ActivateWater)
    );
    let reset = shell.theme_reset_rect();
    assert!(shell.theme_reset_at(Point::new(
        reset.x + reset.width as i32 - 2,
        reset.y + reset.height as i32 - 2
    )));
    let close = shell.theme_close_rect();
    assert!(shell.theme_close_at(Point::new(
        close.x + close.width as i32 / 2,
        close.y + close.height as i32 / 2
    )));
}

#[test]
fn opening_another_surface_closes_themes() {
    let mut shell = shell();
    shell.open_themes().unwrap();
    shell.open_explorer().unwrap();
    assert!(!shell.themes_open());
    shell.open_themes().unwrap();
    shell.open_chat().unwrap();
    assert!(!shell.themes_open());
    shell.open_themes().unwrap();
    shell.open_remote(RemoteLayer::demo()).unwrap();
    assert!(!shell.themes_open());
    shell.open_themes().unwrap();
    shell.open_overlay();
    assert!(!shell.themes_open());
}

#[test]
fn layout_keeps_every_group_and_the_footer_inside_the_panel() {
    let shell = shell();
    let panel = shell.theme_rect();
    let status = shell.theme_status_rect();
    let preview = shell.theme_preview_rect();
    assert!(panel.contains(Point::new(status.x + 1, status.y + 1)));
    assert!(preview.y > status.y + status.height as i32);
    let first = shell.theme_row_rect(0).unwrap();
    assert!(first.y > preview.y + preview.height as i32);
    let last = shell
        .theme_row_rect(theme::ThemeField::ALL.len() - 1)
        .unwrap();
    let reset = shell.theme_reset_rect();
    assert!(reset.y > last.y + last.height as i32);
    assert!(reset.y + reset.height as i32 <= panel.y + panel.height as i32);
    for index in 1..theme::ThemeField::ALL.len() {
        let previous = shell.theme_row_rect(index - 1).unwrap();
        let row = shell.theme_row_rect(index).unwrap();
        assert!(row.y >= previous.y + previous.height as i32);
    }
}

#[test]
fn palette_lists_the_themes_app() {
    let mut shell = shell();
    let entry = shell
        .palette_entries()
        .into_iter()
        .find(|entry| entry.title == "Themes")
        .expect("themes entry");
    assert_eq!(entry.action, PaletteAction::OpenThemes);
    assert_eq!(entry.detail, "Theme · Water settings");

    assert!(shell.palette_push_char('~'));
    assert!(shell.palette_push_char('t'));
    assert!(shell.palette_push_char('h'));
    assert!(shell.palette_push_char('e'));
    assert!(shell.palette_push_char('m'));
    assert!(shell.palette_push_char('e'));
    assert!(shell.palette_push_char('s'));
    let entries = shell.palette_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PaletteAction::OpenThemes);
}
