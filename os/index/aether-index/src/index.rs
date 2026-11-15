//! In-memory file index with bounded scan and ranked queries.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const IGNORED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".cache",
    "__pycache__",
    "proc",
    "sys",
    "dev",
    ".direnv",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub path: PathBuf,
    pub title: String,
    pub subtitle: String,
    /// Case-folded title, computed once. Query scoring and result ordering used
    /// to lowercase every title per query and per comparison.
    lower_title: String,
    haystack: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub title: String,
    pub subtitle: String,
    pub path: String,
}

#[derive(Debug)]
pub struct Index {
    documents: Vec<Document>,
    limit: usize,
}

impl Index {
    pub fn build(roots: &[PathBuf], limit: usize, depth: usize) -> Self {
        let mut index = Self {
            documents: Vec::new(),
            limit,
        };
        for root in roots {
            index.scan(root, depth);
            if index.documents.len() >= limit {
                break;
            }
        }
        index.documents.sort_by(|a, b| a.path.cmp(&b.path));
        index
    }

    fn scan(&mut self, root: &Path, depth: usize) {
        let mut stack = vec![(root.to_path_buf(), 0usize)];
        while let Some((dir, level)) = stack.pop() {
            if self.documents.len() >= self.limit {
                return;
            }
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            let mut children: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| !ignored(path))
                .collect();
            children.sort();
            for path in children {
                if self.documents.len() >= self.limit {
                    return;
                }
                if path.is_dir() {
                    if level < depth {
                        stack.push((path, level + 1));
                    }
                } else if path.is_file() {
                    self.documents.push(document(&path));
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn paths(&self) -> impl Iterator<Item = &std::path::Path> {
        self.documents
            .iter()
            .map(|document| document.path.as_path())
    }

    pub fn upsert(&mut self, path: &Path) -> bool {
        if ignored(path) || !path.is_file() {
            return false;
        }
        let doc = document(path);
        match self
            .documents
            .binary_search_by(|candidate| candidate.path.cmp(&doc.path))
        {
            Ok(index) => {
                if self.documents[index] == doc {
                    return false;
                }
                self.documents[index] = doc;
            }
            Err(index) if self.documents.len() < self.limit => {
                self.documents.insert(index, doc);
            }
            Err(_) => return false,
        }
        true
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        match self
            .documents
            .binary_search_by(|candidate| candidate.path.as_path().cmp(path))
        {
            Ok(index) => {
                self.documents.remove(index);
                true
            }
            Err(_) => false,
        }
    }

    pub fn query(&self, query: &str, limit: usize) -> Vec<Hit> {
        if limit == 0 {
            return Vec::new();
        }
        let query = query.trim().to_lowercase();
        let mut scored: Vec<(u8, &Document)> = self
            .documents
            .iter()
            .filter_map(|doc| score(&query, doc).map(|value| (value, doc)))
            .collect();
        if scored.len() > limit {
            // Selection is cheaper than sorting every match when the caller
            // only wants a palette page; sort the selected prefix afterwards.
            scored.select_nth_unstable_by(limit, ranked);
            scored.truncate(limit);
        }
        scored.sort_by(ranked);
        scored
            .into_iter()
            .map(|(_, doc)| Hit {
                title: doc.title.clone(),
                subtitle: doc.subtitle.clone(),
                path: doc.path.to_string_lossy().to_string(),
            })
            .collect()
    }
}

/// Ranking is stable by score, then case-folded title.
fn ranked(a: &(u8, &Document), b: &(u8, &Document)) -> std::cmp::Ordering {
    a.0.cmp(&b.0)
        .then_with(|| a.1.lower_title.cmp(&b.1.lower_title))
}

fn document(path: &Path) -> Document {
    let title = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let parent = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default();
    let subtitle = if extension.is_empty() {
        parent.clone()
    } else {
        format!("{parent}  ·  {extension}")
    };
    let lower_title = title.to_lowercase();
    Document {
        path: path.to_path_buf(),
        title,
        subtitle,
        lower_title,
        haystack: path.to_string_lossy().to_lowercase(),
    }
}

fn ignored(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| IGNORED.contains(&name) || name.starts_with('.'))
}

/// Prefix > word-prefix > substring > path substring > subsequence.
pub(crate) fn score(query: &str, doc: &Document) -> Option<u8> {
    let title = doc.lower_title.as_str();
    if query.is_empty() {
        return Some(0);
    }
    if title.starts_with(query) {
        return Some(0);
    }
    if title
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| !word.is_empty() && word.starts_with(query))
    {
        return Some(1);
    }
    if title.contains(query) {
        return Some(2);
    }
    if doc.haystack.contains(query) {
        return Some(3);
    }
    if subsequence(query, title) {
        return Some(4);
    }
    None
}

pub(crate) fn subsequence(query: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    query
        .chars()
        .all(|needle| chars.any(|candidate| candidate == needle))
}

/// Prefix modes shared with the palette layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    All,
    Command,
    Agent,
    File,
    Setting,
}

