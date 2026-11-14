//! Minimal XDG desktop-entry discovery for the Aether palette.
//!
//! Scans `applications/*.desktop` below the standard data directories, parses
//! the fields the launcher needs and turns `Exec` into a program plus argument
//! vector. It is deliberately small: no icon parsing, localization or terminal
//! emulation. Entries that require a terminal, are hidden, or are not
//! `Type=Application` are skipped.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// An application discovered from a desktop entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopEntry {
    /// Desktop-file id relative to `applications/`, e.g. `org.example.App.desktop`.
    pub id: String,
    pub name: String,
    pub comment: String,
    pub icon: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub program: String,
    pub args: Vec<String>,
    /// The entry asked for a terminal (`Terminal=true`). Callers may skip it.
    pub terminal: bool,
}

/// Parse one desktop file. Returns `None` for non-applications, hidden entries
/// or files missing `Name`/`Exec`.
pub fn parse(id: &str, contents: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let mut name = None;
    let mut comment = String::new();
    let mut exec = None;
    let mut icon = None;
    let mut is_application = false;
    let mut no_display = false;
    let mut hidden = false;
    let mut terminal = false;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" => name = Some(value.to_string()),
            "Comment" => comment = value.to_string(),
            "Exec" => exec = Some(value.to_string()),
            "Icon" => icon = Some(value.to_string()),
            "Type" => is_application = value == "Application",
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    if !is_application || no_display || hidden || terminal {
        return None;
    }
    let name = name?;
    let exec = exec?;
    let mut argv = split_exec(&exec);
    let program = argv.first()?.clone();
    argv.remove(0);
    Some(DesktopEntry {
        id: id.to_string(),
        name,
        comment,
        icon,
        icon_path: None,
        program,
        args: argv,
        terminal,
    })
}

