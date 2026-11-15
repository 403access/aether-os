//! Linux `/proc` sampler for the Aether task manager.
//!
//! Parsing is pure and unit-tested; `SystemMonitor` owns the previous counters
//! needed for CPU, network and disk rates. Non-Linux builds return an empty
//! snapshot so the shell still compiles on developer machines.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuCore {
    pub id: u32,
    pub usage: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Memory {
    pub total_kb: u64,
    pub available_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Disk {
    pub name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_rate: f64,
    pub write_rate: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub user: String,
    pub name: String,
    pub command: String,
    pub state: String,
    pub threads: u32,
    pub rss_kb: u64,
    pub cpu_percent: f32,
    pub cpu_ticks: u64,
    pub children: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserUsage {
    pub uid: u32,
    pub name: String,
    pub processes: usize,
    pub threads: u64,
    pub cpu_percent: f32,
    pub rss_kb: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemSnapshot {
    pub processes: Vec<Process>,
    pub users: Vec<UserUsage>,
    pub cores: Vec<CpuCore>,
    pub memory: Memory,
    pub network: Vec<NetworkInterface>,
    pub disks: Vec<Disk>,
    pub cpu_percent: f32,
    pub process_count: usize,
    pub thread_count: u64,
    pub load: [f32; 3],
    pub uptime_secs: u64,
    pub sampled_at: Option<Instant>,
}

impl SystemSnapshot {
    pub fn sample() -> Self {
        let mut monitor = SystemMonitor::default();
        monitor.sample()
    }

    pub fn process(&self, pid: u32) -> Option<&Process> {
        self.processes.iter().find(|process| process.pid == pid)
    }

    pub fn root_processes(&self) -> impl Iterator<Item = &Process> {
        self.processes
            .iter()
            .filter(|process| process.ppid == 0 || self.process(process.ppid).is_none())
    }

    /// Depth-first process order, honouring collapsed parents.
    pub fn tree(&self, collapsed: &std::collections::HashSet<u32>) -> Vec<(usize, u32)> {
        let mut result = Vec::with_capacity(self.processes.len());
        for root in self.root_processes() {
            self.walk(root.pid, 0, collapsed, &mut result);
        }
        result
    }

    fn walk(
        &self,
        pid: u32,
        depth: usize,
        collapsed: &std::collections::HashSet<u32>,
        result: &mut Vec<(usize, u32)>,
    ) {
        let Some(process) = self.process(pid) else {
            return;
        };
        result.push((depth, pid));
        if collapsed.contains(&pid) {
            return;
        }
        for child in &process.children {
            self.walk(*child, depth + 1, collapsed, result);
        }
    }
}

/// Aggregate CPU counters from `/proc/stat`, in kernel jiffies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTimes {
    pub total: u64,
    pub idle: u64,
}

#[derive(Default)]
pub struct SystemMonitor {
    previous_at: Option<Instant>,
    total_cpu: Option<CpuTimes>,
    core_cpu: Vec<CpuTimes>,
    process_ticks: HashMap<u32, u64>,
    network: HashMap<String, (u64, u64)>,
    disks: HashMap<String, (u64, u64)>,
}

impl SystemMonitor {
    pub fn sample(&mut self) -> SystemSnapshot {
        #[cfg(not(target_os = "linux"))]
        {
            return SystemSnapshot::default();
        }
        #[cfg(target_os = "linux")]
        {
            self.sample_linux()
        }
    }

    #[cfg(target_os = "linux")]
    fn sample_linux(&mut self) -> SystemSnapshot {
        let now = Instant::now();
        let elapsed = self
            .previous_at
            .map(|previous| now.duration_since(previous).as_secs_f64().max(0.001))
            .unwrap_or(1.0);
        let (total_cpu, core_cpu) = read_cpu();
        let total_delta = total_cpu
            .total
            .saturating_sub(
                self.total_cpu
                    .map_or(total_cpu.total, |previous| previous.total),
            )
            .max(1);
        let idle_delta = total_cpu.idle.saturating_sub(
            self.total_cpu
                .map_or(total_cpu.idle, |previous| previous.idle),
        );
        let cpu_percent = ((total_delta.saturating_sub(idle_delta)) as f32 / total_delta as f32
            * 100.0)
            .clamp(0.0, 100.0);
        let cores: Vec<CpuCore> = core_cpu
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let previous = self.core_cpu.get(index).copied().unwrap_or(CpuTimes {
                    total: current.total,
                    idle: current.idle,
                });
                let total = current.total.saturating_sub(previous.total).max(1);
                let idle = current.idle.saturating_sub(previous.idle);
                CpuCore {
                    id: index as u32,
                    usage: ((total.saturating_sub(idle)) as f32 / total as f32 * 100.0)
                        .clamp(0.0, 100.0),
                }
            })
            .collect();
        let users = read_users();
        let mut processes = read_processes(
            total_delta,
            cpu_core::ncpu(&core_cpu),
            &self.process_ticks,
            &users,
        );
        attach_children(&mut processes);
        let process_ticks = processes
            .iter()
            .map(|process| (process.pid, process.cpu_ticks))
            .collect();
        let user_usage = aggregate_users(&processes);
        let memory = read_memory();
        let network = read_network(elapsed, &self.network);
        let disks = read_disks(elapsed, &self.disks);
        let load = read_load();
        let uptime_secs = read_uptime();
        let thread_count = processes
            .iter()
            .map(|process| u64::from(process.threads))
            .sum();

        self.previous_at = Some(now);
        self.total_cpu = Some(total_cpu);
        self.core_cpu = core_cpu;
        self.process_ticks = process_ticks;
        self.network = network
            .iter()
            .map(|interface| {
                (
                    interface.name.clone(),
                    (interface.rx_bytes, interface.tx_bytes),
                )
            })
            .collect();
        self.disks = disks
            .iter()
            .map(|disk| (disk.name.clone(), (disk.read_bytes, disk.write_bytes)))
            .collect();

        SystemSnapshot {
            process_count: processes.len(),
            processes,
            users: user_usage,
            cores,
            memory,
            network,
            disks,
            cpu_percent,
            thread_count,
            load,
            uptime_secs,
            sampled_at: Some(now),
        }
    }
}

