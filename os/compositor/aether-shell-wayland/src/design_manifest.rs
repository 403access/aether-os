//! Machine-readable design state for the compositor.
//!
//! `aether-shell-wayland --bin design-manifest` prints a flat JSON map of the
//! chrome's tokens, type scale, geometry, typography and effects. The parity
//! tooling (`scripts/screenshots/design-state.mjs`) extracts the same keys from
//! the Water OS demo's computed styles and diffs the two documents, so design
//! drift is caught as property values instead of pixels.
//!
//! Only values the renderer actually paints with belong here. Colours are
//! `#rrggbb` when opaque and `#rrggbbaa` otherwise; lengths are CSS pixels.

use aether_geometry::Rect;
use aether_shell::theme::{ThemeSettings, ThemeTokens};
use serde_json::{json, Map, Value};

use crate::app::overlays::dock::layout as dock_layout;
use crate::app::overlays::menubar::{
    layout as menubar_layout, ResourceMeter, CLOCK_PAD, MENU_GAP, MENU_ITEM_HEIGHT,
};
use crate::app::text_style;
use crate::app::tray::TrayId;
use crate::render::surfaces::status::{BAR_FILL, POPOVER_FILL};

const VIEWPORT: (u32, u32) = (1280, 800);

fn hex(color: [u8; 4]) -> String {
    if color[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color[0], color[1], color[2], color[3]
        )
    }
}

/// Round to 0.01 px, in f64 so JSON prints `12.54` rather than `12.5399…`.
fn px(value: f32) -> Value {
    json!(((value as f64) * 100.0).round() / 100.0)
}

fn weight(scale: u32) -> &'static str {
    if text_style::text_bold(scale) {
        "bold"
    } else {
        "regular"
    }
}

fn tray_id(tray: TrayId) -> &'static str {
    match tray {
        TrayId::Network => "network",
        TrayId::Devices => "devices",
        TrayId::Battery => "battery",
        TrayId::Sound => "sound",
        TrayId::Calendar => "calendar",
    }
}

