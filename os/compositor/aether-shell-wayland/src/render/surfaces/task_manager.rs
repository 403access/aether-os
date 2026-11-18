//! Task Manager surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::helpers::{format_rate, format_uptime, truncate};

impl App {
    pub(crate) fn compose_task_manager(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill_soft = ctx.panel_fill_soft;
        let panel_border_soft = ctx.panel_border_soft;
        let task_here = ctx.flags.task;
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if task_here {
            let bounds = self.task_rect();
            let pane = self.task_pane_rect();
            let width = bounds.width as i32;
            let height = bounds.height as i32;
            let (tab, selected, scroll, snapshot, process_rows, users, collapsed) = {
                let state = self.task_manager.as_ref().expect("task manager is open");
                let snapshot = state.snapshot().clone();
                let process_rows: Vec<(usize, aether_system::Process)> = state
                    .process_rows()
                    .into_iter()
                    .map(|(depth, process)| (depth, process.clone()))
                    .collect();
                let users = snapshot.users.clone();
                (
                    state.tab(),
                    state.selected(),
                    state.scroll(),
                    snapshot,
                    process_rows,
                    users,
                    state.collapsed().clone(),
                )
            };
            let visible = self.task_visible_rows();
            // First-party window chrome.
            if let Some(buffer) = self.chrome_round(
                bounds.width.max(1),
                bounds.height.max(1),
                14,
                [7, 39, 51, 235],
            ) {
                panels.push((buffer, bounds.x, bounds.y));
            }
            if let Some(buffer) =
                self.chrome_round_top(bounds.width.max(1), 40, 14, [10, 51, 66, 255])
            {
                panels.push((buffer, bounds.x, bounds.y));
            }
            if let Some(buffer) = self.chrome_outline(
                bounds.width.max(1),
                bounds.height.max(1),
                14,
                [
                    tokens.accent(255)[0],
                    tokens.accent(255)[1],
                    tokens.accent(255)[2],
                    120,
                ],
            ) {
                panels.push((buffer, bounds.x, bounds.y));
            }
            for (index, color) in [
                [0xff, 0x8a, 0x92, 255],
                [0xff, 0xd2, 0x7a, 255],
                [0x5e, 0xf2, 0xb0, 255],
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(buffer) = self.chrome_round(11, 11, 5, color) {
                    panels.push((buffer, bounds.x + 11 + index as i32 * 17, bounds.y + 14));
                }
            }
            if let Some(buffer) = self.chrome_round(23, 23, 5, tokens.accent(30)) {
                panels.push((buffer, bounds.x + 66, bounds.y + 8));
            }
            display_icons.push((
                "chart".into(),
                13,
                tokens.accent(255),
                bounds.x + 71,
                bounds.y + 13,
            ));
            labels.push((
                "Task Manager".into(),
                bounds.x + 97,
                bounds.y + 13,
                0x100,
                tokens.text,
            ));
            labels.push((
                "Processes and resources".into(),
                bounds.x + 97 + self.text_width("Task Manager", 1) as i32 + 8,
                bounds.y + 13,
                0,
                tokens.dim,
            ));
            let badge = format!("{} tasks", snapshot.process_count);
            let badge_w = 8 + 6 + 6 + self.text_width(&badge, 0) + 8;
            let badge_x = bounds.x + width - 12 - badge_w as i32;
            if let Some(buffer) = self.chrome_round(badge_w, 22, 11, tokens.green_a(28)) {
                panels.push((buffer, badge_x, bounds.y + 9));
            }
            if let Some(buffer) = self.chrome_round(6, 6, 3, tokens.green) {
                panels.push((buffer, badge_x + 8, bounds.y + 17));
            }
            labels.push((badge, badge_x + 20, bounds.y + 14, 0, tokens.green));
            if let Some(buffer) =
                self.chrome_round_bottom(bounds.width.max(1), 32, 13, tokens.surface_a(242))
            {
                panels.push((buffer, bounds.x, bounds.y + height - 32));
            }
            if let Some(buffer) = self.chrome_panel(
                pane.width,
                pane.height,
                12,
                panel_fill_soft,
                panel_border_soft,
            ) {
                panels.push((buffer, pane.x - 18, pane.y - 18));
            }
            // Tabs inside the window body.
            let tabs_x = bounds.x + 16;
            for (index, tab_item) in crate::app::overlays::task_manager::TaskTab::ALL
                .into_iter()
                .enumerate()
            {
                let tab_rect = AetherRect::new(tabs_x + index as i32 * 116, bounds.y + 50, 110, 30);
                if tab_item == tab {
                    if let Some(buffer) = self.chrome_round(110, 30, 8, tokens.accent(38)) {
                        panels.push((buffer, tab_rect.x, tab_rect.y));
                    }
                }
                labels.push((
                    tab_item.label().into(),
                    tab_rect.x + 14,
                    tab_rect.y + 8,
                    1,
                    if tab_item == tab {
                        tokens.text
                    } else {
                        tokens.soft
                    },
                ));
            }
            // Pane header.
            display_icons.push((
                "chart".into(),
                20,
                tokens.accent(255),
                pane.x + 16,
                pane.y + 14,
            ));
            labels.push((tab.label().into(), pane.x + 46, pane.y + 12, 3, tokens.text));
            let subtitle = match tab {
                crate::app::overlays::task_manager::TaskTab::Processes => format!(
                    "{} processes · {} threads · {:.0}% cpu",
                    snapshot.process_count, snapshot.thread_count, snapshot.cpu_percent
                ),
                crate::app::overlays::task_manager::TaskTab::Users => {
                    format!("{} users", users.len())
                }
                crate::app::overlays::task_manager::TaskTab::Resources => format!(
                    "{} cores · {:.1} load",
                    snapshot.cores.len(),
                    snapshot.load[0]
                ),
            };
            labels.push((subtitle, pane.x + 46, pane.y + 34, 0, tokens.dim));
            let count = format!("{} tasks", snapshot.process_count);
            labels.push((
                count.clone(),
                pane.x + pane.width as i32 - self.text_width(&count, 0) as i32 - 16,
                pane.y + 20,
                0,
                tokens.soft,
            ));
            // Column headers / divider.
            let header_y = pane.y + 56;
            chrome.push((
                AetherRect::new(pane.x + 1, header_y, pane.width.saturating_sub(2), 30),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            match tab {
                crate::app::overlays::task_manager::TaskTab::Processes => {
                    labels.push(("PID".into(), pane.x + 16, header_y + 9, 0, tokens.dim));
                    labels.push(("USER".into(), pane.x + 82, header_y + 9, 0, tokens.dim));
                    labels.push(("CPU".into(), pane.x + 190, header_y + 9, 0, tokens.dim));
                    labels.push(("MEM".into(), pane.x + 250, header_y + 9, 0, tokens.dim));
                    labels.push(("STATE".into(), pane.x + 320, header_y + 9, 0, tokens.dim));
                    labels.push(("COMMAND".into(), pane.x + 386, header_y + 9, 0, tokens.dim));
                }
                crate::app::overlays::task_manager::TaskTab::Users => {
                    labels.push(("UID".into(), pane.x + 16, header_y + 9, 0, tokens.dim));
                    labels.push(("USER".into(), pane.x + 90, header_y + 9, 0, tokens.dim));
                    labels.push((
                        "PROCESSES".into(),
                        pane.x + 230,
                        header_y + 9,
                        0,
                        tokens.dim,
                    ));
                    labels.push(("THREADS".into(), pane.x + 350, header_y + 9, 0, tokens.dim));
                    labels.push(("CPU".into(), pane.x + 460, header_y + 9, 0, tokens.dim));
                    labels.push(("MEM".into(), pane.x + 540, header_y + 9, 0, tokens.dim));
                }
                crate::app::overlays::task_manager::TaskTab::Resources => {}
            }
            match tab {
                crate::app::overlays::task_manager::TaskTab::Processes => {
                    let row_rects: Vec<AetherRect> = (0..visible)
                        .map(|index| self.task_row_rect(index))
                        .collect();
                    for offset in 0..visible {
                        let index = scroll + offset;
                        let Some((depth, process)) = process_rows.get(index) else {
                            break;
                        };
                        let Some(row) = row_rects.get(offset).copied() else {
                            break;
                        };
                        let active = index == selected;
                        if active {
                            if let Some(buffer) =
                                self.chrome_round(row.width, row.height, 7, tokens.accent(46))
                            {
                                panels.push((buffer, row.x, row.y));
                            }
                            if let Some(buffer) = self.chrome_round(
                                3,
                                row.height.saturating_sub(10),
                                2,
                                tokens.accent(255),
                            ) {
                                panels.push((buffer, row.x, row.y + 5));
                            }
                        }
                        let mut prefix = "  ".repeat(*depth);
                        if process.children.is_empty() {
                            prefix.push('·');
                        } else if collapsed.contains(&process.pid) {
                            prefix.push('▸');
                        } else {
                            prefix.push('▾');
                        }
                        labels.push((
                            format!("{prefix} {}", process.pid),
                            row.x + 12,
                            row.y + 7,
                            0,
                            tokens.dim,
                        ));
                        labels.push((process.user.clone(), row.x + 66, row.y + 7, 0, tokens.soft));
                        labels.push((
                            format!("{:.1}%", process.cpu_percent),
                            row.x + 174,
                            row.y + 7,
                            0,
                            if process.cpu_percent >= 50.0 {
                                tokens.amber
                            } else if process.cpu_percent >= 10.0 {
                                tokens.accent(255)
                            } else {
                                tokens.soft
                            },
                        ));
                        labels.push((
                            format!("{:.0}M", process.rss_kb as f64 / 1024.0),
                            row.x + 234,
                            row.y + 7,
                            0,
                            tokens.soft,
                        ));
                        labels.push((process.state.clone(), row.x + 304, row.y + 7, 0, tokens.dim));
                        let command = truncate(&process.command, 70);
                        labels.push((command, row.x + 370, row.y + 7, 0, tokens.text_soft));
                    }
                    if process_rows.len() > visible {
                        let track = AetherRect::new(
                            pane.x + pane.width as i32 - 9,
                            self.task_rows_top(),
                            4,
                            (visible * 28) as u32,
                        );
                        if let Some(buffer) = self.chrome_round(4, track.height, 2, tokens.base) {
                            panels.push((buffer, track.x, track.y));
                        }
                        let thumb = ((visible as f32 / process_rows.len() as f32)
                            * track.height as f32)
                            .clamp(12.0, track.height as f32)
                            as u32;
                        let offset = ((scroll as f32 / process_rows.len() as f32)
                            * track.height as f32) as i32;
                        if let Some(buffer) = self.chrome_round(4, thumb, 2, tokens.accent(190)) {
                            panels.push((buffer, track.x, track.y + offset));
                        }
                    }
                }
                crate::app::overlays::task_manager::TaskTab::Users => {
                    let row_rects: Vec<AetherRect> = (0..visible)
                        .map(|index| self.task_row_rect(index))
                        .collect();
                    for offset in 0..visible {
                        let Some(user) = users.get(offset) else {
                            break;
                        };
                        let Some(row) = row_rects.get(offset).copied() else {
                            break;
                        };
                        labels.push((user.uid.to_string(), row.x + 12, row.y + 7, 0, tokens.dim));
                        labels.push((
                            user.name.clone(),
                            row.x + 74,
                            row.y + 7,
                            0,
                            tokens.text_soft,
                        ));
                        labels.push((
                            user.processes.to_string(),
                            row.x + 214,
                            row.y + 7,
                            0,
                            tokens.soft,
                        ));
                        labels.push((
                            user.threads.to_string(),
                            row.x + 334,
                            row.y + 7,
                            0,
                            tokens.soft,
                        ));
                        labels.push((
                            format!("{:.1}%", user.cpu_percent),
                            row.x + 444,
                            row.y + 7,
                            0,
                            tokens.accent(255),
                        ));
                        labels.push((
                            format!("{:.0}M", user.rss_kb as f64 / 1024.0),
                            row.x + 524,
                            row.y + 7,
                            0,
                            tokens.soft,
                        ));
                    }
                }
                crate::app::overlays::task_manager::TaskTab::Resources => {
                    let content_y = pane.y + 68;
                    let card_w = (pane.width as i32 - 44) / 2;
                    let card_h = (pane.height as i32 - 68 - 56) / 2;
                    let cards = [
                        (pane.x + 16, content_y, "CPU"),
                        (pane.x + 28 + card_w, content_y, "MEMORY"),
                        (pane.x + 16, content_y + card_h + 12, "NETWORK"),
                        (pane.x + 28 + card_w, content_y + card_h + 12, "DISK"),
                    ];
                    for (x, y, name) in cards {
                        if let Some(buffer) = self.chrome_round(
                            card_w as u32,
                            card_h as u32,
                            8,
                            tokens.surface_a(220),
                        ) {
                            panels.push((buffer, x, y));
                        }
                        labels.push((name.into(), x + 12, y + 10, 0, tokens.dim));
                    }
                    // CPU card.
                    let cpu_x = pane.x + 28;
                    let cpu_y = content_y + 28;
                    labels.push((
                        format!("{:.0}%", snapshot.cpu_percent),
                        cpu_x,
                        cpu_y - 2,
                        3,
                        tokens.text,
                    ));
                    for (index, core) in snapshot.cores.iter().take(8).enumerate() {
                        let bar_x = cpu_x + (index as i32 % 2) * (card_w / 2 - 8);
                        let bar_y = cpu_y + 28 + (index as i32 / 2) * 22;
                        labels.push((format!("cpu{index}"), bar_x, bar_y, 0, tokens.dim));
                        if let Some(buffer) = self.chrome_round(64, 6, 3, tokens.base) {
                            panels.push((buffer, bar_x + 38, bar_y + 2));
                        }
                        let fill = ((core.usage / 100.0) * 64.0) as u32;
                        if fill > 0 {
                            if let Some(buffer) = self.chrome_round(fill, 6, 3, tokens.accent(220))
                            {
                                panels.push((buffer, bar_x + 38, bar_y + 2));
                            }
                        }
                    }
                    // Memory card.
                    let mem_x = pane.x + 28 + card_w;
                    let mem_y = content_y + 28;
                    let used = snapshot
                        .memory
                        .total_kb
                        .saturating_sub(snapshot.memory.available_kb);
                    labels.push((
                        format!(
                            "{:.1} / {:.1} GB",
                            used as f64 / 1_048_576.0,
                            snapshot.memory.total_kb as f64 / 1_048_576.0
                        ),
                        mem_x,
                        mem_y,
                        1,
                        tokens.text,
                    ));
                    let mem_bar = card_w as u32 - 24;
                    if let Some(buffer) = self.chrome_round(mem_bar, 8, 4, tokens.base) {
                        panels.push((buffer, mem_x, mem_y + 24));
                    }
                    let mem_fill = if snapshot.memory.total_kb > 0 {
                        ((used as f64 / snapshot.memory.total_kb as f64) * mem_bar as f64) as u32
                    } else {
                        0
                    };
                    if mem_fill > 0 {
                        if let Some(buffer) = self.chrome_round(mem_fill, 8, 4, tokens.accent(220))
                        {
                            panels.push((buffer, mem_x, mem_y + 24));
                        }
                    }
                    labels.push((
                        format!(
                            "swap {:.1} / {:.1} GB",
                            (snapshot.memory.swap_total_kb - snapshot.memory.swap_free_kb) as f64
                                / 1_048_576.0,
                            snapshot.memory.swap_total_kb as f64 / 1_048_576.0
                        ),
                        mem_x,
                        mem_y + 40,
                        0,
                        tokens.dim,
                    ));
                    // Network card.
                    let net_x = pane.x + 28;
                    let net_y = content_y + card_h + 40;
                    for (index, interface) in snapshot.network.iter().take(5).enumerate() {
                        labels.push((
                            format!(
                                "{}  ↓ {}/s  ↑ {}/s",
                                interface.name,
                                format_rate(interface.rx_rate),
                                format_rate(interface.tx_rate)
                            ),
                            net_x,
                            net_y + index as i32 * 20,
                            0,
                            tokens.soft,
                        ));
                    }
                    // Disk card.
                    let disk_x = pane.x + 28 + card_w;
                    let disk_y = content_y + card_h + 40;
                    for (index, disk) in snapshot.disks.iter().take(5).enumerate() {
                        labels.push((
                            format!(
                                "{}  r {}/s  w {}/s",
                                disk.name,
                                format_rate(disk.read_rate),
                                format_rate(disk.write_rate)
                            ),
                            disk_x,
                            disk_y + index as i32 * 20,
                            0,
                            tokens.soft,
                        ));
                    }
                    labels.push((
                        format!(
                            "load {:.2} {:.2} {:.2}",
                            snapshot.load[0], snapshot.load[1], snapshot.load[2]
                        ),
                        disk_x,
                        disk_y + 112,
                        0,
                        tokens.dim,
                    ));
                }
            }
            // Footer + status.
            let footer_y = pane.y + pane.height as i32 - 30;
            chrome.push((
                AetherRect::new(pane.x + 1, footer_y, pane.width.saturating_sub(2), 29),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            if let Some((_, process)) = process_rows.get(selected) {
                labels.push((
                    format!("{} · pid {}", truncate(&process.command, 58), process.pid),
                    pane.x + 16,
                    footer_y + 9,
                    0,
                    tokens.soft,
                ));
            }
            labels.push((
                "↑↓ select  ←→ collapse  tab tabs  r refresh  esc close".into(),
                pane.x + pane.width as i32 - 430,
                footer_y + 9,
                0,
                tokens.dim,
            ));
            let status_y = bounds.y + height - 32;
            labels.push((
                format!(
                    "{} processes · {} threads · cpu {:.0}% · load {:.2} · up {}",
                    snapshot.process_count,
                    snapshot.thread_count,
                    snapshot.cpu_percent,
                    snapshot.load[0],
                    format_uptime(snapshot.uptime_secs)
                ),
                bounds.x + 16,
                status_y + 12,
                0,
                tokens.soft,
            ));
            labels.push((
                format!(
                    "mem {:.1}/{:.1} GB",
                    (snapshot.memory.total_kb - snapshot.memory.available_kb) as f64 / 1_048_576.0,
                    snapshot.memory.total_kb as f64 / 1_048_576.0
                ),
                bounds.x + width - 180,
                status_y + 12,
                0,
                tokens.dim,
            ));
        }

        RenderChunk {
            chrome,
            labels,
            icons: Vec::new(),
            panels,
            display_icons,
        }
    }
}
