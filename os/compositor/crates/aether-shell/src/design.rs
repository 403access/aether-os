//! Compositor design tokens mirrored from `packages/design-system/tokens.css`.
//!
//! Values are 0xRRGGBB; the renderer converts them with sRGB alpha.
//! `scripts/design-parity.ts` fails when these drift from the web tokens.

pub const BG: u32 = 0x031922;
pub const PANEL: u32 = 0x072733;
pub const HEADER: u32 = 0x0D3D4B;
pub const ACCENT: u32 = 0x38D6FF;
pub const TEXT: u32 = 0xE9FBFF;
pub const TEXT_DIM: u32 = 0xA5DBE5;

/// Reference explorer rows are 30 px (`packages/ui/src/ui.css`).
pub const EXPLORER_ROW_HEIGHT: u32 = 30;
pub const EXPLORER_VISIBLE_ROWS: usize = 14;

/// Audio overlay geometry (660 x 560, four outputs + three inputs).
pub const AUDIO_PANEL_WIDTH: u32 = 660;
pub const AUDIO_PANEL_HEIGHT: u32 = 560;
pub const AUDIO_ROW_HEIGHT: u32 = 40;
pub const AUDIO_ROW_SPACING: u32 = 46;
pub const AUDIO_ROWS_TOP: i32 = 94;
pub const AUDIO_SECTION_GAP: i32 = 28;