pub fn manifest() -> Value {
    let settings = ThemeSettings::default();
    let tokens = ThemeTokens::for_settings(&settings);
    let font = aether_text::FontRenderer::discover();
    let measure = |text: &str, scale: u32| -> u32 {
        match font.as_ref() {
            Some(font) => {
                font.measure(
                    text,
                    text_style::text_px(scale & 0xff),
                    text_style::text_bold(scale),
                    text_style::text_spacing(scale),
                )
                .0
            }
            None => (text.chars().count() as u32) * 6,
        }
    };

    let meters = [
        ResourceMeter {
            label: "CPU".into(),
            value: "14%".into(),
            fraction: 0.14,
        },
        ResourceMeter {
            label: "RAM".into(),
            value: "7%".into(),
            fraction: 0.07,
        },
        ResourceMeter {
            label: "STORAGE".into(),
            value: "—".into(),
            fraction: 0.0,
        },
    ];
    let bounds = Rect::new(0, 0, VIEWPORT.0, VIEWPORT.1);
    let bar = menubar_layout(
        bounds,
        &measure,
        "Aether OS",
        "Synced",
        &meters,
        "Wed, Nov 18",
        "04:47 PM",
    );
    let dock = dock_layout(bounds);

    let mut values = Map::new();
    let set = |values: &mut Map<String, Value>, key: &str, value: Value| {
        values.insert(key.to_string(), value);
    };

    // The painted background, not the (unused) theme base.
    let background = crate::render::BACKGROUND;
    set(
        &mut values,
        "token.bg",
        json!(hex([
            (background.r() * 255.0).round() as u8,
            (background.g() * 255.0).round() as u8,
            (background.b() * 255.0).round() as u8,
            255,
        ])),
    );
    // Window fill; the demo uses `color-mix(surface-solid 92%, transparent)`.
    set(&mut values, "token.surface", json!(hex(tokens.panel)));
    set(&mut values, "token.popover", json!(hex(POPOVER_FILL)));
    set(&mut values, "token.menubar-fill", json!(hex(BAR_FILL)));
    set(&mut values, "token.text", json!(hex(tokens.text)));
    set(&mut values, "token.text-2", json!(hex(tokens.soft)));
    set(&mut values, "token.text-3", json!(hex(tokens.dim)));
    set(&mut values, "token.accent", json!(hex(tokens.accent(255))));
    set(&mut values, "token.line", json!(hex(tokens.border(38))));
    set(
        &mut values,
        "token.line-strong",
        json!(hex(tokens.border(76))),
    );
    set(&mut values, "token.green", json!(hex(tokens.green)));

    // The fractional OS type ramp shared with the renderer. The legacy
    // integer scales are not part of the OS design state.
    for scale in 7..=13u32 {
        set(
            &mut values,
            &format!("type.{scale}.px"),
            px(text_style::text_px(scale)),
        );
    }

    // Menu bar.
    set(&mut values, "menubar.height", json!(bar.rect.height));
    set(&mut values, "menubar.pad-x", json!(10));
    set(&mut values, "menubar.logo.width", json!(bar.logo.width));
    set(&mut values, "menubar.logo.height", json!(bar.logo.height));
    set(
        &mut values,
        "menubar.app.font-px",
        px(text_style::text_px(10)),
    );
    set(
        &mut values,
        "menubar.app.tracking",
        px(text_style::text_spacing(0x100 | 10)),
    );
    set(&mut values, "menubar.app.weight", json!(weight(0x100 | 10)));
    set(&mut values, "menubar.app.max-width", json!(160));
    set(&mut values, "menubar.menu.item-height", json!(26));
    set(&mut values, "menubar.menu.pad-x", json!(9));
    set(&mut values, "menubar.menu.gap", json!(1));
    set(
        &mut values,
        "menubar.menu.font-px",
        px(text_style::text_px(10)),
    );
    set(&mut values, "menubar.menu.weight", json!(weight(10)));
    set(&mut values, "menubar.menu.radius", json!(5));
    set(&mut values, "menubar.menu.color", json!(hex(tokens.soft)));
    set(&mut values, "menubar.sync.height", json!(bar.sync.height));
    set(
        &mut values,
        "menubar.sync.font-px",
        px(text_style::text_px(9)),
    );
    set(&mut values, "menubar.sync.radius", json!(5));
    set(
        &mut values,
        "menubar.sync.border",
        json!(hex(tokens.border(38))),
    );
    set(
        &mut values,
        "menubar.resources.height",
        json!(bar.resources.height),
    );
    set(&mut values, "menubar.resources.pad-x", json!(8));
    set(&mut values, "menubar.resources.gap", json!(6));
    set(
        &mut values,
        "menubar.resources.label-px",
        px(text_style::text_px(7)),
    );
    set(
        &mut values,
        "menubar.resources.label-tracking",
        px(text_style::text_spacing(0x100 | 7)),
    );
    set(
        &mut values,
        "menubar.resources.label-weight",
        json!(weight(0x100 | 7)),
    );
    set(
        &mut values,
        "menubar.resources.label-color",
        json!(hex(tokens.dim)),
    );
    set(
        &mut values,
        "menubar.resources.value-px",
        px(text_style::text_px(8)),
    );
    set(
        &mut values,
        "menubar.resources.value-weight",
        json!(weight(0x100 | 8)),
    );
    set(
        &mut values,
        "menubar.resources.value-color",
        json!(hex(tokens.soft)),
    );
    set(&mut values, "menubar.resources.meter-w", json!(20));
    set(&mut values, "menubar.resources.meter-h", json!(3));
    set(&mut values, "menubar.clock.height", json!(bar.clock.height));
    set(&mut values, "menubar.clock.pad-x", json!(CLOCK_PAD));
    set(&mut values, "menubar.clock.gap", json!(7));
    set(
        &mut values,
        "menubar.clock.font-px",
        px(text_style::text_px(9)),
    );
    set(
        &mut values,
        "menubar.clock.date-color",
        json!(hex(tokens.dim)),
    );
    set(
        &mut values,
        "menubar.clock.time-color",
        json!(hex(tokens.text)),
    );
    set(
        &mut values,
        "menubar.clock.time-weight",
        json!(weight(0x100 | 9)),
    );
    set(&mut values, "menubar.tray.button-width", json!(31));
    // Battery: 8 + 15 + 6 + value + 8.
    let battery_width = 8 + 15 + 6 + measure("78%", 9) + 8;
    set(
        &mut values,
        "menubar.tray.battery-width",
        json!(battery_width),
    );
    set(&mut values, "menubar.tray.icon", json!(15));

    // Window chrome.
    set(&mut values, "window.radius", json!(14));
    set(&mut values, "window.titlebar-height", json!(40));
    set(&mut values, "window.light-size", json!(11));
    set(&mut values, "window.light-gap", json!(6));
    set(
        &mut values,
        "window.light-close",
        json!(hex([0xff, 0x8a, 0x92, 255])),
    );
    set(
        &mut values,
        "window.light-min",
        json!(hex([0xff, 0xd2, 0x7a, 255])),
    );
    set(
        &mut values,
        "window.light-max",
        json!(hex([0x5e, 0xf2, 0xb0, 255])),
    );
    set(&mut values, "window.mark", json!(23));
    set(&mut values, "window.mark-icon", json!(13));
    set(&mut values, "window.title-px", px(text_style::text_px(10)));
    set(
        &mut values,
        "window.title-weight",
        json!(weight(0x100 | 10)),
    );
    set(
        &mut values,
        "window.title-tracking",
        px(text_style::text_spacing(0x100 | 10)),
    );
    set(&mut values, "window.title-color", json!(hex(tokens.text)));
    set(
        &mut values,
        "window.subtitle-px",
        px(text_style::text_px(10)),
    );
    set(&mut values, "window.subtitle-color", json!(hex(tokens.dim)));
    set(&mut values, "window.inset-highlight", json!(0.07));
    set(&mut values, "window.tide.ring", json!(2.5));
    set(&mut values, "window.tide.cycle-ms", json!(8000));
    set(&mut values, "window.tide.opacity", json!(0.85));

    // Menu popover: `menu_layout` constants.
    set(&mut values, "menu.offset", json!(7));
    set(&mut values, "menu.min-width", json!(244));
    set(&mut values, "menu.padding", json!(5));
    set(&mut values, "menu.radius", json!(10));
    set(&mut values, "menu.popover-fill", json!(hex(POPOVER_FILL)));
    set(&mut values, "menu.row-height", json!(MENU_ITEM_HEIGHT));
    set(&mut values, "menu.row-font-px", px(text_style::text_px(12)));
    set(&mut values, "menu.row-weight", json!(weight(12)));
    set(&mut values, "menu.icon", json!(15));
    set(&mut values, "menu.icon-gap", json!(MENU_GAP));

    // Dock.
    set(&mut values, "dock.width", json!(dock.rect.width));
    set(&mut values, "dock.height", json!(dock.rect.height));
    set(&mut values, "dock.pad", json!(8));
    set(&mut values, "dock.item", json!(46));
    set(&mut values, "dock.tile", json!(38));
    set(&mut values, "dock.tile-radius", json!(10));
    set(&mut values, "dock.gap", json!(6));
    set(&mut values, "dock.radius", json!(17));

    // Tray panel sizes.
    for tray in [
        TrayId::Network,
        TrayId::Devices,
        TrayId::Battery,
        TrayId::Sound,
        TrayId::Calendar,
    ] {
        let (width, height) = tray.size();
        let id = tray_id(tray);
        set(&mut values, &format!("tray.{id}.width"), json!(width));
        set(&mut values, &format!("tray.{id}.height"), json!(height));
    }

    // Structure counts that the demo exposes in the DOM.
    set(&mut values, "structure.menus", json!(5));
    set(&mut values, "structure.dock-items", json!(dock.items.len()));
    set(
        &mut values,
        "structure.settings-tabs",
        json!(crate::app::system::SETTINGS_TABS.len()),
    );

    json!({
        "side": "aether",
        "viewport": {"width": VIEWPORT.0, "height": VIEWPORT.1},
        "values": values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_covers_the_chrome() {
        let manifest = manifest();
        let values = manifest["values"].as_object().expect("values");
        assert!(values.len() > 60);
        assert_eq!(values["menubar.height"], json!(38));
        assert_eq!(values["window.radius"], json!(14));
        assert_eq!(values["type.10.px"], json!(12.54));
    }
}
