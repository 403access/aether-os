//! Water OS menu bar geometry.
//!
//! Mirrors `design-system/os/os.css` (`.os-menubar`, `.os-status`,
//! `.os-resources`, `.os-clock`) at a 1.14 interface scale. The layout is pure
//! geometry so the renderer and hit-testing cannot drift.

use aether_geometry::Rect;

/// Height of the full-width menu bar, matching `.os-menubar`.
pub const MENUBAR_HEIGHT: u32 = 38;
/// Height of every interactive item inside the bar.
const ITEM_HEIGHT: u32 = 26;
/// Item top inside the bar: `(38 - 26) / 2`.
const ITEM_Y: i32 = 6;
/// Horizontal bar padding.
const BAR_PAD: i32 = 10;
/// Gap between the sync pill, resources and the tray icons.
const ITEM_GAP: i32 = 5;
/// Extra space before the resource group.
const RESOURCE_GAP: i32 = 6;
/// Tray icon buttons (`.os-status--icon`): 8 px padding + 15 px glyph.
const ICON_BUTTON_WIDTH: i32 = 31;
/// Menu bar icon buttons (`.os-menubar__icon`).
const MENUBAR_ICON: i32 = 26;
const GLYPH: u32 = 15;
/// Menu button horizontal padding.
const MENU_PAD: i32 = 9;
/// Clock padding and the gap between date and time.
const CLOCK_PAD: i32 = 10;
const CLOCK_GAP: i32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuId {
    File,
    View,
    Go,
    Window,
    Help,
}

impl MenuId {
    pub const ALL: [Self; 5] = [Self::File, Self::View, Self::Go, Self::Window, Self::Help];

    pub fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::View => "View",
            Self::Go => "Go",
            Self::Window => "Window",
            Self::Help => "Help",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Sync,
    Resources,
    Network,
    Devices,
    Battery,
    Sound,
    Clock,
    Appearance,
    Gallery,
}

impl TrayAction {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Sync => "drop",
            Self::Resources => "chart",
            Self::Network => "wifi",
            Self::Devices => "bluetooth",
            Self::Battery => "battery",
            Self::Sound => "audio",
            Self::Clock => "calendar",
            Self::Appearance => "moon",
            Self::Gallery => "grid",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenubarAction {
    Logo,
    App,
    Menu(MenuId),
    Tray(TrayAction),
}

/// Commands a menu can run. Kept in geometry so the popover layout and the
/// input handler share one list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    NewTerminal,
    Explorer,
    TaskManager,
    Calendar,
    Themes,
    Shortcuts,
    CloseWindow,
    ToggleTheme,
    About,
    Network,
    Battery,
    Devices,
    Radio,
    Studio,
    Compare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuEntry {
    pub label: &'static str,
    pub icon: &'static str,
    pub command: MenuCommand,
    /// Draw a rule above this item.
    pub separator_before: bool,
}

macro_rules! entry {
    ($label:literal, $icon:literal, $command:ident) => {
        MenuEntry {
            label: $label,
            icon: $icon,
            command: MenuCommand::$command,
            separator_before: false,
        }
    };
    ($label:literal, $icon:literal, $command:ident, sep) => {
        MenuEntry {
            label: $label,
            icon: $icon,
            command: MenuCommand::$command,
            separator_before: true,
        }
    };
}

pub const FILE_MENU: &[MenuEntry] = &[
    entry!("New Terminal", "terminal", NewTerminal),
    entry!("Open Explorer", "folder", Explorer),
    entry!("Task Manager", "chart", TaskManager),
    entry!("Water settings", "drop", Themes, sep),
    entry!("Close window", "close", CloseWindow),
];

pub const VIEW_MENU: &[MenuEntry] = &[
    entry!("Toggle appearance", "moon", ToggleTheme),
    entry!("Water settings", "drop", Themes),
    entry!("Keyboard shortcuts", "keyboard", Shortcuts, sep),
];

pub const GO_MENU: &[MenuEntry] = &[
    entry!("Indexed files", "folder", Explorer),
    entry!("New terminal", "terminal", NewTerminal),
    entry!("Task Manager", "chart", TaskManager, sep),
];

pub const WINDOW_MENU: &[MenuEntry] = &[
    entry!("Terminal", "terminal", NewTerminal),
    entry!("Explorer", "folder", Explorer),
    entry!("Task Manager", "chart", TaskManager),
    entry!("Calendar", "calendar", Calendar),
    entry!("Network", "wifi", Network),
    entry!("Battery", "battery", Battery),
    entry!("Devices", "bluetooth", Devices),
    entry!("Tide Radio", "audio", Radio),
    entry!("App Studio", "layers", Studio),
    entry!("Data Cloud Compare", "explorer", Compare),
    entry!("Water settings", "drop", Themes),
    entry!("About Aether OS", "drop", About),
    entry!("Keyboard shortcuts", "keyboard", Shortcuts),
];

pub const HELP_MENU: &[MenuEntry] = &[
    entry!("Keyboard shortcuts", "keyboard", Shortcuts),
    entry!("About Aether OS", "drop", About),
];

pub fn menu_entries(id: MenuId) -> &'static [MenuEntry] {
    match id {
        MenuId::File => FILE_MENU,
        MenuId::View => VIEW_MENU,
        MenuId::Go => GO_MENU,
        MenuId::Window => WINDOW_MENU,
        MenuId::Help => HELP_MENU,
    }
}

