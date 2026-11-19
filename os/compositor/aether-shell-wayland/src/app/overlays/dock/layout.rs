//! Desktop dock geometry.
//!
//! Mirrors `.os-dock` from the Water OS demo: a centered glass strip 8 px above
//! the bottom edge, 46 px items with 38 px tiles and a separator before the
//! utility group. Pure geometry so the renderer and hit-testing agree.

use aether_geometry::Rect;

pub const DOCK_HEIGHT: u32 = 62;
pub const DOCK_BOTTOM: i32 = 8;
const PAD: i32 = 8;
const ITEM: i32 = 46;
const GAP: i32 = 6;
const SEP_WIDTH: i32 = 1;
const SEP_MARGIN: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockAction {
    Explorer,
    Search,
    Settings,
    Terminal,
    Radio,
    About,
    Trash,
    Gallery,
}

impl DockAction {
    pub const FIRST_GROUP: [Self; 6] = [
        Self::Explorer,
        Self::Search,
        Self::Settings,
        Self::Terminal,
        Self::Radio,
        Self::About,
    ];
    pub const SECOND_GROUP: [Self; 2] = [Self::Trash, Self::Gallery];

    pub fn label(self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search",
            Self::Settings => "Settings",
            Self::Terminal => "Terminal",
            Self::Radio => "Tide Radio",
            Self::About => "About",
            Self::Trash => "Trash",
            Self::Gallery => "Design system",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Explorer => "folder",
            Self::Search => "search",
            Self::Settings => "gear",
            Self::Terminal => "terminal",
            Self::Radio => "audio",
            Self::About => "drop",
            Self::Trash => "trash",
            Self::Gallery => "layers",
        }
    }

    /// Per-tile accent from the demo dock.
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Explorer => [0x38, 0xd6, 0xff],
            Self::Search | Self::Trash => [0x9e, 0xc5, 0xce],
            Self::Settings => [0x5e, 0xf2, 0xb0],
            Self::Terminal => [0x6f, 0xd5, 0xff],
            Self::Radio => [0xff, 0x9a, 0xd5],
            Self::About => [0xff, 0xd2, 0x7a],
            Self::Gallery => [0xc3, 0xa8, 0xff],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dock {
    pub rect: Rect,
    pub items: Vec<(DockAction, Rect)>,
    pub separator: Rect,
}

pub fn layout(bounds: Rect) -> Dock {
    let group_one = DockAction::FIRST_GROUP.len() as i32;
    let group_two = DockAction::SECOND_GROUP.len() as i32;
    let items = group_one + group_two;
    let width = PAD * 2 + ITEM * items + GAP * (items - 1) + SEP_WIDTH + SEP_MARGIN * 2 + GAP;
    let x = bounds.x + (bounds.width as i32 - width) / 2;
    let y = bounds.y + bounds.height as i32 - DOCK_BOTTOM - DOCK_HEIGHT as i32;
    let rect = Rect::new(x, y, width as u32, DOCK_HEIGHT);

    let mut item_rects = Vec::new();
    let mut cursor = rect.x + PAD;
    let item_y = rect.y + (DOCK_HEIGHT as i32 - ITEM) / 2;
    for action in DockAction::FIRST_GROUP {
        item_rects.push((action, Rect::new(cursor, item_y, ITEM as u32, ITEM as u32)));
        cursor += ITEM + GAP;
    }
    let separator = Rect::new(cursor + SEP_MARGIN, rect.y + 14, SEP_WIDTH as u32, 34);
    cursor += SEP_WIDTH + SEP_MARGIN * 2;
    for action in DockAction::SECOND_GROUP {
        item_rects.push((
            action,
            Rect::new(cursor + GAP, item_y, ITEM as u32, ITEM as u32),
        ));
        cursor += ITEM + GAP;
    }

    Dock {
        rect,
        items: item_rects,
        separator,
    }
}

pub fn hit(dock: &Dock, x: i32, y: i32) -> Option<DockAction> {
    let point = aether_geometry::Point::new(x, y);
    dock.items
        .iter()
        .find(|(_, rect)| rect.contains(point))
        .map(|(action, _)| *action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_is_centered_above_the_bottom_edge() {
        let dock = layout(Rect::new(0, 0, 1280, 800));
        assert_eq!(dock.rect.bottom(), 792);
        assert_eq!(dock.rect.height, DOCK_HEIGHT);
        let center = dock.rect.x + dock.rect.width as i32 / 2;
        assert!((center - 640).abs() <= 2, "center {center}");
        // The demo dock is 441 px wide at 1280.
        assert!(
            (dock.rect.width as i32 - 441).abs() <= 4,
            "width {}",
            dock.rect.width
        );
    }

    #[test]
    fn items_are_46_px_and_never_overlap() {
        let dock = layout(Rect::new(0, 0, 1280, 800));
        assert_eq!(dock.items.len(), 8);
        for (_, rect) in &dock.items {
            assert_eq!(rect.width, 46);
            assert_eq!(rect.height, 46);
        }
        for pair in dock.items.windows(2) {
            assert!(pair[0].1.right() as i32 <= pair[1].1.x);
        }
    }

    #[test]
    fn separator_sits_between_the_groups() {
        let dock = layout(Rect::new(0, 0, 1280, 800));
        let about = dock.items[5].1;
        let trash = dock.items[6].1;
        assert!(dock.separator.x > about.right() as i32);
        assert!((dock.separator.x as i64) < trash.x as i64);
    }

    #[test]
    fn hits_return_actions() {
        let dock = layout(Rect::new(0, 0, 1280, 800));
        let explorer = dock.items[0].1;
        assert_eq!(
            hit(&dock, explorer.x + 4, explorer.y + 4),
            Some(DockAction::Explorer)
        );
        assert_eq!(hit(&dock, 2, 2), None);
    }
}
