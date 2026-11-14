//! Xcursor theme loading.
//!
//! Parses the Xcursor file format (`Xcur`) into the same `CursorBitmap` used by
//! the built-in shapes. This keeps cursor rendering backend-neutral: the winit
//! adapter uploads the pixels, and a DRM backend can do the same.

use std::path::{Path, PathBuf};

use crate::{CursorBitmap, CursorShape};

const IMAGE_TYPE: u32 = 0xfffd_0002;

/// Xcursor theme search roots and preferred theme names.
#[derive(Clone, Debug, Default)]
pub struct CursorTheme {
    roots: Vec<PathBuf>,
    themes: Vec<String>,
}

impl CursorTheme {
    /// Discover themes from `AETHER_CURSOR_THEME`, `XCURSOR_THEME`,
    /// `XCURSOR_PATH` and the standard icon directories.
    pub fn discover() -> Self {
        let mut roots = Vec::new();
        if let Some(path) = std::env::var_os("XCURSOR_PATH") {
            roots.extend(std::env::split_paths(&path));
        }
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(&home).join(".icons"));
            roots.push(PathBuf::from(&home).join(".local/share/icons"));
        }
        roots.push(PathBuf::from("/usr/share/icons"));
        roots.push(PathBuf::from("/run/current-system/sw/share/icons"));
        roots.push(PathBuf::from("/usr/local/share/icons"));
        roots.dedup();

        let mut themes = Vec::new();
        for key in ["AETHER_CURSOR_THEME", "XCURSOR_THEME"] {
            if let Some(theme) = std::env::var_os(key) {
                themes.push(theme.to_string_lossy().to_string());
            }
        }
        for theme in ["default", "Adwaita", "hicolor", "breeze"] {
            themes.push(theme.to_string());
        }
        themes.dedup();
        Self { roots, themes }
    }

    /// Build a theme with explicit roots (used by tests and embedders).
    pub fn from_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            themes: vec!["default".to_string(), "Adwaita".to_string()],
        }
    }

    /// Load one shape at the requested nominal pixel size.
    pub fn load(&self, shape: CursorShape, nominal: u32) -> Option<CursorBitmap> {
        for root in &self.roots {
            for theme in &self.themes {
                for name in shape.cursor_names() {
                    let path = theme_path(root, theme, name);
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Some(bitmap) = parse_xcursor(&bytes, nominal) {
                            return Some(bitmap);
                        }
                    }
                }
            }
            for name in shape.cursor_names() {
                let path = root.join("cursors").join(name);
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Some(bitmap) = parse_xcursor(&bytes, nominal) {
                        return Some(bitmap);
                    }
                }
            }
        }
        None
    }
}

fn theme_path(root: &Path, theme: &str, name: &str) -> PathBuf {
    if theme.is_empty() {
        root.join("cursors").join(name)
    } else {
        root.join(theme).join("cursors").join(name)
    }
}

/// Parse an Xcursor file, choosing the image nearest `nominal` pixels.
pub fn parse_xcursor(bytes: &[u8], nominal: u32) -> Option<CursorBitmap> {
    if bytes.len() < 16 || &bytes[0..4] != b"Xcur" {
        return None;
    }
    let read_u32 = |offset: usize| -> Option<u32> {
        bytes
            .get(offset..offset + 4)
            .map(|slice| u32::from_le_bytes(slice.try_into().expect("four bytes")))
    };
    let header = read_u32(4)? as usize;
    let count = read_u32(12)? as usize;
    let mut best: Option<(u32, CursorBitmap)> = None;

    for index in 0..count {
        let entry = header.checked_add(index.checked_mul(12)?)?;
        let kind = read_u32(entry)?;
        if kind != IMAGE_TYPE {
            continue;
        }
        let subtype = read_u32(entry + 4)?;
        let position = read_u32(entry + 8)? as usize;
        let chunk_header = read_u32(position)? as usize;
        let width = read_u32(position + 16)?;
        let height = read_u32(position + 20)?;
        let hotspot_x = read_u32(position + 24)?;
        let hotspot_y = read_u32(position + 28)?;
        let pixels_start = position.checked_add(chunk_header)?;
        let count = (width as usize).checked_mul(height as usize)?;
        let pixels_end = pixels_start.checked_add(count.checked_mul(4)?)?;
        let raw = bytes.get(pixels_start..pixels_end)?;
        let mut pixels = Vec::with_capacity(count * 4);
        for argb in raw.as_chunks::<4>().0 {
            let value = u32::from_le_bytes(*argb);
            pixels.extend_from_slice(&[
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
                ((value >> 24) & 0xff) as u8,
            ]);
        }
        let bitmap = CursorBitmap {
            width,
            height,
            hotspot_x: hotspot_x as i32,
            hotspot_y: hotspot_y as i32,
            pixels,
        };
        let distance = subtype.abs_diff(nominal);
        if best
            .as_ref()
            .is_none_or(|(best_distance, _)| distance < *best_distance)
        {
            best = Some((distance, bitmap));
        }
    }

    best.map(|(_, bitmap)| bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xcursor_file(width: u32, height: u32) -> Vec<u8> {
        let image_size = 36 + (width * height * 4) as usize;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"Xcur");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&0x0001_0000u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&IMAGE_TYPE.to_le_bytes());
        bytes.extend_from_slice(&32u32.to_le_bytes());
        bytes.extend_from_slice(&28u32.to_le_bytes());
        bytes.extend_from_slice(&36u32.to_le_bytes());
        bytes.extend_from_slice(&IMAGE_TYPE.to_le_bytes());
        bytes.extend_from_slice(&32u32.to_le_bytes());
        bytes.extend_from_slice(&0x0001_0000u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        for index in 0..width * height {
            let color: u32 = if index == 0 { 0xffff_ffff } else { 0xff00_00ff };
            bytes.extend_from_slice(&color.to_le_bytes());
        }
        assert_eq!(bytes.len(), 28 + image_size);
        bytes
    }

    #[test]
    fn parses_an_image_and_hotspot() {
        let bitmap = parse_xcursor(&xcursor_file(2, 2), 32).expect("bitmap");
        assert_eq!(bitmap.width, 2);
        assert_eq!(bitmap.height, 2);
        assert_eq!((bitmap.hotspot_x, bitmap.hotspot_y), (1, 2));
        assert_eq!(&bitmap.pixels[0..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_xcursor(b"not an xcursor file", 32).is_none());
        assert!(parse_xcursor(b"Xcur", 32).is_none());
    }
}