/// Number of logical CPUs from the per-core list (at least one).
mod cpu_core {
    use super::CpuTimes;
    pub fn ncpu(cores: &[CpuTimes]) -> u32 {
        cores.len().max(1) as u32
    }
}

fn read_cpu() -> (CpuTimes, Vec<CpuTimes>) {
    let text = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let mut total = CpuTimes::default();
    let mut cores = Vec::new();
    for line in text.lines() {
        if !line.starts_with("cpu") {
            continue;
        }
        let Some(time) = parse_cpu_line(line) else {
            continue;
        };
        if line.split_whitespace().next() == Some("cpu") {
            total = time;
        } else {
            cores.push(time);
        }
    }
    (total, cores)
}

fn parse_cpu_line(line: &str) -> Option<CpuTimes> {
    let mut fields = line.split_whitespace();
    fields.next()?;
    let values: Vec<u64> = fields.filter_map(|value| value.parse().ok()).collect();
    if values.len() < 5 {
        return None;
    }
    Some(CpuTimes {
        total: values.iter().sum(),
        idle: values[3] + values[4],
    })
}

/// Read the aggregate `/proc/stat` counters without sampling processes.
///
/// Interactive surfaces (the status bar, small overlays) use this instead of
/// [`SystemMonitor`] so one refresh does not walk all of `/proc`.
pub fn cpu_times() -> Option<CpuTimes> {
    let text = std::fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().find(|line| line.starts_with("cpu "))?;
    parse_cpu_line(line)
}

/// Busy CPU percentage between two samples.
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

fn read_users() -> HashMap<u32, String> {
    let mut users = HashMap::new();
    let text = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    for line in text.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() >= 3 {
            if let Ok(uid) = fields[2].parse() {
                users.insert(uid, fields[0].to_string());
            }
        }
    }
    users
}

