//! Frame pacing.
//!
//! The compositor targets a deadline, not a frame queue. If a frame is late it
//! is dropped and superseded; the next present re-anchors the deadline to the
//! actual present time so the error does not accumulate.

pub const NS_PER_SEC: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug)]
pub struct FramePacer {
    interval_ns: u64,
    margin_ns: u64,
    next_deadline_ns: Option<u64>,
    presented: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacingReport {
    pub interval_ns: u64,
    pub next_deadline_ns: Option<u64>,
    pub presented: u64,
    pub missed: u64,
}

impl FramePacer {
    /// Default margin is one seventh of the frame interval (~1.19 ms at 120 Hz).
    pub fn new(refresh_hz: u32) -> Self {
        assert!(refresh_hz > 0, "refresh rate must be positive");
        Self::with_margin(refresh_hz, NS_PER_SEC / u64::from(refresh_hz) / 7)
    }

    pub fn with_margin(refresh_hz: u32, margin_ns: u64) -> Self {
        assert!(refresh_hz > 0, "refresh rate must be positive");
        let interval_ns = (NS_PER_SEC / u64::from(refresh_hz)).max(1);
        Self {
            interval_ns,
            margin_ns: margin_ns.min(interval_ns),
            next_deadline_ns: None,
            presented: 0,
        }
    }

    pub fn interval_ns(&self) -> u64 {
        self.interval_ns
    }

    pub fn margin_ns(&self) -> u64 {
        self.margin_ns
    }

    pub fn deadline(&self) -> Option<u64> {
        self.next_deadline_ns
    }

    pub fn presented_frames(&self) -> u64 {
        self.presented
    }

    /// Record a present and schedule the next deadline.
    pub fn on_present(&mut self, now_ns: u64) {
        self.next_deadline_ns = Some(now_ns.saturating_add(self.interval_ns));
        self.presented += 1;
    }

    /// True when the next frame should be submitted now. The first call is
    /// always true so the shell can paint before the first deadline.
    pub fn should_present(&self, now_ns: u64) -> bool {
        match self.next_deadline_ns {
            None => true,
            Some(deadline) => now_ns.saturating_add(self.margin_ns) >= deadline,
        }
    }

    /// How many deadlines have passed since the last present.
    pub fn missed_deadlines(&self, now_ns: u64) -> u64 {
        match self.next_deadline_ns {
            None => 0,
            Some(deadline) if now_ns > deadline => (now_ns - deadline) / self.interval_ns + 1,
            Some(_) => 0,
        }
    }

    pub fn report(&self, now_ns: u64) -> PacingReport {
        PacingReport {
            interval_ns: self.interval_ns,
            next_deadline_ns: self.next_deadline_ns,
            presented: self.presented,
            missed: self.missed_deadlines(now_ns),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_for_common_rates() {
        assert_eq!(FramePacer::new(120).interval_ns(), 8_333_333);
        assert_eq!(FramePacer::new(60).interval_ns(), 16_666_666);
        assert_eq!(FramePacer::new(1_000).interval_ns(), 1_000_000);
    }

    #[test]
    fn first_frame_is_immediate() {
        let pacer = FramePacer::new(120);
        assert!(pacer.should_present(0));
        assert_eq!(pacer.missed_deadlines(1_000_000), 0);
    }

    #[test]
    fn next_frame_waits_for_the_deadline_minus_margin() {
        let mut pacer = FramePacer::with_margin(120, 1_200_000);
        pacer.on_present(0);
        assert!(!pacer.should_present(0));
        assert!(!pacer.should_present(7_000_000));
        assert!(pacer.should_present(7_133_333));
        assert!(pacer.should_present(8_333_333));
    }

    #[test]
    fn presenting_reanchors_the_deadline() {
        let mut pacer = FramePacer::with_margin(120, 0);
        pacer.on_present(0);
        pacer.on_present(10_000_000);
        assert_eq!(pacer.deadline(), Some(18_333_333));
        assert_eq!(pacer.presented_frames(), 2);
    }

    #[test]
    fn missed_deadlines_counts_without_queueing() {
        let mut pacer = FramePacer::with_margin(120, 0);
        pacer.on_present(0); // deadline 8_333_333
        assert_eq!(pacer.missed_deadlines(8_333_333), 0);
        assert_eq!(pacer.missed_deadlines(8_333_334), 1);
        assert_eq!(pacer.missed_deadlines(30_000_000), 3);
        // Presenting late supersedes the backlog rather than queueing it.
        pacer.on_present(30_000_000);
        assert_eq!(pacer.deadline(), Some(38_333_333));
        assert_eq!(pacer.missed_deadlines(30_000_000), 0);
    }

    #[test]
    fn report_carries_counters() {
        let mut pacer = FramePacer::new(60);
        pacer.on_present(1_000);
        let report = pacer.report(20_000_000);
        assert_eq!(report.presented, 1);
        assert!(report.missed >= 1);
        assert_eq!(report.interval_ns, 16_666_666);
    }
}
