//! The Water OS menu bar: brand, application menus, system status and tray.
//!
//! This replaces the original top shortcut dock. It is passive chrome like the
//! desktop's own bar, but every item is a real target: menus open popovers,
//! the clock opens the Calendar app, the resource group opens the task manager
//! and the tray mirrors the audio overlay, theme settings and launcher.

mod clock;
mod layout;

pub(crate) use clock::{Clock, WEEKDAY_HEADERS};
pub(crate) use layout::{
    hit, menu_hit, menu_layout, MenuCommand, MenuId, MenuPanel, Menubar, MenubarAction,
    ResourceMeter, TrayAction, MENUBAR_HEIGHT,
};

use aether_geometry::Point as AetherPoint;
use aether_shell::OutputId;

use super::super::system::SystemApp;
use super::super::App;

/// Label on the sync pill. The index worker owns the real state; the pill
/// mirrors its presence rather than inventing a percentage.
const SYNC_LABEL: &str = "Synced";

impl App {
    pub(crate) fn menubar(&self, output: OutputId) -> Menubar {
        let bounds = self.shell.output_bounds(output);
        let clock = Clock::now();
        let status = self.status();
        let cpu = status.cpu_percent.min(100) as f32 / 100.0;
        let memory = if status.mem_total_mb > 0 {
            status.mem_used_mb as f32 / status.mem_total_mb as f32
        } else {
            0.0
        };
        let meters = [
            ResourceMeter {
                label: "CPU".into(),
                value: format!("{}%", status.cpu_percent),
                fraction: cpu,
            },
            ResourceMeter {
                label: "RAM".into(),
                value: format!("{}%", (memory * 100.0).round() as i64),
                fraction: memory,
            },
            ResourceMeter {
                label: "STORAGE".into(),
                value: "—".into(),
                fraction: 0.0,
            },
        ];
        let title = self.menubar_title();
        let measure = |text: &str, scale: u32| self.text_width(text, scale);
        let bar = layout::layout(
            bounds,
            &measure,
            &title,
            SYNC_LABEL,
            &meters,
            &clock.short_date(),
            &clock.time_label(),
        );
        if std::env::var_os("AETHER_MENUBAR_DEBUG").is_some() {
            use std::sync::atomic::{AtomicBool, Ordering};
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                println!(
                    "aether-shell: menubar app={:?} menus={:?} sync={:?} resources={:?} clock={:?} traps={:?}",
                    bar.app, bar.menus, bar.sync, bar.resources, bar.clock, bar.tray
                );
            }
        }
        bar
    }

    /// The active application name: the frontmost overlay, else the focused
    /// client window, else the shell itself.
    fn menubar_title(&self) -> String {
        if self.calendar_open() {
            return "Calendar".to_string();
        }
        if self.shell.explorer_open() {
            return "Files".to_string();
        }
        if self.system_open() {
            return self
                .system
                .app
                .map(|app| app.title().to_string())
                .unwrap_or_else(|| "Aether OS".to_string());
        }
        if self.shell.themes_open() {
            return "Water settings".to_string();
        }
        if self.shell.audio_open() {
            return "Audio".to_string();
        }
        if self.task_manager_open() {
            return "Task Manager".to_string();
        }
        if self.shell.overlay().is_some() {
            return "Aether OS".to_string();
        }
        self.shell
            .focused_window()
            .and_then(|id| {
                self.shell
                    .windows()
                    .into_iter()
                    .find(|window| window.id == id)
                    .map(|window| window.title.clone())
            })
            .unwrap_or_else(|| "Aether OS".to_string())
    }

    /// The bar is chrome: fullscreen clients hide it so they get every pixel.
    pub(crate) fn menubar_visible(&self, output: OutputId) -> bool {
        self.shell.fullscreen_window_on(output).is_none()
    }

    pub(crate) fn menubar_action_at(&self, point: AetherPoint) -> Option<MenubarAction> {
        let output = self.shell.output_at(point)?;
        if !self.menubar_visible(output) {
            return None;
        }
        hit(&self.menubar(output), point.x, point.y)
    }

    pub(crate) fn menubar_menu_panel(&self, output: OutputId, menu: MenuId) -> MenuPanel {
        let bar = self.menubar(output);
        let measure = |text: &str, scale: u32| self.text_width(text, scale);
        menu_layout(&bar, menu, &measure)
    }

    /// Menu command under the pointer, if a popover is open.
    pub(crate) fn menubar_command_at(&self, point: AetherPoint) -> Option<MenuCommand> {
        let menu = self.menu_open?;
        let output = self.shell.output_at(point)?;
        let panel = self.menubar_menu_panel(output, menu);
        menu_hit(&panel, point.x, point.y)
    }

    pub(crate) fn toggle_menu(&mut self, menu: MenuId) {
        self.menu_open = (self.menu_open != Some(menu)).then_some(menu);
        self.mark_full_damage();
    }

    pub(crate) fn close_menu(&mut self) {
        if self.menu_open.take().is_some() {
            self.mark_full_damage();
        }
    }

    pub(crate) fn activate_menubar(&mut self, action: MenubarAction) {
        match action {
            MenubarAction::Logo => {
                self.close_menu();
                self.open_system(SystemApp::About);
            }
            MenubarAction::App => {}
            MenubarAction::Menu(menu) => self.toggle_menu(menu),
            MenubarAction::Tray(tray) => {
                self.close_menu();
                self.activate_tray(tray);
            }
        }
    }

    fn activate_tray(&mut self, action: TrayAction) {
        match action {
            TrayAction::Sync | TrayAction::Resources => {
                if self.task_manager_open() {
                    self.close_task_manager();
                } else {
                    self.open_task_manager();
                }
            }
            TrayAction::Network => self.toggle_tray(crate::app::tray::TrayId::Network),
            TrayAction::Devices => self.toggle_tray(crate::app::tray::TrayId::Devices),
            TrayAction::Battery => self.toggle_tray(crate::app::tray::TrayId::Battery),
            TrayAction::Sound => {
                self.toggle_tray(crate::app::tray::TrayId::Sound);
            }
            TrayAction::Clock => {
                self.toggle_tray(crate::app::tray::TrayId::Calendar);
            }
            TrayAction::Appearance => {
                if self.shell.toggle_theme_mode() {
                    self.apply_theme_action(Some(aether_shell::theme::ThemeAction::Changed));
                }
            }
            TrayAction::Gallery => {
                println!("aether-shell: tray gallery");
            }
        }
    }

    pub(crate) fn activate_menu_command(&mut self, command: MenuCommand) {
        self.close_menu();
        match command {
            MenuCommand::NewTerminal => {
                self.spawn_terminal();
                println!("aether-shell: menu terminal");
            }
            MenuCommand::Explorer => self.toggle_explorer(),
            MenuCommand::Calendar => self.open_calendar(),
            MenuCommand::TaskManager => {
                if self.task_manager_open() {
                    self.close_task_manager();
                } else {
                    self.open_task_manager();
                }
            }
            MenuCommand::Themes => self.toggle_themes(),
            MenuCommand::About => self.open_system(SystemApp::About),
            MenuCommand::Network => self.open_system(SystemApp::Network),
            MenuCommand::Battery => self.open_system(SystemApp::Battery),
            MenuCommand::Devices => self.open_system(SystemApp::Devices),
            MenuCommand::Radio => self.open_system(SystemApp::Radio),
            MenuCommand::Studio => self.open_system(SystemApp::Studio),
            MenuCommand::Compare => self.open_system(SystemApp::Compare),
            MenuCommand::Shortcuts => self.toggle_shortcuts(),
            MenuCommand::CloseWindow => {
                if let Some(id) = self.shell.focused_window() {
                    if let Some(toplevel) = self.windows.get(&id) {
                        toplevel.send_close();
                    }
                    println!("aether-shell: menu close window id={id}");
                }
            }
            MenuCommand::ToggleTheme => {
                if self.shell.toggle_theme_mode() {
                    self.apply_theme_action(Some(aether_shell::theme::ThemeAction::Changed));
                }
            }
        }
    }
}
