//! Agent sandboxes surface composition.

use std::sync::Arc;

use aether_geometry::Rect as AetherRect;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;

use crate::app::overlays::sandboxes::SandboxStatus;
use crate::app::App;
use crate::render::context::{FrameContext, RenderChunk};
use crate::render::helpers::truncate;
use crate::{scene_box, scene_node, scene_text};

impl App {
    pub(crate) fn compose_sandboxes(&mut self, ctx: &FrameContext) -> RenderChunk {
        let tokens = ctx.tokens;
        let panel_fill_soft = ctx.panel_fill_soft;
        let panel_border_soft = ctx.panel_border_soft;
        let here = ctx.flags.sandboxes;
        let mut chrome: Vec<(AetherRect, [f32; 4])> = Vec::new();
        let mut labels: Vec<(String, i32, i32, u32, [u8; 4])> = Vec::new();
        let mut panels: Vec<(Arc<MemoryRenderBuffer>, i32, i32)> = Vec::new();
        let mut display_icons: Vec<(String, u32, [u8; 4], i32, i32)> = Vec::new();

        if here {
            let bounds = self.sandbox_rect();
            let list = self.sandbox_list_rect();
            let detail = self.sandbox_detail_rect();
            let width = bounds.width as i32;
            let height = bounds.height as i32;
            let (records, snapshot, selected, scroll, message, running) = {
                let state = self.sandboxes.as_ref().expect("sandbox panel is open");
                (
                    state.records().to_vec(),
                    state.snapshot().clone(),
                    state.selected(),
                    state.scroll(),
                    state.message().map(str::to_string),
                    state.running_count(),
                )
            };
            let visible = self.sandbox_visible_rows();
            let panel = scene_box!((), None, "overlay", bounds, tokens.surface_a(236), 0.0);
            let list_node = scene_node!((), Some(panel), "agent-list", list);

            if let Some(buffer) =
                self.chrome_round(bounds.width.max(1), 62, 0, tokens.surface_a(236))
            {
                panels.push((buffer, bounds.x, bounds.y));
            }
            if let Some(buffer) =
                self.chrome_round(bounds.width.max(1), 36, 0, tokens.surface_a(242))
            {
                panels.push((buffer, bounds.x, bounds.y + height - 36));
            }
            if let Some(buffer) = self.chrome_panel(
                list.width,
                list.height,
                12,
                panel_fill_soft,
                panel_border_soft,
            ) {
                panels.push((buffer, list.x - 6, list.y - 6));
            }
            if let Some(buffer) = self.chrome_panel(
                detail.width,
                detail.height,
                12,
                panel_fill_soft,
                panel_border_soft,
            ) {
                panels.push((buffer, detail.x - 6, detail.y - 6));
            }

            display_icons.push((
                "agents".into(),
                28,
                tokens.accent(255),
                bounds.x + 18,
                bounds.y + 17,
            ));
            labels.push((
                "Agent sandboxes".into(),
                bounds.x + 56,
                bounds.y + 12,
                2,
                tokens.text,
            ));
            labels.push((
                format!(
                    "runtime registry · landlock + seccomp · {running}/{} running",
                    records.len()
                ),
                bounds.x + 56,
                bounds.y + 35,
                0,
                tokens.dim,
            ));
            display_icons.push((
                "close".into(),
                16,
                tokens.dim,
                self.sandbox_close_rect().x,
                self.sandbox_close_rect().y + 4,
            ));

            // List header.
            scene_text!(
                (),
                Some(list_node),
                "title",
                "Sandboxes",
                AetherRect::new(
                    list.x + 14,
                    list.y + 12,
                    self.text_width("Sandboxes", 1),
                    20
                ),
                1,
                tokens.text
            );
            labels.push(("Sandboxes".into(), list.x + 14, list.y + 12, 1, tokens.text));
            let count = format!("{} records", records.len());
            labels.push((
                count.clone(),
                list.x + list.width as i32 - self.text_width(&count, 0) as i32 - 14,
                list.y + 13,
                0,
                tokens.soft,
            ));
            let header_y = list.y + 38;
            chrome.push((
                AetherRect::new(list.x + 1, header_y, list.width.saturating_sub(2), 30),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            labels.push(("AGENT".into(), list.x + 30, header_y + 9, 0, tokens.dim));
            labels.push((
                "STATUS".into(),
                list.x + list.width as i32 - 118,
                header_y + 9,
                0,
                tokens.dim,
            ));

            if records.is_empty() {
                labels.push((
                    "No sandboxed agents".into(),
                    list.x + 16,
                    list.y + 92,
                    1,
                    tokens.soft,
                ));
                labels.push((
                    "Start one with `aether-sandbox run --id demo --read ~/work \\".into(),
                    list.x + 16,
                    list.y + 118,
                    0,
                    tokens.dim,
                ));
                labels.push((
                    "  --write ~/work/scratch -- /usr/bin/python3 task.py`".into(),
                    list.x + 16,
                    list.y + 136,
                    0,
                    tokens.dim,
                ));
            }

            for offset in 0..visible {
                let index = scroll + offset;
                let Some(record) = records.get(index) else {
                    break;
                };
                let row = self.sandbox_row_rect(offset);
                let active = index == selected;
                if active {
                    if let Some(buffer) =
                        self.chrome_round(row.width, row.height, 7, tokens.accent(46))
                    {
                        panels.push((buffer, row.x, row.y));
                    }
                    if let Some(buffer) =
                        self.chrome_round(3, row.height.saturating_sub(10), 2, tokens.accent(255))
                    {
                        panels.push((buffer, row.x, row.y + 5));
                    }
                }
                let running_here = snapshot
                    .process(record.pid)
                    .is_some_and(|process| process.state != "Z");
                let status = if running_here {
                    SandboxStatus::Running
                } else {
                    SandboxStatus::Stale
                };
                let dot = if running_here {
                    tokens.green
                } else {
                    tokens.amber
                };
                if let Some(buffer) = self.chrome_round(8, 8, 4, dot) {
                    panels.push((buffer, row.x + 12, row.y + 15));
                }
                labels.push((
                    truncate(&record.label, 34),
                    row.x + 30,
                    row.y + 8,
                    1,
                    tokens.text,
                ));
                labels.push((
                    format!(
                        "{} · {} · {}",
                        truncate(&record.id, 20),
                        record.trust,
                        record.tier
                    ),
                    row.x + 30,
                    row.y + 29,
                    0,
                    tokens.dim,
                ));
                labels.push((
                    status.label().into(),
                    row.x + row.width as i32 - 90,
                    row.y + 8,
                    0,
                    if running_here {
                        tokens.green
                    } else {
                        tokens.amber
                    },
                ));
                let usage = snapshot
                    .process(record.pid)
                    .map(|process| {
                        format!(
                            "pid {} · {:.1}% · {:.0}M",
                            process.pid,
                            process.cpu_percent,
                            process.rss_kb as f64 / 1024.0
                        )
                    })
                    .unwrap_or_else(|| format!("pid {} · no process", record.pid));
                labels.push((
                    usage,
                    row.x + row.width as i32 - 178,
                    row.y + 29,
                    0,
                    tokens.soft,
                ));
            }

            if records.len() > visible {
                let track = AetherRect::new(
                    list.x + list.width as i32 - 9,
                    self.sandbox_rows_top(),
                    4,
                    (visible * 54) as u32,
                );
                if let Some(buffer) = self.chrome_round(4, track.height, 2, tokens.base) {
                    panels.push((buffer, track.x, track.y));
                }
                let thumb = ((visible as f32 / records.len() as f32) * track.height as f32)
                    .clamp(12.0, track.height as f32) as u32;
                let offset = ((scroll as f32 / records.len() as f32) * track.height as f32) as i32;
                if let Some(buffer) = self.chrome_round(4, thumb, 2, tokens.accent(190)) {
                    panels.push((
                        buffer,
                        track.x,
                        track.y + offset.min(track.height as i32 - thumb as i32).max(0),
                    ));
                }
            }

            // Detail pane.
            labels.push((
                "Sandbox detail".into(),
                detail.x + 14,
                detail.y + 12,
                1,
                tokens.text,
            ));
            if let Some(record) = records.get(selected) {
                let mut y = detail.y + 48;
                labels.push((record.label.clone(), detail.x + 14, y, 2, tokens.strong));
                y += 28;
                labels.push((record.id.clone(), detail.x + 14, y, 0, tokens.dim));
                y += 28;
                labels.push((
                    format!(
                        "{} · {} · {}",
                        record.trust, record.tier, record.enforcement
                    ),
                    detail.x + 14,
                    y,
                    0,
                    tokens.soft,
                ));
                y += 26;
                let process = snapshot.process(record.pid);
                let status = if process.is_some_and(|process| process.state != "Z") {
                    (SandboxStatus::Running, tokens.green)
                } else {
                    (SandboxStatus::Stale, tokens.amber)
                };
                labels.push((
                    format!("● {}", status.0.label()),
                    detail.x + 14,
                    y,
                    0,
                    status.1,
                ));
                labels.push((
                    format!(
                        "pid {} · uid {}",
                        record.pid,
                        record
                            .uid
                            .map(|uid| uid.to_string())
                            .unwrap_or_else(|| "host".into())
                    ),
                    detail.x + 120,
                    y,
                    0,
                    tokens.soft,
                ));
                if let Some(process) = process {
                    labels.push((
                        format!(
                            "cpu {:.1}% · rss {:.0}M · threads {}",
                            process.cpu_percent,
                            process.rss_kb as f64 / 1024.0,
                            process.threads
                        ),
                        detail.x + 14,
                        y + 20,
                        0,
                        tokens.dim,
                    ));
                    y += 20;
                }
                y += 30;
                labels.push(("LIMITS".into(), detail.x + 14, y, 0, tokens.dim));
                y += 20;
                labels.push((
                    format!(
                        "{} MiB · timeout {} · net {}",
                        record.memory_mib,
                        record
                            .timeout_secs
                            .map(|seconds| format!("{seconds}s"))
                            .unwrap_or_else(|| "none".into()),
                        if record.allow_net {
                            "allowed"
                        } else {
                            "denied"
                        }
                    ),
                    detail.x + 14,
                    y,
                    0,
                    if record.allow_net && record.trust != "trusted" {
                        tokens.amber
                    } else {
                        tokens.soft
                    },
                ));
                y += 30;
                labels.push(("CAPABILITIES".into(), detail.x + 14, y, 0, tokens.dim));
                y += 20;
                if record.capabilities.is_empty() {
                    labels.push(("none".into(), detail.x + 14, y, 0, tokens.soft));
                    y += 20;
                } else {
                    for capability in record.capabilities.iter().take(4) {
                        labels.push((
                            format!("· {}", truncate(capability, 52)),
                            detail.x + 14,
                            y,
                            0,
                            tokens.soft,
                        ));
                        y += 20;
                    }
                }
                y += 12;
                labels.push(("READ PATHS".into(), detail.x + 14, y, 0, tokens.dim));
                y += 20;
                for path in record.read_paths.iter().take(4) {
                    labels.push((
                        format!("· {}", truncate(path, 56)),
                        detail.x + 14,
                        y,
                        0,
                        tokens.soft,
                    ));
                    y += 20;
                }
                if record.read_paths.len() > 4 {
                    labels.push((
                        format!("· +{} more", record.read_paths.len() - 4),
                        detail.x + 14,
                        y,
                        0,
                        tokens.dim,
                    ));
                    y += 20;
                }
                y += 10;
                labels.push(("WRITE PATHS".into(), detail.x + 14, y, 0, tokens.dim));
                y += 20;
                for path in record.write_paths.iter().take(4) {
                    labels.push((
                        format!("· {}", truncate(path, 56)),
                        detail.x + 14,
                        y,
                        0,
                        tokens.soft,
                    ));
                    y += 20;
                }
                if record.write_paths.is_empty() {
                    labels.push(("read-only".into(), detail.x + 14, y, 0, tokens.soft));
                    y += 20;
                }
                let bottom = detail.y + detail.height as i32 - 44;
                if !record.warnings.is_empty() && y < bottom {
                    y += 10;
                    labels.push(("WARNINGS".into(), detail.x + 14, y, 0, tokens.dim));
                    y += 20;
                    for warning in record.warnings.iter().take(3) {
                        labels.push((
                            format!("! {}", truncate(warning, 54)),
                            detail.x + 14,
                            y,
                            0,
                            tokens.amber,
                        ));
                        y += 20;
                    }
                }
                if let Some(log) = &record.log_path {
                    if y < bottom {
                        y += 10;
                        labels.push((
                            format!("log · {}", truncate(log, 52)),
                            detail.x + 14,
                            y,
                            0,
                            tokens.dim,
                        ));
                        y += 20;
                    }
                }
                if y > bottom {
                    labels.push(("…".into(), detail.x + 14, bottom, 0, tokens.dim));
                }
                labels.push((
                    format!(
                        "{} · {}",
                        truncate(&record.program, 40),
                        truncate(&record.args.join(" "), 28)
                    ),
                    detail.x + 14,
                    detail.y + detail.height as i32 - 24,
                    0,
                    tokens.dim,
                ));
            } else {
                labels.push((
                    "Select a sandbox to inspect its isolation plan.".into(),
                    detail.x + 14,
                    detail.y + 52,
                    0,
                    tokens.soft,
                ));
            }

            let footer_y = list.y + list.height as i32 - 30;
            chrome.push((
                AetherRect::new(list.x + 1, footer_y, list.width.saturating_sub(2), 29),
                tokens.surface_a(190).map(|value| value as f32 / 255.0),
            ));
            labels.push((
                message.unwrap_or_else(|| {
                    records
                        .get(selected)
                        .map(|record| format!("{} · pid {}", record.id, record.pid))
                        .unwrap_or_else(|| "no selection".into())
                }),
                list.x + 12,
                footer_y + 9,
                0,
                tokens.soft,
            ));
            labels.push((
                "↑↓ select  k stop  ⇧K force  l log  r refresh  c clean  esc close".into(),
                bounds.x + width - 500,
                bounds.y + height - 24,
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
