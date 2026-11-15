//! Palette entries and process launching.
//!
//! Desktop files, an optional external palette feed and search-index results
//! are merged into the shell's launcher/file providers; activated entries are
//! spawned with the shell's Wayland socket in their environment.

use aether_shell::{parse_palette_query, PaletteFile, PaletteLauncher, PaletteMode};

use crate::agents::parse_launchers;
use crate::index_worker::should_apply;

use super::App;

fn desktop_data_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(std::path::PathBuf::from(data_home));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home).join(".local/share"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/share:/usr/share"));
    dirs.extend(std::env::split_paths(&data_dirs));
    dirs.dedup();
    dirs
}

fn file_roots() -> Vec<std::path::PathBuf> {
    if let Some(roots) = std::env::var_os("AETHER_FILE_ROOTS") {
        return std::env::split_paths(&roots).collect();
    }
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .into_iter()
        .collect()
}

pub(crate) fn file_index() -> Vec<PaletteFile> {
    let limit = std::env::var("AETHER_FILE_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000usize);
    let depth = std::env::var("AETHER_FILE_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5usize);
    aether_files::scan(&file_roots(), limit, depth)
        .into_iter()
        .map(|entry| PaletteFile {
            name: entry.name,
            detail: if entry.detail.is_empty() {
                "file".to_string()
            } else {
                entry.detail
            },
            path: entry.path.to_string_lossy().to_string(),
        })
        .collect()
}

fn external_launchers() -> Vec<PaletteLauncher> {
    let Some(path) = std::env::var_os("AETHER_PALETTE_JSON") else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return Vec::new();
    };
    parse_launchers(&value)
}

fn desktop_launchers() -> Vec<PaletteLauncher> {
    aether_desktop::discover(&desktop_data_dirs())
        .into_iter()
        .flat_map(|entry| {
            let detail = if entry.comment.is_empty() {
                "application".to_string()
            } else {
                entry.comment.clone()
            };
            let icon = entry
                .icon_path
                .map(|path| path.to_string_lossy().to_string());
            let mut launchers = vec![PaletteLauncher {
                name: entry.name.clone(),
                detail,
                icon: icon.clone(),
                program: entry.program.clone(),
                args: entry.args.clone(),
            }];
            for action in entry.actions {
                launchers.push(PaletteLauncher {
                    name: format!("{} - {}", entry.name, action.name),
                    detail: "action".to_string(),
                    icon: icon.clone(),
                    program: action.program,
                    args: action.args,
                });
            }
            launchers
        })
        .collect()
}

impl App {
    pub(crate) fn combined_launchers(&self) -> Vec<PaletteLauncher> {
        let mut launchers = desktop_launchers();
        launchers.extend(external_launchers());
        launchers.extend(self.agent_sessions.launchers().iter().cloned());
        launchers
    }

    pub(crate) fn refresh_launchers(&mut self) {
        let launchers = self.combined_launchers();
        self.shell.set_launchers(launchers);
    }

    pub(crate) fn refresh_providers(&mut self) {
        let launchers = self.combined_launchers();
        let files = if self.index_socket.is_some() {
            self.index_query.clear();
            Vec::new()
        } else {
            file_index()
        };
        println!(
            "aether-shell-smithay: reloaded {} desktop apps, {} files",
            launchers.len(),
            files.len()
        );
        self.shell.set_launchers(launchers);
        self.shell.set_files(files);
    }

    pub(crate) fn refresh_index_results(&mut self) {
        if self.index_socket.is_none() || self.shell.overlay().is_none() {
            return;
        }
        let (mode, term) = parse_palette_query(self.shell.palette_query());
        if !matches!(mode, PaletteMode::All | PaletteMode::File) {
            return;
        }
        if term == self.index_query {
            return;
        }
        self.index_query = term.clone();
        if term.is_empty() {
            if let Some(worker) = self.index_worker.as_mut() {
                worker.cancel();
            }
            self.shell.set_indexed_files(Vec::new());
            self.mark_full_damage();
            return;
        }
        if let Some(worker) = self.index_worker.as_mut() {
            let generation = worker.request(term);
            println!("aether-shell: index query generation {generation}");
        }
    }

    /// Drain the latest index result without blocking. Results for superseded
    /// queries are discarded; identical result sets do not recompose.
    pub fn poll_index_results(&mut self) {
        let Some((generation, files)) = self.index_worker.as_ref().and_then(|worker| worker.poll())
        else {
            return;
        };
        let current = self
            .index_worker
            .as_ref()
            .map(|worker| worker.current_generation())
            .unwrap_or(0);
        if !should_apply(generation, current) {
            return;
        }
        if self.shell.files() == files.as_slice() {
            return;
        }
        println!(
            "aether-shell: index results generation {generation}: {} rows",
            files.len()
        );
        self.shell.set_indexed_files(files);
        self.mark_overlay_damage();
    }

    pub(crate) fn spawn_terminal(&mut self) {
        let program =
            std::env::var("AETHER_TERMINAL").unwrap_or_else(|_| "aether-terminal".to_string());
        self.spawn_program(&program, &[], "terminal");
    }

    pub(crate) fn spawn_launcher(&mut self, launcher: PaletteLauncher) {
        self.spawn_program(&launcher.program, &launcher.args, &launcher.name);
    }

    pub(crate) fn open_path(&self, path: &str) {
        let opener = std::env::var("AETHER_OPENER").unwrap_or_else(|_| "xdg-open".to_string());
        let mut command = std::process::Command::new(&opener);
        command.arg(path);
        command.env("WAYLAND_DISPLAY", &self.socket_name);
        match command.spawn() {
            Ok(child) => println!("aether-shell-smithay: opened {path} (pid {})", child.id()),
            Err(error) => {
                eprintln!("aether-shell-smithay: opener {opener:?} failed for {path:?}: {error}")
            }
        }
    }

    pub(crate) fn spawn_program(&self, program: &str, args: &[String], label: &str) {
        let mut command = std::process::Command::new(program);
        command.args(args);
        command.env("WAYLAND_DISPLAY", &self.socket_name);
        match command.spawn() {
            Ok(child) => println!(
                "aether-shell-smithay: spawned app {label} (pid {})",
                child.id()
            ),
            Err(error) => {
                eprintln!("aether-shell-smithay: app {label} ({program:?}) failed: {error}")
            }
        }
    }
}
