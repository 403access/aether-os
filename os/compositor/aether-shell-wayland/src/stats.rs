//! Newline-delimited frame statistics shared by winit and DRM.

use std::io::Write;
use std::path::Path;

use smithay::utils::{Physical, Rectangle};

/// Append one JSON line to a stats file. Errors are intentionally ignored:
/// instrumentation must never take the compositor down.
pub fn append(path: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Total pixel area of a damage list.
pub fn damage_area(damage: &[Rectangle<i32, Physical>]) -> i64 {
    damage
        .iter()
        .map(|rect| i64::from(rect.size.w.max(0)) * i64::from(rect.size.h.max(0)))
        .sum()
}
