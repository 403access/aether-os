//! Host probes used by read-only tools.

use std::process::Command;

use serde_json::{json, Value};

use crate::control::ControlClient;

pub fn status(control: Option<&ControlClient>) -> Result<Value, String> {
    let compositor = match control {
        Some(control) => match control.call("status", &json!({})) {
            Ok(status) => json!({"connected": true, "status": status}),
            Err(error) => json!({"connected": false, "error": error}),
        },
        None => json!({"connected": false, "error": "AETHER_CONTROL_SOCKET is not set"}),
    };
    Ok(json!({
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "hostname": hostname(),
        "uptime_secs": uptime_secs(),
        "load": load_average(),
        "memory": memory(),
        "cpu_count": std::thread::available_parallelism().map(|count| count.get()).unwrap_or(0),
        "process_count": process_count(),
        "compositor": compositor,
    }))
}

pub fn processes(limit: usize, sort: &str, query: Option<&str>) -> Result<Value, String> {
    let output = command(
        "ps",
        &["-axo", "pid=,ppid=,user=,%cpu=,%mem=,rss=,comm=,args="],
    );
    #[cfg(target_os = "linux")]
    let rows = match output {
        Some(output) => output
            .lines()
            .filter_map(parse_process_line)
            .collect::<Vec<Value>>(),
        None => linux_processes(),
    };
    #[cfg(not(target_os = "linux"))]
    let rows = output
        .ok_or_else(|| "ps(1) is unavailable".to_string())?
        .lines()
        .filter_map(parse_process_line)
        .collect::<Vec<Value>>();
    let mut rows = rows;
    let filter = query.map(|query| query.to_lowercase());
    rows.retain(|row| {
        let Some(filter) = filter.as_ref() else {
            return true;
        };
        row["name"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains(filter)
            || row["command"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
                .contains(filter)
    });
    match sort {
        "memory" => rows.sort_by(|a, b| {
            b["rss_kb"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["rss_kb"].as_u64().unwrap_or_default())
        }),
        "pid" => rows.sort_by_key(|row| row["pid"].as_u64().unwrap_or_default()),
        _ => rows.sort_by(|a, b| {
            b["cpu_percent"]
                .as_f64()
                .unwrap_or_default()
                .total_cmp(&a["cpu_percent"].as_f64().unwrap_or_default())
        }),
    }
    rows.truncate(limit);
    Ok(json!({"processes": rows, "limit": limit, "sort": sort}))
}

fn parse_process_line(line: &str) -> Option<Value> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 7 {
        return None;
    }
    let pid: u64 = fields[0].parse().ok()?;
    let ppid: u64 = fields[1].parse().ok()?;
    let user = fields[2].to_string();
    let cpu: f64 = fields[3].parse().unwrap_or_default();
    let mem: f64 = fields[4].parse().unwrap_or_default();
    let rss: u64 = fields[5].parse().unwrap_or_default();
    let name = fields[6].to_string();
    let command = fields[6..].join(" ");
    Some(json!({
        "pid": pid,
        "ppid": ppid,
        "user": user,
        "cpu_percent": (cpu * 10.0).round() / 10.0,
        "memory_percent": (mem * 10.0).round() / 10.0,
        "rss_kb": rss,
        "name": name,
        "command": command,
    }))
}

#[cfg(target_os = "linux")]
fn linux_processes() -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|name| name.parse::<u64>().ok()) else {
            continue;
        };
        let directory = entry.path();
        let Ok(stat) = std::fs::read_to_string(directory.join("stat")) else {
            continue;
        };
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let comm = stat
            .find('(')
            .map(|open| stat[open + 1..close].to_string())
            .unwrap_or_default();
        let rest: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        let state = rest.first().copied().unwrap_or_default();
        let ppid: u64 = rest
            .get(1)
            .and_then(|value| value.parse().ok())
            .unwrap_or_default();
        let rss_pages = rest
            .get(21)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        let status = std::fs::read_to_string(directory.join("status")).unwrap_or_default();
        let uid = status
            .lines()
            .find(|line| line.starts_with("Uid:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();
        let threads = status
            .lines()
            .find(|line| line.starts_with("Threads:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1);
        let command = std::fs::read(directory.join("cmdline"))
            .map(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .replace('\0', " ")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();
        let command = if command.is_empty() {
            comm.clone()
        } else {
            command
        };
        rows.push(json!({
            "pid": pid,
            "ppid": ppid,
            "user": uid.to_string(),
            "cpu_percent": 0.0,
            "memory_percent": 0.0,
            "rss_kb": rss_pages.saturating_mul(4),
            "threads": threads,
            "state": state,
            "name": comm,
            "command": command,
        }));
    }
    rows
}

pub fn notify(title: &str, body: &str) -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape_apple(body),
            escape_apple(title)
        );
        let status = Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map_err(|error| format!("osascript failed: {error}"))?;
        if !status.success() {
            return Err("osascript notification failed".into());
        }
        Ok(json!({"notified": true, "via": "osascript"}))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let status = Command::new("notify-send")
            .arg(title)
            .arg(body)
            .status()
            .map_err(|error| format!("notify-send failed: {error}"))?;
        if !status.success() {
            return Err("notify-send failed".into());
        }
        Ok(json!({"notified": true, "via": "notify-send"}))
    }
}

