//! Latency and frame-interval ledgers.
//!
//! Samples are microseconds; the report converts to milliseconds. Percentiles use
//! the nearest-rank method, which is what the gate scripts compare against.

use serde::{Deserialize, Serialize};

/// P7 gate: glass-to-glass on a wired LAN.
pub const GLASS_TO_GLASS_BUDGET_MS: f32 = 40.0;
/// The frame interval may be at most 1.5× the negotiated target.
pub const INTERVAL_FACTOR: f32 = 1.5;
/// Capture/import/encode budget before the network.
pub const CAPTURE_TO_SEND_BUDGET_US: u64 = 12_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatencyStats {
    samples: Vec<u64>,
}

impl LatencyStats {
    pub fn push(&mut self, value_us: u64) {
        self.samples.push(value_us);
    }

    pub fn extend<I: IntoIterator<Item = u64>>(&mut self, values: I) {
        self.samples.extend(values);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn samples(&self) -> &[u64] {
        &self.samples
    }

    pub fn percentile(&self, percentile: f32) -> Option<u64> {
        percentile_of(&self.samples, percentile)
    }

    pub fn p50(&self) -> Option<u64> {
        self.percentile(0.50)
    }

    pub fn p99(&self) -> Option<u64> {
        self.percentile(0.99)
    }

    pub fn max(&self) -> Option<u64> {
        self.samples.iter().copied().max()
    }

    pub fn mean(&self) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let total: u64 = self.samples.iter().sum();
        Some(total as f64 / self.samples.len() as f64)
    }
}

fn percentile_of(samples: &[u64], percentile: f32) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = ((percentile.clamp(0.0, 1.0) * sorted.len() as f32).ceil() as usize)
        .max(1)
        .min(sorted.len());
    Some(sorted[rank - 1])
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameLedger {
    pub intervals_us: LatencyStats,
    pub target_interval_us: u64,
    pub missed: u64,
}

impl FrameLedger {
    pub fn new(target_interval_us: u64) -> Self {
        Self {
            intervals_us: LatencyStats::default(),
            target_interval_us: target_interval_us.max(1),
            missed: 0,
        }
    }

    pub fn push_interval(&mut self, interval_us: u64) {
        if interval_us > self.target_interval_us.saturating_mul(3) / 2 {
            self.missed += 1;
        }
        self.intervals_us.push(interval_us);
    }

    pub fn p99_ok(&self) -> bool {
        let budget = (self.target_interval_us as f32 * INTERVAL_FACTOR) as u64;
        self.intervals_us
            .p99()
            .map(|p99| p99 <= budget)
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionReport {
    pub tier: String,
    pub codec: String,
    pub software: bool,
    pub frames_encoded: u64,
    pub frames_sent: u64,
    pub frames_dropped: u64,
    pub keyframes: u64,
    pub bytes_sent: u64,
    pub encode_us_p50: Option<u64>,
    pub encode_us_p99: Option<u64>,
    pub capture_to_send_us_p50: Option<u64>,
    pub capture_to_send_us_p99: Option<u64>,
    pub interval_us_p50: Option<u64>,
    pub interval_us_p99: Option<u64>,
    pub target_interval_us: u64,
    pub interval_budget_ok: bool,
    pub capture_budget_ok: bool,
    pub max_queue: u32,
    pub copied_bytes: u64,
    pub cpu_frames: u64,
    pub dma_frames: u64,
    pub rate_kbps: u32,
    pub glass_to_glass_budget_ms: f32,
    pub note: String,
}

impl SessionReport {
    pub fn summary_line(&self) -> String {
        format!(
            "{} {}: {} frames, {} dropped, encode p99 {:.2} ms, interval p99 {:.2} ms, {} copied bytes",
            self.tier,
            self.codec,
            self.frames_sent,
            self.frames_dropped,
            self.encode_us_p99.unwrap_or(0) as f32 / 1_000.0,
            self.interval_us_p99.unwrap_or(0) as f32 / 1_000.0,
            self.copied_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_nearest_rank() {
        let mut stats = LatencyStats::default();
        for value in 1..=100 {
            stats.push(value);
        }
        assert_eq!(stats.p50(), Some(50));
        assert_eq!(stats.p99(), Some(99));
        assert_eq!(stats.max(), Some(100));
        assert_eq!(stats.mean(), Some(50.5));
        assert_eq!(LatencyStats::default().p99(), None);
    }

    #[test]
    fn frame_ledger_marks_misses_and_enforces_the_factor() {
        let mut ledger = FrameLedger::new(16_667);
        for _ in 0..99 {
            ledger.push_interval(16_667);
        }
        ledger.push_interval(30_000);
        assert_eq!(ledger.missed, 1);
        assert!(ledger.p99_ok());
        let mut bad = FrameLedger::new(16_667);
        for _ in 0..99 {
            bad.push_interval(30_000);
        }
        assert!(!bad.p99_ok());
    }
}