pub fn parse_mode(input: &str) -> (Mode, &str) {
    let trimmed = input.trim_start();
    match trimmed.chars().next() {
        Some('>') => (Mode::Command, trimmed[1..].trim()),
        Some('@') => (Mode::Agent, trimmed[1..].trim()),
        Some('/') => (Mode::File, trimmed[1..].trim()),
        Some('~') => (Mode::Setting, trimmed[1..].trim()),
        _ => (Mode::All, trimmed.trim()),
    }
}

/// Deduplicate paths while preserving order.
pub fn dedupe(paths: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("aether-index-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("alpha.txt"), "alpha").unwrap();
        fs::write(root.join("notes/beta.md"), "beta").unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/ignored.js"), "ignored").unwrap();
        root
    }

    #[test]
    fn scans_and_ignores_vendor_dirs() {
        let root = fixture("scan");
        let index = Index::build(std::slice::from_ref(&root), 100, 4);
        assert_eq!(index.len(), 2);
        assert!(index.query("alpha", 10)[0].path.ends_with("alpha.txt"));
        assert!(index.query("ignored", 10).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ranks_prefix_before_substring_and_subsequence() {
        let root = fixture("rank");
        let mut index = Index::build(std::slice::from_ref(&root), 100, 4);
        let path = root.join("car.txt");
        fs::write(&path, "car").unwrap();
        index.upsert(&path);
        assert!(index.query("car", 10)[0].title.starts_with("car"));
        assert!(index
            .query("ct", 10)
            .iter()
            .any(|hit| hit.title == "car.txt"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn limit_returns_the_prefix_of_the_full_ranking() {
        let root = fixture("limit");
        for name in ["bravo.txt", "charlie.txt", "delta.txt"] {
            fs::write(root.join(name), name).unwrap();
        }
        let index = Index::build(std::slice::from_ref(&root), 100, 4);
        let all = index.query("", 100);
        let page = index.query("", 2);
        assert_eq!(page.len(), 2);
        assert_eq!(
            page.iter().map(|hit| &hit.path).collect::<Vec<_>>(),
            all.iter().take(2).map(|hit| &hit.path).collect::<Vec<_>>()
        );
        assert!(index.query("", 0).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn case_folded_query_matches_mixed_case_titles() {
        let root = fixture("fold");
        fs::write(root.join("Bravo.md"), "bravo").unwrap();
        let index = Index::build(std::slice::from_ref(&root), 100, 4);
        assert_eq!(index.query("BRAVO", 5)[0].title, "Bravo.md");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_upsert_and_remove() {
        let root = fixture("live");
        let mut index = Index::build(std::slice::from_ref(&root), 100, 4);
        let path = root.join("new.md");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "new").unwrap();
        assert!(index.upsert(&path));
        assert_eq!(index.query("new", 10).len(), 1);
        fs::remove_file(&path).unwrap();
        assert!(index.remove(&path));
        assert!(index.query("new", 10).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_prefix_modes() {
        assert_eq!(parse_mode("/etc"), (Mode::File, "etc"));
        assert_eq!(parse_mode("  >run"), (Mode::Command, "run"));
        assert_eq!(parse_mode("plain"), (Mode::All, "plain"));
    }
}