/// Item height and padding inside a menu popover.
const MENU_ITEM_HEIGHT: u32 = 30;
const MENU_POP_PAD: i32 = 5;
const MENU_RULE: i32 = 9;
const MENU_ICON: u32 = 15;
const MENU_GAP: i32 = 8;
const MENU_SIDE_PAD: i32 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuPanel {
    pub menu: MenuId,
    pub rect: Rect,
    pub items: Vec<(MenuEntry, Rect)>,
}

pub fn menu_layout(bar: &Menubar, menu: MenuId, measure: Measure<'_>) -> MenuPanel {
    let entries = menu_entries(menu);
    let mut widest = 244;
    for entry in entries {
        widest = widest
            .max(text(measure, entry.label, 1) + MENU_ICON as i32 + MENU_GAP + 2 * MENU_SIDE_PAD);
    }
    let width = widest.min(320);
    let mut height = MENU_POP_PAD * 2;
    for entry in entries {
        height += MENU_ITEM_HEIGHT as i32;
        if entry.separator_before {
            height += MENU_RULE;
        }
    }
    let button = bar
        .menus
        .iter()
        .find(|(id, _)| *id == menu)
        .map(|(_, rect)| *rect)
        .unwrap_or(bar.rect);
    let x = button
        .x
        .min(bar.rect.right() as i32 - width - 8)
        .max(bar.rect.x + 8);
    let rect = Rect::new(x, bar.rect.bottom() as i32 + 7, width as u32, height as u32);
    let mut items = Vec::new();
    let mut y = rect.y + MENU_POP_PAD;
    for entry in entries {
        if entry.separator_before {
            y += MENU_RULE;
        }
        items.push((
            *entry,
            Rect::new(rect.x + 4, y, (width - 8) as u32, MENU_ITEM_HEIGHT),
        ));
        y += MENU_ITEM_HEIGHT as i32;
    }
    MenuPanel { menu, rect, items }
}

pub fn menu_hit(panel: &MenuPanel, x: i32, y: i32) -> Option<MenuCommand> {
    let point = aether_geometry::Point::new(x, y);
    panel
        .items
        .iter()
        .find(|(_, rect)| rect.contains(point))
        .map(|(entry, _)| entry.command)
}

/// One resource meter: caption, fill fraction and value text.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceMeter {
    pub label: String,
    pub value: String,
    pub fraction: f32,
}