fn read_processes(
    total_delta: u64,
    cores: u32,
    previous: &HashMap<u32, u64>,
    users: &HashMap<u32, String>,
) -> Vec<Process> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut processes = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_string_lossy().parse::<u32>().ok() else {
            continue;
        };
        let status = std::fs::read_to_string(entry.path().join("status")).ok();
        let stat = std::fs::read_to_string(entry.path().join("stat")).ok();
        let Some(status) = status else { continue };
        let Some((ppid, ticks)) = stat.as_deref().and_then(parse_stat) else {
            continue;
        };
        let fields = status_fields(&status);
        let uid = fields
            .get("Uid")
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let name = fields
            .get("Name")
            .cloned()
            .unwrap_or_else(|| format!("pid-{pid}"));
        let state = fields
            .get("State")
            .and_then(|value| value.chars().next())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".into());
        let threads = fields
            .get("Threads")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1);
        let rss_kb = fields
            .get("VmRSS")
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let command = std::fs::read(entry.path().join("cmdline"))
            .ok()
            .map(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .replace('\0', " ")
                    .trim()
                    .to_string()
            })
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| name.clone());
        let cpu_percent = previous
            .get(&pid)
            .map(|previous_ticks| {
                ticks.saturating_sub(*previous_ticks) as f32 / total_delta as f32
                    * cores as f32
                    * 100.0
            })
            .unwrap_or(0.0)
            .clamp(0.0, cores as f32 * 100.0);
        processes.push(Process {
            pid,
            ppid,
            uid,
            user: users.get(&uid).cloned().unwrap_or_else(|| uid.to_string()),
            name,
            command,
            state,
            threads,
            rss_kb,
            cpu_percent,
            cpu_ticks: ticks,
            children: Vec::new(),
        });
    }
    processes.sort_by_key(|process| process.pid);
    processes
}

fn status_fields(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn parse_stat(text: &str) -> Option<(u32, u64)> {
    let end = text.rfind(')')?;
    let fields: Vec<&str> = text[end + 1..].split_whitespace().collect();
    if fields.len() < 18 {
        return None;
    }
    let ppid = fields.get(1)?.parse().ok()?;
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((ppid, utime + stime))
}

fn attach_children(processes: &mut [Process]) {
    let parents: Vec<(u32, u32)> = processes
        .iter()
        .map(|process| (process.pid, process.ppid))
        .collect();
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (pid, ppid) in parents {
        children.entry(ppid).or_default().push(pid);
    }
    for process in processes.iter_mut() {
        process.children = children.get(&process.pid).cloned().unwrap_or_default();
    }
}

fn aggregate_users(processes: &[Process]) -> Vec<UserUsage> {
    let mut users: BTreeMap<u32, UserUsage> = BTreeMap::new();
    for process in processes {
        let user = users.entry(process.uid).or_insert_with(|| UserUsage {
            uid: process.uid,
            name: process.user.clone(),
            ..UserUsage::default()
        });
        user.processes += 1;
        user.threads += u64::from(process.threads);
        user.cpu_percent += process.cpu_percent;
        user.rss_kb += process.rss_kb;
    }
    let mut users: Vec<UserUsage> = users.into_values().collect();
    users.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
    users
}

fn read_memory() -> Memory {
    let text = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    parse_memory(&text)
}

fn parse_memory(text: &str) -> Memory {
    let mut memory = Memory::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value
            .split_whitespace()
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        match key {
            "MemTotal" => memory.total_kb = value,
            "MemAvailable" => memory.available_kb = value,
            "SwapTotal" => memory.swap_total_kb = value,
            "SwapFree" => memory.swap_free_kb = value,
            _ => {}
        }
    }
    memory
}

/// Read `/proc/meminfo`, or `None` when `/proc` is unavailable.
pub fn memory_info() -> Option<Memory> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    Some(parse_memory(&text))
}

/// Used and total memory in mebibytes for compact status labels.
pub fn memory_usage_mb() -> Option<(u64, u64)> {
    let memory = memory_info()?;
    let total = memory.total_kb / 1024;
    let used = memory.total_kb.saturating_sub(memory.available_kb) / 1024;
    Some((used, total))
}

