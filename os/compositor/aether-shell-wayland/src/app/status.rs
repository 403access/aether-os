//! Status-bar sampling, frame timing and text metrics shared with the renderer.

use aether_text::text_width;

mod sample;

pub(crate) use aether_system::{cpu_times as read_cpu, CpuTimes};
pub use sample::{StatusSnapshot, SAMPLE_INTERVAL};

use super::App;

impl App {
    pub fn text_rasterizations(&self) -> u64 {
        self.text_rasterizations
    }

    pub fn text_px(scale: u32) -> f32 {
        super::text_style::text_px(scale)
    }

    pub fn text_spacing(scale: u32) -> f32 {
        super::text_style::text_spacing(scale)
    }

    pub fn text_bold(scale: u32) -> bool {
        super::text_style::text_bold(scale)
    }

    pub fn text_width(&self, text: &str, scale: u32) -> u32 {
        let bold = super::text_style::text_bold(scale);
        match self.font.as_ref() {
            Some(font) => {
                font.measure(
                    text,
                    Self::text_px(scale & 0xff),
                    bold,
                    Self::text_spacing(scale),
                )
                .0
            }
            None => text_width(text, scale.max(1)),
        }
    }

    pub fn font_is_loaded(&self) -> bool {
        self.font.is_some()
    }

    pub fn set_frame_time_ms(&mut self, frame_ms: f32) {
        self.frame_time_ms = frame_ms;
    }

    /// True when the status bar should be sampled and drawn. Fullscreen hides
    /// it and stops polling.
    pub fn status_visible(&self) -> bool {
        self.status_enabled
            && !self
                .shell
                .outputs()
                .iter()
                .all(|output| self.shell.fullscreen_window_on(output.id).is_some())
    }

    pub fn status(&self) -> &StatusSnapshot {
        &self.status
    }

    pub(crate) fn poll_status(&mut self) {
        // Sample only while a frame is already due. A timer-driven sample would
        // itself create damage and break the zero-frame idle gate; values
        // refresh on the next input-driven frame instead.
        if !self.status_visible()
            || !self.needs_frame()
            || self.status_last.elapsed() < SAMPLE_INTERVAL
        {
            return;
        }
        self.status = StatusSnapshot::sample(
            self.status_cpu,
            self.agent_sessions.session_count(),
            self.frame_time_ms,
        );
        self.status.remote = self.shell.remote_status().unwrap_or_default();
        self.status_cpu = read_cpu();
        self.status_last = std::time::Instant::now();
    }
}
