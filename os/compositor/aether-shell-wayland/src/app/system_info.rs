//! Host facts for the system apps: network, battery, Bluetooth and machine.
//!
//! Reads `/sys`, `/proc` and `/etc/resolv.conf` directly. Parsers are pure so
//! they can be tested against fixtures; missing files degrade to empty values
//! instead of failing, which keeps the apps usable in containers and VMs.

use std::fs;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetInterface {
    pub name: String,
    pub mac: String,
    pub state: String,
    pub wireless: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkFacts {
    pub interfaces: Vec<NetInterface>,
    pub ip: Option<String>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
}

impl NetworkFacts {
    pub fn primary(&self) -> Option<&NetInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.state == "up")
            .or_else(|| self.interfaces.first())
    }

    pub fn wireless(&self) -> bool {
        self.interfaces.iter().any(|interface| interface.wireless)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatteryFacts {
    pub present: bool,
    pub capacity: u8,
    pub status: String,
    pub health: String,
    pub cycles: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BluetoothFacts {
    pub adapter: bool,
    pub devices: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemFacts {
    pub hostname: String,
    pub kernel: String,
    pub cpu: String,
    pub memory_gb: f32,
}

pub fn network() -> NetworkFacts {
    let mut interfaces = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            let base = entry.path();
            let mac = read_trim(base.join("address")).unwrap_or_default();
            let state = read_trim(base.join("operstate")).unwrap_or_else(|| "unknown".into());
            let wireless = base.join("wireless").exists();
            interfaces.push(NetInterface {
                name,
                mac: mac.to_uppercase(),
                state,
                wireless,
            });
        }
    }
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));

    let ip = fs::read_to_string("/proc/net/fib_trie")
        .ok()
        .and_then(|text| local_ipv4(&text));
    let gateway = fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|text| default_gateway(&text));
    let dns = fs::read_to_string("/etc/resolv.conf")
        .ok()
        .map(|text| nameservers(&text))
        .unwrap_or_default();

    NetworkFacts {
        interfaces,
        ip,
        gateway,
        dns,
    }
}

pub fn battery() -> BatteryFacts {
    let mut facts = BatteryFacts::default();
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return facts;
    };
    for entry in entries.flatten() {
        let base = entry.path();
        if read_trim(base.join("type")).as_deref() != Some("Battery") {
            continue;
        }
        facts.present = true;
        facts.capacity = read_trim(base.join("capacity"))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        facts.status = read_trim(base.join("status")).unwrap_or_else(|| "Unknown".into());
        facts.health = read_trim(base.join("health")).unwrap_or_else(|| "—".into());
        facts.cycles = read_trim(base.join("cycle_count")).and_then(|value| value.parse().ok());
        facts.temperature = read_trim(base.join("temp"))
            .and_then(|value| value.parse::<f32>().ok())
            .map(|value| value / 10.0);
        break;
    }
    facts
}

pub fn bluetooth() -> BluetoothFacts {
    let adapter = fs::read_dir("/sys/class/bluetooth")
        .map(|entries| entries.flatten().next().is_some())
        .unwrap_or(false);
    BluetoothFacts {
        adapter,
        devices: Vec::new(),
    }
}

pub fn system() -> SystemFacts {
    let hostname = read_trim("/etc/hostname").unwrap_or_default();
    let kernel = read_trim("/proc/sys/kernel/osrelease").unwrap_or_default();
    let cpu = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| model_name(&text))
        .unwrap_or_else(|| "Unknown CPU".into());
    let memory_gb = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| mem_total_gb(&text))
        .unwrap_or(0.0);
    SystemFacts {
        hostname,
        kernel,
        cpu,
        memory_gb,
    }
}

fn read_trim(path: impl AsRef<std::path::Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// First non-loopback `|-- a.b.c.d` immediately followed by `/32 host LOCAL`.
pub(crate) fn local_ipv4(fib: &str) -> Option<String> {
    let mut candidate: Option<String> = None;
    for line in fib.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("|-- ") {
            if is_ipv4(rest.trim()) {
                candidate = Some(rest.trim().to_string());
            }
        } else if trimmed.contains("/32 host LOCAL") {
            if let Some(address) = candidate.take() {
                if !address.starts_with("127.") {
                    return Some(address);
                }
            }
        }
    }
    None
}

/// Default route gateway from `/proc/net/route` (little-endian hex).
pub(crate) fn default_gateway(route: &str) -> Option<String> {
    for line in route.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 || fields[1] != "00000000" {
            continue;
        }
        let hex = fields[2];
        if hex.len() != 8 {
            continue;
        }
        let mut octets = [0u8; 4];
        for index in 0..4 {
            octets[index] = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        }
        octets.reverse();
        return Some(format!(
            "{}.{}.{}.{}",
            octets[0], octets[1], octets[2], octets[3]
        ));
    }
    None
}

pub(crate) fn nameservers(resolv: &str) -> Vec<String> {
    resolv
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nameserver "))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn model_name(cpuinfo: &str) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        line.strip_prefix("model name")
            .and_then(|rest| rest.split_once(':'))
            .map(|(_, value)| value.trim().to_string())
    })
}

fn mem_total_gb(meminfo: &str) -> Option<f32> {
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("MemTotal:")?;
        let kb: f32 = rest.split_whitespace().next()?.parse().ok()?;
        Some((kb / 1_048_576.0 * 10.0).round() / 10.0)
    })
}

fn is_ipv4(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 4 && parts.iter().all(|part| part.parse::<u8>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_default_gateway() {
        let route =
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
                     eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n";
        assert_eq!(default_gateway(route).as_deref(), Some("192.168.1.1"));
    }

    #[test]
    fn parses_nameservers() {
        let resolv = "search lan\nnameserver 1.1.1.1\nnameserver 9.9.9.9\n";
        assert_eq!(nameservers(resolv), ["1.1.1.1", "9.9.9.9"]);
    }

    #[test]
    fn finds_the_local_ipv4_address() {
        let fib = "|-- 127.0.0.1\n   /32 host LOCAL\n|-- 192.168.1.24\n   /32 host LOCAL\n";
        assert_eq!(local_ipv4(fib).as_deref(), Some("192.168.1.24"));
    }

    #[test]
    fn parses_cpu_and_memory() {
        let cpu = "processor\t: 0\nmodel name\t: Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz\n";
        assert_eq!(
            model_name(cpu).as_deref(),
            Some("Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz")
        );
        let mem = "MemTotal:       16384000 kB\n";
        assert_eq!(mem_total_gb(mem), Some(15.6));
    }

    #[test]
    fn live_reads_never_panic() {
        let _ = network();
        let _ = battery();
        let _ = bluetooth();
        let _ = system();
    }
}
