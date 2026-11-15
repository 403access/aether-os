//! Capture → encode → transport pipeline.
//!
//! The pipeline owns a single encoded-frame slot. A new capture replaces the
//! slot instead of queueing behind it; the transport sees the newest frame and
//! the dropped counter records the stale one. This is the no-queue rule the P7
//! gate checks.

use serde::{Deserialize, Serialize};

use crate::capture::{CaptureFrame, CopyAuditor, FramePayload};
use crate::encode::{
    BgraFrame, EncodeError, EncodedFrame, EncoderConfig, EncoderKind, H264Encoder, RateController,
};
use crate::ladder::Tier;
use crate::stats::{
    FrameLedger, LatencyStats, SessionReport, CAPTURE_TO_SEND_BUDGET_US, GLASS_TO_GLASS_BUDGET_MS,
};
use crate::transport::{Transport, TransportError, TransportStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub encoder: EncoderConfig,
    pub tier: Tier,
    pub target_fps: u32,
}

impl PipelineConfig {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self {
            encoder: EncoderConfig::low_latency(width, height, fps)
                .with_kind(EncoderKind::Software),
            tier: Tier::WebRtc,
            target_fps: fps.clamp(1, 240),
        }
    }

    pub fn target_interval_us(&self) -> u64 {
        1_000_000 / u64::from(self.target_fps.max(1))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineEvent {
    Encoded {
        sequence: u64,
        bytes: usize,
        keyframe: bool,
        encode_us: u64,
    },
    Dropped {
        sequence: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineError {
    Encode(EncodeError),
    Transport(TransportError),
    DmaBufImport,
    UnsupportedFormat,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(error) => write!(f, "encode: {error}"),
            Self::Transport(error) => write!(f, "transport: {error}"),
            Self::DmaBufImport => write!(
                f,
                "DMA-BUF import needs the compositor/VA-API binding; the software reader cannot map it"
            ),
            Self::UnsupportedFormat => write!(f, "unsupported capture format"),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Reads a capture into the software encoder. The target binding replaces this
/// with a VA-API import that never maps pixels on the CPU.
pub trait FrameReader {
    fn read_bgra(&mut self, frame: &CaptureFrame) -> Result<Vec<u8>, PipelineError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CpuFrameReader;

impl FrameReader for CpuFrameReader {
    fn read_bgra(&mut self, frame: &CaptureFrame) -> Result<Vec<u8>, PipelineError> {
        match &frame.payload {
            FramePayload::Cpu(cpu) => Ok(cpu.data.clone()),
            FramePayload::DmaBuf(_) => Err(PipelineError::DmaBufImport),
        }
    }
}

pub struct Pipeline<T: Transport, R: FrameReader = CpuFrameReader> {
    config: PipelineConfig,
    encoder: H264Encoder,
    reader: R,
    transport: T,
    rate: RateController,
    auditor: CopyAuditor,
    slot: Option<EncodedFrame>,
    last_capture_us: Option<u64>,
    encode_us: LatencyStats,
    capture_to_send_us: LatencyStats,
    intervals: FrameLedger,
    frames_encoded: u64,
    frames_dropped: u64,
    keyframes: u64,
}

impl<T: Transport> Pipeline<T, CpuFrameReader> {
    pub fn new(config: PipelineConfig, transport: T) -> Result<Self, PipelineError> {
        Self::with_reader(config, transport, CpuFrameReader)
    }
}

impl<T: Transport, R: FrameReader> Pipeline<T, R> {
    pub fn with_reader(
        config: PipelineConfig,
        transport: T,
        reader: R,
    ) -> Result<Self, PipelineError> {
        let encoder = H264Encoder::new(config.encoder).map_err(PipelineError::Encode)?;
        let rate = RateController::new(config.encoder.bitrate_kbps);
        let intervals = FrameLedger::new(config.target_interval_us());
        Ok(Self {
            config,
            encoder,
            reader,
            transport,
            rate,
            auditor: CopyAuditor::default(),
            slot: None,
            last_capture_us: None,
            encode_us: LatencyStats::default(),
            capture_to_send_us: LatencyStats::default(),
            intervals,
            frames_encoded: 0,
            frames_dropped: 0,
            keyframes: 0,
        })
    }

    pub fn push_capture(
        &mut self,
        frame: &CaptureFrame,
        now_us: u64,
    ) -> Result<PipelineEvent, PipelineError> {
        self.auditor.record(frame);
        if self.slot.take().is_some() {
            self.frames_dropped += 1;
        }
        let pixels = self.reader.read_bgra(frame)?;
        let data = BgraFrame::new(frame.width, frame.height, frame.width * 4, &pixels);
        let mut encoded = self.encoder.encode(&data).map_err(PipelineError::Encode)?;
        encoded.pts_us = frame.captured_at_us;
        encoded.sequence = frame.sequence;
        self.frames_encoded += 1;
        if encoded.keyframe {
            self.keyframes += 1;
        }
        self.encode_us.push(encoded.encode_us);
        if let Some(previous) = self.last_capture_us {
            self.intervals
                .push_interval(now_us.saturating_sub(previous));
        }
        self.last_capture_us = Some(now_us);
        let event = PipelineEvent::Encoded {
            sequence: encoded.sequence,
            bytes: encoded.bytes(),
            keyframe: encoded.keyframe,
            encode_us: encoded.encode_us,
        };
        self.slot = Some(encoded);
        Ok(event)
    }

    /// Send the newest encoded frame. Returns the sequence that left the host.
    pub fn tick(&mut self, now_us: u64) -> Option<u64> {
        let frame = self.slot.take()?;
        let sequence = frame.sequence;
        self.capture_to_send_us
            .push(now_us.saturating_sub(frame.pts_us));
        if let Err(_error) = self.transport.send_frame(&frame, now_us) {
            return None;
        }
        if let Some(interval) = self.intervals.intervals_us.samples().last().copied() {
            self.rate
                .on_interval(interval, self.config.target_interval_us(), false);
        }
        Some(sequence)
    }

    pub fn pending(&self) -> bool {
        self.slot.is_some()
    }

    pub fn auditor(&self) -> CopyAuditor {
        self.auditor
    }

    pub fn rate_kbps(&self) -> u32 {
        self.rate.target_kbps
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn report(&self) -> SessionReport {
        let stats: TransportStats = self.transport.stats();
        let software = self.config.encoder.kind == EncoderKind::Software;
        SessionReport {
            tier: self.config.tier.short().to_string(),
            codec: "h264".to_string(),
            software,
            frames_encoded: self.frames_encoded,
            frames_sent: stats.frames_sent,
            frames_dropped: self.frames_dropped + stats.frames_dropped,
            keyframes: self.keyframes,
            bytes_sent: stats.bytes_sent,
            encode_us_p50: self.encode_us.p50(),
            encode_us_p99: self.encode_us.p99(),
            capture_to_send_us_p50: self.capture_to_send_us.p50(),
            capture_to_send_us_p99: self.capture_to_send_us.p99(),
            interval_us_p50: self.intervals.intervals_us.p50(),
            interval_us_p99: self.intervals.intervals_us.p99(),
            target_interval_us: self.intervals.target_interval_us,
            interval_budget_ok: self.intervals.p99_ok(),
            capture_budget_ok: self
                .capture_to_send_us
                .p99()
                .map(|p99| p99 <= CAPTURE_TO_SEND_BUDGET_US)
                .unwrap_or(true),
            max_queue: stats.max_queue,
            copied_bytes: self.auditor.copied_bytes,
            cpu_frames: self.auditor.cpu_frames,
            dma_frames: self.auditor.dma_frames,
            rate_kbps: self.rate.target_kbps,
            glass_to_glass_budget_ms: GLASS_TO_GLASS_BUDGET_MS,
            note: if self.auditor.cpu_frames > 0 {
                "software capture; DMA-BUF import is target-only".to_string()
            } else {
                "dma-buf capture; no CPU pixel copies".to_string()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureSource;
    use crate::capture::{
        CaptureFrame, CpuFrame, DamageRect, DmaBufFrame, PixelFormat, SyntheticSource,
    };
    use crate::transport::LoopbackTransport;

    fn config() -> PipelineConfig {
        let mut config = PipelineConfig::new(160, 96, 60);
        config.encoder = config.encoder.with_bitrate(2_000);
        config
    }

    fn cpu_frame(sequence: u64) -> CaptureFrame {
        let mut source = SyntheticSource::new(160, 96, 60);
        let mut frame = source.next_frame(0).unwrap();
        frame.sequence = sequence;
        frame.captured_at_us = sequence * 16_667;
        frame
    }

    #[test]
    fn replaces_the_pending_frame_instead_of_queueing() {
        let mut pipeline = Pipeline::new(config(), LoopbackTransport::new()).unwrap();
        pipeline.push_capture(&cpu_frame(1), 0).unwrap();
        pipeline.push_capture(&cpu_frame(2), 16_667).unwrap();
        let report = pipeline.report();
        assert_eq!(report.frames_dropped, 1);
        assert_eq!(report.frames_encoded, 2);
        let sequence = pipeline.tick(20_000).expect("sent");
        assert_eq!(sequence, 2);
        assert_eq!(pipeline.transport().stats().frames_sent, 1);
        assert!(!pipeline.pending());
    }

    #[test]
    fn report_tracks_intervals_budgets_and_copies() {
        let mut pipeline = Pipeline::new(config(), LoopbackTransport::new()).unwrap();
        for index in 0..30u64 {
            pipeline
                .push_capture(&cpu_frame(index + 1), index * 16_667)
                .unwrap();
            let now = index * 16_667 + 1_000;
            pipeline.tick(now);
            pipeline.transport_mut().deliver(now + 2_000);
        }
        let report = pipeline.report();
        assert_eq!(report.frames_sent, 30);
        assert_eq!(report.frames_dropped, 0);
        assert_eq!(report.cpu_frames, 30);
        assert_eq!(report.dma_frames, 0);
        assert!(report.interval_budget_ok);
        assert!(report.capture_budget_ok);
        assert!(report.copied_bytes > 0);
        assert!(report.note.contains("target-only"));
        assert!(report.summary_line().contains("h264"));
    }

    #[test]
    fn dma_buf_frames_need_the_target_reader() {
        let mut pipeline = Pipeline::new(config(), LoopbackTransport::new()).unwrap();
        let frame = CaptureFrame {
            sequence: 1,
            captured_at_us: 0,
            output: 1,
            width: 160,
            height: 96,
            damage: vec![DamageRect::new(0, 0, 160, 96)],
            payload: FramePayload::DmaBuf(DmaBufFrame::new(160, 96, PixelFormat::Bgra, 640)),
        };
        assert_eq!(
            pipeline.push_capture(&frame, 0),
            Err(PipelineError::DmaBufImport)
        );
        assert_eq!(pipeline.auditor().dma_frames, 1);
        assert_eq!(pipeline.auditor().copied_bytes, 0);
    }

    #[test]
    fn cpu_frame_helper_is_valid() {
        let frame = cpu_frame(1);
        assert!(matches!(frame.payload, FramePayload::Cpu(CpuFrame { .. })));
        assert_eq!(frame.width, 160);
    }
}