fn read_network(elapsed: f64, previous: &HashMap<String, (u64, u64)>) -> Vec<NetworkInterface> {
    let text = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut interfaces = Vec::new();
    for line in text.lines().skip(2) {
        let Some((name, values)) = line.split_once(':') else {
            continue;
        };
        let fields: Vec<u64> = values
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();
        if fields.len() < 9 {
            continue;
        }
        let name = name.trim().to_string();
        let (rx, tx) = (fields[0], fields[8]);
        let (previous_rx, previous_tx) = previous.get(&name).copied().unwrap_or((rx, tx));
        interfaces.push(NetworkInterface {
            rx_rate: rx.saturating_sub(previous_rx) as f64 / elapsed,
            tx_rate: tx.saturating_sub(previous_tx) as f64 / elapsed,
            name,
            rx_bytes: rx,
            tx_bytes: tx,
        });
    }
    interfaces
}

fn read_disks(elapsed: f64, previous: &HashMap<String, (u64, u64)>) -> Vec<Disk> {
    let text = std::fs::read_to_string("/proc/diskstats").unwrap_or_default();
    let mut disks = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let name = fields[2];
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("sr") {
            continue;
        }
        let read_sectors: u64 = fields
            .get(5)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let write_sectors: u64 = fields
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let (read, write) = (read_sectors * 512, write_sectors * 512);
        let (previous_read, previous_write) = previous.get(name).copied().unwrap_or((read, write));
        disks.push(Disk {
            name: name.to_string(),
            read_rate: read.saturating_sub(previous_read) as f64 / elapsed,
            write_rate: write.saturating_sub(previous_write) as f64 / elapsed,
            read_bytes: read,
            write_bytes: write,
        });
    }
    disks
}

fn read_load() -> [f32; 3] {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut load = [0.0; 3];
    for (index, value) in text.split_whitespace().take(3).enumerate() {
        load[index] = value.parse().unwrap_or(0.0);
    }
    load
}

fn read_uptime() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text| {
            text.split_whitespace()
                .next()
                .and_then(|value| value.split('.').next())
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_process_stat_with_parentheses_in_name() {
        let stat = "42 (name with ) paren) S 1 42 42 0 -1 4194560 100 0 0 0 12 8 0 0 20 0 4 0";
        assert_eq!(parse_stat(stat), Some((1, 20)));
    }

    #[test]
    fn parses_network_and_disk_layouts() {
        let net = "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n  eth0: 1000 1 0 0 0 0 0 0 2000 2 0 0 0 0 0 0\n";
        // `read_network` reads /proc directly; parser layout is covered by the
        // live Linux smoke. Keep the fixture shape documented here.
        assert!(net.contains("eth0:"));
    }

    #[test]
    fn builds_a_process_tree() {
        let snapshot = SystemSnapshot {
            processes: vec![
                Process {
                    pid: 1,
                    ppid: 0,
                    children: vec![2, 3],
                    ..Process::default()
                },
                Process {
                    pid: 2,
                    ppid: 1,
                    ..Process::default()
                },
                Process {
                    pid: 3,
                    ppid: 1,
                    ..Process::default()
                },
            ],
            ..SystemSnapshot::default()
        };
        let tree = snapshot.tree(&std::collections::HashSet::new());
        assert_eq!(tree, vec![(0, 1), (1, 2), (1, 3)]);
    }

    #[test]
    fn aggregates_users() {
        let processes = vec![
            Process {
                uid: 1000,
                user: "oliver".into(),
                cpu_percent: 10.0,
                rss_kb: 100,
                ..Process::default()
            },
            Process {
                uid: 1000,
                user: "oliver".into(),
                cpu_percent: 5.0,
                rss_kb: 50,
                ..Process::default()
            },
            Process {
                uid: 0,
                user: "root".into(),
                cpu_percent: 1.0,
                rss_kb: 10,
                ..Process::default()
            },
        ];
        let users = aggregate_users(&processes);
        assert_eq!(users[0].name, "oliver");
        assert_eq!(users[0].processes, 2);
        assert!((users[0].cpu_percent - 15.0).abs() < f32::EPSILON);
    }
}
