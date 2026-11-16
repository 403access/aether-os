//! App-level integration tests (damage tests live next to `damage.rs`).

use aether_geometry::{Point as AetherPoint, Rect as AetherRect};
use aether_shell::{OutputInfo, WindowSpec};
use smithay::utils::Rectangle;

use super::windows::FrameHit;
use super::*;
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
use smithay::utils::Transform;

fn test_app() -> (Display<App>, App) {
    let dir = std::env::temp_dir().join(format!("aether-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::env::set_var("XDG_RUNTIME_DIR", &dir);
    std::env::remove_var("AETHER_INDEX_SOCKET");
    std::env::remove_var("AETHER_AGENT_SOCKET");
    let display: Display<App> = Display::new().expect("display");
    let (app, _keyboard) = App::new(&display.handle()).expect("app");
    (display, app)
}

fn output(name: &str) -> Output {
    let output = Output::new(
        name.to_string(),
        PhysicalProperties {
            size: (300, 200).into(),
            subpixel: Subpixel::Unknown,
            make: "Aether".into(),
            model: name.into(),
        },
    );
    let mode = OutputMode {
        size: (800, 600).into(),
        refresh: 120_000,
    };
    output.set_preferred(mode);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );
    output
}

#[test]
fn take_damage_drains_once() {
    let (_display, mut app) = test_app();
    app.mark_full_damage();
    assert!(app.needs_frame());
    let (full, damage) = app.take_damage();
    assert!(full);
    assert!(damage.is_empty());
    assert!(!app.needs_frame());
    let (full, damage) = app.take_damage();
    assert!(!full);
    assert!(damage.is_empty());

    app.mark_damage_rect(AetherRect::new(10, 20, 30, 40));
    let (full, damage) = app.take_damage();
    assert!(!full);
    assert_eq!(damage.len(), 1);
    assert!(!app.needs_frame());
}

#[test]
fn client_frames_inset_content_and_hit_the_title_bar() {
    let (_display, mut app) = test_app();
    let id = app
        .shell
        .add_window(WindowSpec::new("Wayland Terminal", "wayland.client"));
    let rect = app.window_rect(id).expect("window rect");
    let content = app.window_content_rect(id).expect("content rect");
    assert_eq!(content.y, rect.y + WINDOW_FRAME_HEIGHT);
    assert_eq!(
        content.height,
        rect.height.saturating_sub(WINDOW_FRAME_HEIGHT as u32)
    );

    let title = AetherPoint::new(rect.x + 20, rect.y + 5);
    assert_eq!(app.window_frame_hit(title), Some((id, FrameHit::Title)));
    let body = AetherPoint::new(rect.x + 20, content.y + 5);
    assert_eq!(app.window_frame_hit(body), None);

    let close = app.window_close_rect(id).expect("close rect");
    let center = AetherPoint::new(close.x + 2, close.y + 2);
    assert_eq!(app.window_frame_hit(center), Some((id, FrameHit::Close)));
}

#[test]
fn add_and_remove_output_updates_the_shell() {
    let (_display, mut app) = test_app();
    app.add_output(
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: AetherRect::new(1280, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
        output("right"),
    );
    assert!(app.shell().outputs().iter().any(|o| o.id == 2));
    assert_eq!(
        app.shell().output_bounds(2),
        AetherRect::new(1280, 0, 800, 600)
    );
    assert!(app.remove_output(2).is_some());
    assert!(!app.shell().outputs().iter().any(|o| o.id == 2));
}

#[test]
fn output_damage_is_clipped_to_each_output() {
    let (_display, mut app) = test_app();
    app.add_output(
        OutputInfo {
            id: 2,
            name: "right".into(),
            rect: AetherRect::new(1280, 0, 800, 600),
            scale: 1.0,
            refresh_hz: 60,
            physical_size_mm: (0, 0),
            primary: false,
        },
        output("right"),
    );
    app.take_damage();
    app.mark_damage_rect(AetherRect::new(1290, 10, 30, 40));
    let (full, damage) = app.output_damage(Some(2)).expect("second output damage");
    assert!(!full);
    assert_eq!(
        damage,
        vec![Rectangle::new((10, 10).into(), (30, 40).into())]
    );
    assert!(app.output_damage(Some(1)).is_none());
}
