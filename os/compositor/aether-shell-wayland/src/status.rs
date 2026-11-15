//! Compositor status-bar sampling: agents, CPU, memory, audio and frame time.
//!
//! Sampling is called at most once per second and only while the bar is
//! visible; it reads `/proc` and optionally invokes `wpctl`.

use std::process::Command;
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusSnapshot {
    pub agents: usize,
    pub cpu_percent: u8,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub audio: String,
    /// Active remote-session summary; empty when no session exists.
    pub remote: String,
    pub frame_ms: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CpuTimes {
    pub idle: u64,
    pub total: u64,
}

pub fn read_cpu() -> Option<CpuTimes> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let mut fields = line
        .split_whitespace()
        .skip(1)
        .map(|value| value.parse::<u64>().ok());
    let mut values = [0u64; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value = fields.next().flatten()?;
        if index == 7 {
            break;
        }
    }
    let idle = values[3] + values[4];
    Some(CpuTimes {
        idle,
        total: values.iter().sum(),
    })
}

pub fn cpu_percent(previous: Option<CpuTimes>, current: CpuTimes) -> u8 {
    let Some(previous) = previous else {
        return 0;
    };
    let total = current.total.saturating_sub(previous.total);
    if total == 0 {
        return 0;
    }
    let busy = total.saturating_sub(current.idle.saturating_sub(previous.idle));
    ((busy.saturating_mul(100) / total).min(100)) as u8
}

pub fn read_memory() -> Option<(u64, u64)> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = None;
    let mut available = None;
    for line in info.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            total = value.split_whitespace().next()?.parse::<u64>().ok();
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            available = value.split_whitespace().next()?.parse::<u64>().ok();
        }
    }
    let total = total? / 1024;
    let available = available.unwrap_or(0) / 1024;
    Some((total.saturating_sub(available), total))
}

pub fn read_audio() -> String {
    let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
    else {
        return "audio -".to_string();
    };
    if !output.status.success() {
        return "audio -".to_string();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let volume = text
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| (value * 100.0).round() as u32)
        .unwrap_or(0);
    if text.contains("[MUTED]") {
        "audio muted".to_string()
    } else {
        format!("audio {volume}%")
    }
}

impl StatusSnapshot {
    pub fn sample(previous: Option<CpuTimes>, agents: usize, frame_ms: f32) -> Self {
        let current = read_cpu();
        let (used, total) = read_memory().unwrap_or((0, 0));
        Self {
            agents,
            cpu_percent: current.map(|now| cpu_percent(previous, now)).unwrap_or(0),
            mem_used_mb: used,
            mem_total_mb: total,
            audio: read_audio(),
            remote: String::new(),
            frame_ms,
        }
    }

    pub fn label(&self) -> String {
        let mut label = format!(
            "{} agents  {}%  {}/{} MB  {}",
            self.agents, self.cpu_percent, self.mem_used_mb, self.mem_total_mb, self.audio
        );
        if !self.remote.is_empty() {
            label.push_str("  ");
            label.push_str(&self.remote);
        }
        label.push_str(&format!("  {:.1} ms", self.frame_ms));
        label
    }
}

pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percent_uses_the_delta() {
        let previous = CpuTimes {
            idle: 100,
            total: 200,
        };
        let current = CpuTimes {
            idle: 150,
            total: 300,
        };
        assert_eq!(cpu_percent(Some(previous), current), 50);
        assert_eq!(cpu_percent(None, current), 0);
        assert_eq!(
            cpu_percent(
                Some(previous),
                CpuTimes {
                    idle: 200,
                    total: 200
                }
            ),
            0
        );
    }

    #[test]
    fn label_contains_every_field() {
        let status = StatusSnapshot {
            agents: 2,
            cpu_percent: 17,
            mem_used_mb: 512,
            mem_total_mb: 2048,
            audio: "audio 65%".into(),
            remote: "remote 18.4 ms · 12.0 Mbps · H.264".into(),
            frame_ms: 8.3,
        };
        let label = status.label();
        assert!(label.contains("remote 18.4 ms"));
        assert!(label.contains("2 agents"));
        assert!(label.contains("17%"));
        assert!(label.contains("512/2048 MB"));
        assert!(label.contains("audio 65%"));
        assert!(label.contains("8.3 ms"));
    }
}
