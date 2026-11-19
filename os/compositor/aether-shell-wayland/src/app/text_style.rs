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
        _ => font::HEADING_PX,
    }
}

/// Whether a scale renders bold. Legacy heading scales (2-6) are bold; the OS
/// scales (7-11) are regular unless the caller sets the bold bit.
pub fn text_bold(scale: u32) -> bool {
    let base = scale & 0xff;
    ((2..=6).contains(&base)) || (scale & 0x100) != 0
}

/// Letter spacing from the design system: `0.04em` on resource captions and
/// `-0.01em` on titles.
pub fn text_spacing(scale: u32) -> f32 {
    let base = scale & 0xff;
    let bold = text_bold(scale);
    match base {
        7 => 0.32,
        10 if bold => -0.13,
        _ => 0.0,
    }
}