/// Split an `Exec` value into arguments, handling simple single/double quoting
/// and backslash escapes, then dropping XDG field-code arguments (`%f`, `%U`,
/// ...) because the palette has no file/URL context.
pub fn split_exec(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = exec.chars().peekable();

    while let Some(character) = chars.next() {
        match quote {
            Some(delimiter) => {
                if character == delimiter {
                    quote = None;
                } else if character == '\\' && delimiter == '"' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else {
                    current.push(character);
                }
            }
            None => match character {
                '\'' | '"' => quote = Some(character),
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(character),
            },
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
        .into_iter()
        .filter(|token| !is_field_code(token))
        .map(|token| token.replace("%%", "%"))
        .collect()
}

fn is_field_code(token: &str) -> bool {
    matches!(
        token,
        "%f" | "%F" | "%u" | "%U" | "%i" | "%c" | "%k" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m"
    )
}

/// Discover entries below `<dir>/applications` for each data directory.
/// Earlier directories take precedence; results are sorted by name.
pub fn discover(data_dirs: &[PathBuf]) -> Vec<DesktopEntry> {
    let mut entries: BTreeMap<String, DesktopEntry> = BTreeMap::new();
    for dir in data_dirs.iter().rev() {
        let applications = dir.join("applications");
        walk(&applications, &applications, &mut entries);
    }
    let mut found: Vec<DesktopEntry> = entries.into_values().collect();
    for entry in &mut found {
        entry.icon_path = entry
            .icon
            .as_deref()
            .and_then(|icon| find_icon(icon, data_dirs));
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found
}

fn find_icon(name: &str, data_dirs: &[PathBuf]) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.is_absolute() && direct.is_file() {
        return Some(direct.to_path_buf());
    }
    const SIZES: [&str; 5] = ["48x48", "32x32", "64x64", "128x128", "256x256"];
    for dir in data_dirs {
        for size in SIZES {
            let path = dir
                .join("icons/hicolor")
                .join(size)
                .join("apps")
                .join(format!("{name}.png"));
            if path.is_file() {
                return Some(path);
            }
        }
        let path = dir.join("pixmaps").join(format!("{name}.png"));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn walk(root: &Path, current: &Path, entries: &mut BTreeMap<String, DesktopEntry>) {
    let Ok(read_dir) = fs::read_dir(current) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, entries);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let id = path
            .strip_prefix(root)
            .ok()
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        if let Some(parsed) = parse(&id, &contents) {
            entries.insert(id, parsed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY: &str = "\
[Desktop Entry]
Type=Application
Name=Example Editor
Comment=Edit things
Exec=example-editor --new %U
Terminal=false
";

    #[test]
    fn parses_an_application_entry() {
        let entry = parse("example.desktop", ENTRY).expect("entry");
        assert_eq!(entry.name, "Example Editor");
        assert_eq!(entry.comment, "Edit things");
        assert_eq!(entry.program, "example-editor");
        assert_eq!(entry.args, ["--new"]);
        assert_eq!(entry.icon, None);
        assert!(!entry.terminal);
    }

    #[test]
    fn skips_hidden_non_apps_and_terminal_entries() {
        assert!(parse("x.desktop", "[Desktop Entry]\nType=Link\nName=X\nExec=x\n").is_none());
        assert!(parse(
            "x.desktop",
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n"
        )
        .is_none());
        assert!(parse(
            "x.desktop",
            "[Desktop Entry]\nType=Application\nName=X\nExec=htop\nTerminal=true\n"
        )
        .is_none());
        assert!(parse(
            "x.desktop",
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nHidden=true\n"
        )
        .is_none());
    }

    #[test]
    fn only_reads_the_desktop_entry_group() {
        let contents = "\
[Desktop Action New]
Name=Wrong
Exec=wrong

[Desktop Entry]
Type=Application
Name=Right
Exec=right
";
        let entry = parse("x.desktop", contents).expect("entry");
        assert_eq!(entry.name, "Right");
        assert_eq!(entry.program, "right");
    }

    #[test]
    fn splits_quoted_exec_arguments() {
        assert_eq!(
            split_exec(r#"app --title "Hello World" 'second arg' plain"#),
            ["app", "--title", "Hello World", "second arg", "plain"]
        );
        assert_eq!(split_exec("app 100%"), ["app", "100%"]);
        assert_eq!(split_exec("app %%"), ["app", "%"]);
        assert_eq!(split_exec("app %f %U"), ["app"]);
        assert_eq!(split_exec("app --path=%f"), ["app", "--path=%f"]);
    }

    #[test]
    fn discovers_and_dedupes_data_dirs() {
        let root = std::env::temp_dir().join(format!(
            "aether-desktop-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let high = root.join("high");
        let low = root.join("low");
        fs::create_dir_all(high.join("applications")).unwrap();
        fs::create_dir_all(low.join("applications")).unwrap();
        fs::write(
            high.join("applications/one.desktop"),
            "[Desktop Entry]\nType=Application\nName=High One\nExec=high-one\n",
        )
        .unwrap();
        fs::write(
            low.join("applications/one.desktop"),
            "[Desktop Entry]\nType=Application\nName=Low One\nExec=low-one\n",
        )
        .unwrap();
        fs::write(
            low.join("applications/two.desktop"),
            "[Desktop Entry]\nType=Application\nName=Two\nExec=two\nIcon=two-icon\n",
        )
        .unwrap();
        let icon_dir = low.join("icons/hicolor/32x32/apps");
        fs::create_dir_all(&icon_dir).unwrap();
        let icon_path = icon_dir.join("two-icon.png");
        fs::write(&icon_path, b"png").unwrap();
        fs::write(
            low.join("applications/skip.desktop"),
            "[Desktop Entry]\nType=Application\nName=Skip\nExec=skip\nNoDisplay=true\n",
        )
        .unwrap();

        let entries = discover(&[high, low]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "High One");
        assert_eq!(entries[0].program, "high-one");
        assert_eq!(entries[1].name, "Two");
        assert_eq!(entries[1].icon.as_deref(), Some("two-icon"));
        assert_eq!(entries[1].icon_path.as_deref(), Some(icon_path.as_path()));
        fs::remove_dir_all(root).ok();
    }
}