#[cfg(target_os = "macos")]
fn escape_apple(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r', '\t'], " ")
}

fn hostname() -> String {
    if let Some(value) = command("hostname", &[]) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return value;
        }
    }
    for path in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(value) = std::fs::read_to_string(path) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return value;
            }
        }
    }
    String::new()
}

fn uptime_secs() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/uptime") {
            if let Some(first) = text.split_whitespace().next() {
                if let Ok(seconds) = first.parse::<f64>() {
                    return seconds as u64;
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(text) = command("sysctl", &["-n", "kern.boottime"]) {
            if let Some(index) = text.find("sec = ") {
                let tail = &text[index + 6..];
                if let Some(seconds) = tail
                    .split(|character: char| !character.is_ascii_digit())
                    .next()
                    .and_then(|value| value.parse::<u64>().ok())
                {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.as_secs())
                        .unwrap_or_default();
                    return now.saturating_sub(seconds);
                }
            }
        }
    }
    0
}

fn load_average() -> [f64; 3] {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/loadavg") {
            let mut values = text.split_whitespace();
            return [
                values
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default(),
                values
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default(),
                values
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default(),
            ];
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(text) = command("sysctl", &["-n", "vm.loadavg"]) {
            let numbers: Vec<f64> = text
                .trim_matches(|character: char| !character.is_ascii_digit() && character != '.')
                .split_whitespace()
                .filter_map(|value| value.trim_matches('}').parse().ok())
                .collect();
            if numbers.len() >= 3 {
                return [numbers[0], numbers[1], numbers[2]];
            }
        }
    }
    [0.0, 0.0, 0.0]
}

#[cfg(target_os = "linux")]
fn memory() -> Value {
    let mut total = 0u64;
    let mut available = 0u64;
    if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("MemTotal:") {
                total = parse_kb(value) * 1024;
            } else if let Some(value) = line.strip_prefix("MemAvailable:") {
                available = parse_kb(value) * 1024;
            }
        }
    }
    json!({"total_bytes": total, "available_bytes": available})
}

#[cfg(target_os = "macos")]
fn memory() -> Value {
    let total = command("sysctl", &["-n", "hw.memsize"])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or_default();
    let page_size = command("sysctl", &["-n", "hw.pagesize"])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(4096);
    let free_pages = command("vm_stat", &[])
        .map(|text| {
            text.lines()
                .filter(|line| {
                    line.starts_with("Pages free:")
                        || line.starts_with("Pages inactive:")
                        || line.starts_with("Pages speculative:")
                })
                .filter_map(|line| {
                    let value = line.split(':').nth(1)?;
                    value.trim().trim_end_matches('.').parse::<u64>().ok()
                })
                .sum::<u64>()
        })
        .unwrap_or_default();
    json!({
        "total_bytes": total,
        "available_bytes": free_pages.saturating_mul(page_size),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn memory() -> Value {
    json!({"total_bytes": 0, "available_bytes": 0})
}

fn process_count() -> usize {
    if let Some(text) = command("ps", &["-axo", "pid="]) {
        let count = text.lines().filter(|line| !line.trim().is_empty()).count();
        if count > 0 {
            return count;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/proc") {
            return entries
                .flatten()
                .filter(|entry| {
                    entry.file_name().to_str().is_some_and(|name| {
                        name.chars().all(|character| character.is_ascii_digit())
                    })
                })
                .count();
        }
    }
    0
}

#[cfg(target_os = "linux")]
fn parse_kb(value: &str) -> u64 {
    value
        .split_whitespace()
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

fn command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_ps_row() {
        let row =
            parse_process_line("  501     1 oliver   2.5  1.2  40960 /usr/bin/foo --flag value")
                .unwrap();
        assert_eq!(row["pid"], 501);
        assert_eq!(row["ppid"], 1);
        assert_eq!(row["user"], "oliver");
        assert_eq!(row["cpu_percent"], 2.5);
        assert_eq!(row["rss_kb"], 40960);
        assert_eq!(row["name"], "/usr/bin/foo");
        assert_eq!(row["command"], "/usr/bin/foo --flag value");
    }

    #[test]
    fn rejects_short_rows() {
        assert!(parse_process_line("1 2 3").is_none());
    }
}
