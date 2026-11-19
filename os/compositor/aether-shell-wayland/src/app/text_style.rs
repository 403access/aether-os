//! Chrome type scale shared by the renderer and the design manifest.
//!
//! Scales 0-6 are the original integer sizes; 7-12 are the OS concept's
//! fractional sizes (CSS px after the 1.14 OS scale).

use aether_text::font;

/// Pixel size for a chrome text style.
pub fn text_px(scale: u32) -> f32 {
    match scale {
        0 => font::SMALL_PX,
        1 => font::BODY_PX,
        2 => font::TITLE_PX,
        3 => font::HEADING_PX,
        4 => font::TINY_PX,
        5 => font::MICRO_PX,
        6 => font::CAPTION_PX,
        7 => font::OS_CAPTION_PX,
        8 => font::OS_MICRO_PX,
        9 => font::OS_SMALL_PX,
        10 => font::OS_XS_PX,
        11 => font::OS_SM_PX,
        12 => font::OS_MENU_PX,
        13 => font::OS_LABEL_CAPS_PX,
        _ => font::HEADING_PX,
    }
}

/// Weight flags carried in the high bits of a text scale.
pub const WEIGHT_BOLD: u32 = 0x100;
pub const WEIGHT_SEMIBOLD: u32 = 0x200;
pub const WEIGHT_MEDIUM: u32 = 0x400;
pub const WEIGHT_650: u32 = 0x800;

/// Numeric weight for the variable font.
pub fn text_weight(scale: u32) -> u16 {
    if scale & WEIGHT_650 != 0 {
        650
    } else if scale & WEIGHT_MEDIUM != 0 {
        550
    } else if scale & WEIGHT_SEMIBOLD != 0 {
        600
    } else if scale & WEIGHT_BOLD != 0 {
        700
    } else {
        400
    }
}

/// Human-readable weight for scene/manifest output.
pub fn text_weight_label(scale: u32) -> &'static str {
    match text_weight(scale) {
        650 => "semibold",
        600 => "medium",
        550 => "medium",
        700 => "bold",
        _ => "regular",
    }
}

/// Whether a scale renders bold. Legacy heading scales (2-6) are bold; the OS
/// scales (7-11) are regular unless the caller sets a weight flag.
pub fn text_bold(scale: u32) -> bool {
    let base = scale & 0xff;
    ((2..=6).contains(&base)) || text_weight(scale) >= 600
}

/// Letter spacing from the design system: `0.04em` on resource captions and
/// `-0.01em` on titles.
pub fn text_spacing(scale: u32) -> f32 {
    let base = scale & 0xff;
    let bold = text_bold(scale);
    match base {
        7 => 0.32,
        10 if bold => -0.13,
        13 if bold => 0.82,
        _ => 0.0,
    }
}
