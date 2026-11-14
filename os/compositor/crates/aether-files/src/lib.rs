//! Bounded filename index for the Aether palette.
//!
//! This is a small, synchronous scan used to make files reachable from the
//! palette before the full system indexer is wired in. It walks configured
//! roots, skips hidden entries and heavy build directories, caps depth/entries
//! and sorts by name. It indexes names and paths only, not file contents.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// One file discovered by the scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    /// Parent directory relative to the scan root, for display.
    pub detail: String,
}

/// Walk `roots` and return at most `max_entries` files up to `max_depth`.
pub fn scan(roots: &[PathBuf], max_entries: usize, max_depth: usize) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for root in roots {
        walk(root, root, 0, max_depth, max_entries, &mut entries);
        if entries.len() >= max_entries {
            break;
        }
    }
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    entries
}

fn walk(
    root: &Path,
    directory: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    entries: &mut Vec<FileEntry>,
) {
    if depth > max_depth || entries.len() >= max_entries {
        return;
    }
    let Ok(read_dir) = fs::read_dir(directory) else {
        return;
    };
    for entry in read_dir.flatten() {
        if entries.len() >= max_entries {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            if !ignored_dir(&name) {
                walk(root, &path, depth + 1, max_depth, max_entries, entries);
            }
        } else if file_type.is_file() {
            let detail = path
                .parent()
                .and_then(|parent| parent.strip_prefix(root).ok())
                .map(|parent| parent.to_string_lossy().to_string())
                .unwrap_or_default();
            entries.push(FileEntry { name, path, detail });
        }
    }
}

fn ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "target" | "vendor" | "__pycache__" | "build" | "dist"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "aether-files-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("notes/deep")).unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("alpha.txt"), "a").unwrap();
        fs::write(root.join("notes/beta.md"), "b").unwrap();
        fs::write(root.join("notes/deep/gamma.rs"), "c").unwrap();
        fs::write(root.join(".secret"), "s").unwrap();
        fs::write(root.join(".hidden/inside.txt"), "h").unwrap();
        fs::write(root.join("target/debug/ignored.txt"), "i").unwrap();
        root
    }

    #[test]
    fn scans_nested_files_and_skips_noise() {
        let root = fixture();
        let entries = scan(std::slice::from_ref(&root), 100, 4);
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["alpha.txt", "beta.md", "gamma.rs"]);
        let beta = entries
            .iter()
            .find(|entry| entry.name == "beta.md")
            .unwrap();
        assert_eq!(beta.detail, "notes");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn respects_depth_and_entry_limits() {
        let root = fixture();
        let shallow = scan(std::slice::from_ref(&root), 100, 0);
        assert_eq!(shallow.len(), 1);

        let capped = scan(std::slice::from_ref(&root), 2, 4);
        assert_eq!(capped.len(), 2);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn handles_missing_roots() {
        assert!(scan(&[PathBuf::from("/definitely/missing/aether")], 10, 2).is_empty());
    }
}
