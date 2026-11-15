//! Visual top-centre shortcut dock.
//!
//! The dock is pure geometry so the renderer and hit-testing cannot drift.

use aether_geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopAction {
    Palette,
    Terminal,
    Files,
    Chat,
    Audio,
    Agents,
    Shortcuts,
}

impl TopAction {
    pub const ALL: [Self; 7] = [
        Self::Palette,
        Self::Terminal,
        Self::Files,
        Self::Chat,
        Self::Audio,
        Self::Agents,
        Self::Shortcuts,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Palette => "Palette",
            Self::Terminal => "Terminal",
            Self::Files => "Files",
            Self::Chat => "Chat",
            Self::Audio => "Audio",
            Self::Agents => "Agents",
            Self::Shortcuts => "Shortcuts",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Palette => "command",
            Self::Terminal => "terminal",
            Self::Files => "folder",
            Self::Chat => "chat",
            Self::Audio => "audio",
            Self::Agents => "agents",
            Self::Shortcuts => "keyboard",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopBar {
    pub rect: Rect,
    pub items: Vec<(TopAction, Rect)>,
    pub compact: bool,
}

pub const DOCK_HEIGHT: u32 = 34;
const GAP: i32 = 4;
const PAD: i32 = 5;

pub fn layout(bounds: Rect) -> TopBar {
    let compact = bounds.width < 900;
    let item_width = if compact { 38_i32 } else { 96 };
    let total = item_width * TopAction::ALL.len() as i32
        + GAP * (TopAction::ALL.len() as i32 - 1)
        + PAD * 2;
    let width = total.min(bounds.width.saturating_sub(24) as i32).max(1);
    let x = bounds.x + (bounds.width as i32 - width) / 2;
    let y = bounds.y + 7;
    let rect = Rect::new(x, y, width as u32, DOCK_HEIGHT);
    let items = TopAction::ALL
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            let item = Rect::new(
                x + PAD + index as i32 * (item_width + GAP),
                y + 3,
                item_width as u32,
                DOCK_HEIGHT - 6,
            );
            (action, item)
        })
        .collect();
    TopBar {
        rect,
        items,
        compact,
    }
}

pub fn hit(bar: &TopBar, x: i32, y: i32) -> Option<TopAction> {
    if !bar.rect.contains(aether_geometry::Point::new(x, y)) {
        return None;
    }
    bar.items
        .iter()
        .find(|(_, rect)| rect.contains(aether_geometry::Point::new(x, y)))
        .map(|(action, _)| *action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_is_centered_and_clickable() {
        let bounds = Rect::new(0, 0, 1280, 800);
        let bar = layout(bounds);
        assert!(!bar.compact);
        assert_eq!(bar.items.len(), 7);
        let center = bar.rect.x + bar.rect.width as i32 / 2;
        assert!((center - 640).abs() <= 2);
        let (action, rect) = bar.items[0];
        assert_eq!(action, TopAction::Palette);
        assert_eq!(hit(&bar, rect.x + 4, rect.y + 4), Some(action));
        assert_eq!(hit(&bar, 2, 2), None);
    }

    #[test]
    fn narrow_outputs_use_icons_only() {
        let bar = layout(Rect::new(0, 0, 800, 600));
        assert!(bar.compact);
        assert!(bar.rect.width < 340);
    }
}