/// A clickable item inside the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub action: MenubarAction,
    pub rect: Rect,
    pub glyph: Option<(String, u32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Menubar {
    pub rect: Rect,
    pub logo: Rect,
    pub app: Rect,
    pub app_title: String,
    pub menus: Vec<(MenuId, Rect)>,
    pub sync: Rect,
    pub sync_label: String,
    pub resources: Rect,
    pub meters: Vec<(ResourceMeter, Rect)>,
    pub tray: Vec<(TrayAction, Rect)>,
    pub clock: Rect,
    pub clock_date: String,
    pub clock_time: String,
    pub items: Vec<Item>,
    pub compact: bool,
}

/// Measure a chrome label with the compositor font, in whole pixels.
pub type Measure<'a> = &'a dyn Fn(&str, u32) -> u32;

fn text(measure: Measure<'_>, text: &str, scale: u32) -> i32 {
    measure(text, scale) as i32
}

/// Lay the menu bar out for an output. `app_title` is the focused app name and
/// `now` supplies the two clock strings so the layout stays pure.
#[allow(clippy::too_many_arguments)]
pub fn layout(
    bounds: Rect,
    measure: Measure<'_>,
    app_title: &str,
    sync_label: &str,
    meters: &[ResourceMeter],
    clock_date: &str,
    clock_time: &str,
) -> Menubar {
    let compact = bounds.width < 900;
    let rect = Rect::new(bounds.x, bounds.y, bounds.width, MENUBAR_HEIGHT);
    let top = bounds.y + ITEM_Y;
    let right = bounds.x + bounds.width as i32 - BAR_PAD;

    let logo = Rect::new(bounds.x + BAR_PAD, top, 28, ITEM_HEIGHT);

    // App title: `.os-menubar__app` — max 160 px, 2 px + 8 px padding.
    let mut title = app_title.to_string();
    let max_title = 160 - 10;
    if text(measure, &title, 10) > max_title && title.chars().count() > 4 {
        let mut limit = title.chars().count();
        while limit > 4
            && text(measure, &title.chars().take(limit).collect::<String>(), 1) > max_title
        {
            limit -= 1;
        }
        title = title.chars().take(limit).collect();
    }
    let app_width = (text(measure, &title, 10) + 10).max(24);
    let app = Rect::new(logo.right() as i32 + 4, top, app_width as u32, ITEM_HEIGHT);

    // Menus: gap 1 px between buttons, 9 px horizontal padding.
    let mut menus = Vec::new();
    let mut menu_x = app.right() as i32 + 4;
    for id in MenuId::ALL {
        let width = text(measure, id.label(), 10) + MENU_PAD * 2;
        let item = Rect::new(menu_x, top, width as u32, ITEM_HEIGHT);
        menus.push((id, item));
        menu_x += width + 1;
    }

    // Right cluster, packed from the right edge with 5 px gaps.
    let gallery = Rect::new(right - MENUBAR_ICON, top, MENUBAR_ICON as u32, ITEM_HEIGHT);
    let appearance = Rect::new(
        gallery.x - ITEM_GAP - MENUBAR_ICON,
        top,
        MENUBAR_ICON as u32,
        ITEM_HEIGHT,
    );

    let clock_width =
        CLOCK_PAD * 2 + text(measure, clock_date, 9) + CLOCK_GAP + text(measure, clock_time, 9);
    let clock = Rect::new(
        appearance.x - ITEM_GAP - clock_width,
        top,
        clock_width as u32,
        ITEM_HEIGHT,
    );
    let sound = Rect::new(
        clock.x - ITEM_GAP - ICON_BUTTON_WIDTH,
        top,
        ICON_BUTTON_WIDTH as u32,
        ITEM_HEIGHT,
    );
    // Battery shows the glyph plus a percentage: 8 + 15 + 6 + value + 8.
    let battery_width = 8 + GLYPH as i32 + 6 + text(measure, "78%", 9) + 8;
    let battery = Rect::new(
        sound.x - ITEM_GAP - battery_width,
        top,
        battery_width as u32,
        ITEM_HEIGHT,
    );
    let devices = Rect::new(
        battery.x - ITEM_GAP - ICON_BUTTON_WIDTH,
        top,
        ICON_BUTTON_WIDTH as u32,
        ITEM_HEIGHT,
    );
    let network = Rect::new(
        devices.x - ITEM_GAP - ICON_BUTTON_WIDTH,
        top,
        ICON_BUTTON_WIDTH as u32,
        ITEM_HEIGHT,
    );

    // Resources: 8 px padding, 6 px gaps, 7 px + 1 px between meters.
    let meter_widths: Vec<i32> = meters
        .iter()
        .map(|meter| {
            let value_min = if meter.label == "STORAGE" { 42 } else { 26 };
            text(measure, &meter.label, 7)
                + 4
                + 20
                + 4
                + text(measure, &meter.value, 8).max(value_min)
        })
        .collect();
    // `.os-resource + .os-resource` adds 7 px padding and a 1 px divider on
    // top of the 6 px flex gap.
    let separators = meters.len().saturating_sub(1) as i32;
    let resources_width =
        16 + meter_widths.iter().sum::<i32>() + 8 * separators + 7 * separators + 6;
    let resources = Rect::new(
        network.x - ITEM_GAP - resources_width,
        top,
        resources_width as u32,
        ITEM_HEIGHT,
    );

    // Sync pill: 8 px padding, 6 px dot, 6 px gap.
    let sync_width = 8 + 6 + 6 + text(measure, sync_label, 9) + 8;
    let sync = Rect::new(
        resources.x - RESOURCE_GAP - sync_width,
        top,
        sync_width as u32,
        ITEM_HEIGHT,
    );

    // Meter rects inside the resource group, flowing left to right.
    let mut meters_with_rects = Vec::new();
    let mut meter_x = resources.x + 8;
    for (meter, width) in meters.iter().zip(meter_widths.iter()) {
        let rect = Rect::new(meter_x, top, *width as u32, ITEM_HEIGHT);
        meters_with_rects.push((meter.clone(), rect));
        meter_x += width + 8 + 7;
    }

    let mut items = Vec::new();
    items.push(Item {
        action: MenubarAction::Logo,
        rect: logo,
        glyph: None,
    });
    items.push(Item {
        action: MenubarAction::App,
        rect: app,
        glyph: None,
    });
    for (id, rect) in &menus {
        items.push(Item {
            action: MenubarAction::Menu(*id),
            rect: *rect,
            glyph: None,
        });
    }
    let tray_items: [(TrayAction, Rect); 8] = [
        (TrayAction::Sync, sync),
        (TrayAction::Resources, resources),
        (TrayAction::Network, network),
        (TrayAction::Devices, devices),
        (TrayAction::Battery, battery),
        (TrayAction::Sound, sound),
        (TrayAction::Clock, clock),
        (TrayAction::Appearance, appearance),
    ];
    let mut tray = Vec::new();
    for (action, rect) in tray_items {
        tray.push((action, rect));
        items.push(Item {
            action: MenubarAction::Tray(action),
            rect,
            glyph: Some((action.icon().to_string(), GLYPH)),
        });
    }
    tray.push((TrayAction::Gallery, gallery));
    items.push(Item {
        action: MenubarAction::Tray(TrayAction::Gallery),
        rect: gallery,
        glyph: Some((TrayAction::Gallery.icon().to_string(), GLYPH)),
    });

    Menubar {
        rect,
        logo,
        app,
        app_title: title,
        menus,
        sync,
        sync_label: sync_label.to_string(),
        resources,
        meters: meters_with_rects,
        tray,
        clock,
        clock_date: clock_date.to_string(),
        clock_time: clock_time.to_string(),
        items,
        compact,
    }
}

pub fn hit(bar: &Menubar, x: i32, y: i32) -> Option<MenubarAction> {
    let point = aether_geometry::Point::new(x, y);
    if !bar.rect.contains(point) {
        return None;
    }
    bar.items
        .iter()
        .find(|item| item.rect.contains(point))
        .map(|item| item.action)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic metrics: 6 px per character at the 11 px scale, 5 px at 9 px.
    fn measure(text: &str, scale: u32) -> u32 {
        let per = if scale == 3 { 5 } else { 6 };
        text.chars().count() as u32 * per
    }

    fn meters() -> Vec<ResourceMeter> {
        vec![
            ResourceMeter {
                label: "CPU".into(),
                value: "25%".into(),
                fraction: 0.25,
            },
            ResourceMeter {
                label: "RAM".into(),
                value: "61%".into(),
                fraction: 0.61,
            },
            ResourceMeter {
                label: "STORAGE".into(),
                value: "812 GB".into(),
                fraction: 0.79,
            },
        ]
    }

    fn bar(width: u32) -> Menubar {
        layout(
            Rect::new(0, 0, width, 800),
            &measure,
            "Tank ⇄ Jottacloud",
            "Synced",
            &meters(),
            "Wed, Nov 18",
            "04:47 PM",
        )
    }

    #[test]
    fn bar_is_full_width_and_38_px() {
        let menubar = bar(1280);
        assert_eq!(menubar.rect.height, MENUBAR_HEIGHT);
        assert_eq!(menubar.rect.width, 1280);
        assert_eq!(menubar.logo.x, 10);
        assert_eq!(menubar.logo.width, 28);
        assert!(!menubar.compact);
    }

    #[test]
    fn tray_packs_from_the_right_edge() {
        let menubar = bar(1280);
        let gallery = menubar
            .tray
            .iter()
            .find(|(action, _)| *action == TrayAction::Gallery)
            .map(|(_, rect)| *rect)
            .expect("gallery");
        assert_eq!(gallery.right(), 1270);
        assert_eq!(gallery.width, 26);
    }

    #[test]
    fn right_cluster_items_never_overlap() {
        let menubar = bar(1280);
        let mut sorted: Vec<Rect> = menubar.tray.iter().map(|(_, rect)| *rect).collect();
        sorted.sort_by_key(|rect| rect.x);
        for pair in sorted.windows(2) {
            assert!(
                pair[0].right() as i32 <= pair[1].x,
                "overlapping tray items: {:?} {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn menus_follow_the_app_title_and_are_clickable() {
        let menubar = bar(1280);
        assert_eq!(menubar.menus.len(), 5);
        assert_eq!(menubar.menus[0].0, MenuId::File);
        let (_, file) = menubar.menus[0];
        assert_eq!(
            hit(&menubar, file.x + 4, file.y + 4),
            Some(MenubarAction::Menu(MenuId::File))
        );
        assert_eq!(hit(&menubar, 2, 2), None);
        assert!((menubar.menus[4].1.right() as i32) < menubar.sync.x);
    }

    #[test]
    fn sync_pill_sits_left_of_the_resources() {
        let menubar = bar(1280);
        assert!((menubar.sync.right() as i32) < menubar.resources.x);
        assert_eq!(
            menubar.resources.x - menubar.sync.right() as i32,
            RESOURCE_GAP
        );
        assert_eq!(
            hit(&menubar, menubar.sync.x + 2, menubar.sync.y + 2),
            Some(MenubarAction::Tray(TrayAction::Sync))
        );
    }

    #[test]
    fn clock_is_clickable() {
        let menubar = bar(1280);
        assert_eq!(
            hit(&menubar, menubar.clock.x + 4, menubar.clock.y + 4),
            Some(MenubarAction::Tray(TrayAction::Clock))
        );
    }
}
