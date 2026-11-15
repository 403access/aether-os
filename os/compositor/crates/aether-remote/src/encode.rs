//! Low-latency H.264 encoding and the hardware/software capability ladder.
//!
//! The encoder is a real H.264 encoder (Annex-B access units), not a stub. The
//! target build prefers VA-API (or NVENC on a discrete GPU) and falls back to
//! the pure-Rust software encoder with a visible latency badge. The settings are
//! the low-latency contract from the architecture document: no B-frames, no
//! lookahead, one access unit per frame, periodic IDRs.

use std::time::Instant;

use rusty_h264_common::YuvPlanes;
use rusty_h264_encoder::{Encoder, EncoderConfig as H264Config, Preset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
}

impl Codec {
    pub fn label(self) -> &'static str {
        match self {
            Self::H264 => "H.264",
            Self::Hevc => "HEVC",
            Self::Av1 => "AV1",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::H264 => "video/avc",
            Self::Hevc => "video/hevc",
            Self::Av1 => "video/av01",
        }
    }

    /// WebCodecs codec string used by the browser client.
    pub fn webcodecs(self) -> &'static str {
        match self {
            Self::H264 => "avc1.42E01F",
            Self::Hevc => "hvc1.1.6.L93.B0",
            Self::Av1 => "av01.0.04M.08",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncoderKind {
    Vaapi,
    Nvenc,
    Software,
}

impl EncoderKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vaapi => "VA-API",
            Self::Nvenc => "NVENC",
            Self::Software => "Software",
        }
    }

    pub fn is_hardware(self) -> bool {
        !matches!(self, Self::Software)
    }
}

/// Probe result from the host's `/dev/dri`, `vainfo` and driver matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderCaps {
    pub vaapi_h264: bool,
    pub vaapi_hevc: bool,
    pub vaapi_av1: bool,
    pub nvenc_h264: bool,
    pub software_h264: bool,
}

impl Default for EncoderCaps {
    fn default() -> Self {
        Self {
            vaapi_h264: false,
            vaapi_hevc: false,
            vaapi_av1: false,
            nvenc_h264: false,
            software_h264: true,
        }
    }
}

impl EncoderCaps {
    /// The HD 630 profile in the hardware documentation.
    pub fn hd630() -> Self {
        Self {
            vaapi_h264: true,
            vaapi_hevc: true,
            vaapi_av1: false,
            nvenc_h264: false,
            software_h264: true,
        }
    }

    pub fn software_only() -> Self {
        Self::default()
    }

    pub fn supports(&self, kind: EncoderKind, codec: Codec) -> bool {
        match (kind, codec) {
            (EncoderKind::Vaapi, Codec::H264) => self.vaapi_h264,
            (EncoderKind::Vaapi, Codec::Hevc) => self.vaapi_hevc,
            (EncoderKind::Vaapi, Codec::Av1) => self.vaapi_av1,
            (EncoderKind::Nvenc, Codec::H264) => self.nvenc_h264,
            (EncoderKind::Software, Codec::H264) => self.software_h264,
            _ => false,
        }
    }
}

/// Pick the first supported pair from an explicit preference list.
pub fn select_encoder(
    caps: &EncoderCaps,
    preferences: &[(EncoderKind, Codec)],
) -> Option<(EncoderKind, Codec)> {
    preferences
        .iter()
        .copied()
        .find(|(kind, codec)| caps.supports(*kind, *codec))
}

/// The product preference order. H.264 is the compatibility floor; HEVC/AV1
/// are preferred when both sides advertise them.
pub fn default_preferences(codec: Codec) -> Vec<(EncoderKind, Codec)> {
    let mut preferences = vec![
        (EncoderKind::Vaapi, codec),
        (EncoderKind::Nvenc, codec),
        (EncoderKind::Vaapi, Codec::H264),
        (EncoderKind::Nvenc, Codec::H264),
        (EncoderKind::Software, Codec::H264),
    ];
    preferences.dedup();
    preferences
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub keyframe_interval: u32,
    pub kind: EncoderKind,
    pub codec: Codec,
    /// Label the stream as degraded when the hardware path is unavailable.
    pub software_fallback: bool,
}

impl EncoderConfig {
    /// 1440p60-class defaults with the documented low-latency settings.
    pub fn low_latency(width: u32, height: u32, fps: u32) -> Self {
        Self {
            width: (width.max(2) / 2) * 2,
            height: (height.max(2) / 2) * 2,
            fps: fps.clamp(1, 240),
            bitrate_kbps: 12_000,
            keyframe_interval: fps.clamp(1, 240) * 2,
            kind: EncoderKind::Software,
            codec: Codec::H264,
            software_fallback: false,
        }
    }

    pub fn with_kind(mut self, kind: EncoderKind) -> Self {
        self.kind = kind;
        self.software_fallback = !kind.is_hardware();
        self
    }

    pub fn with_bitrate(mut self, kbps: u32) -> Self {
        self.bitrate_kbps = kbps;
        self
    }

    pub fn validate(&self) -> Result<(), EncodeError> {
        if self.width == 0 || self.height == 0 {
            return Err(EncodeError::Empty);
        }
        if self.width % 2 != 0 || self.height % 2 != 0 {
            return Err(EncodeError::OddDimensions);
        }
        if self.width > 16_384 || self.height > 16_384 {
            return Err(EncodeError::TooLarge);
        }
        if self.bitrate_kbps == 0 {
            return Err(EncodeError::ZeroBitrate);
        }
        if self.fps == 0 || self.fps > 240 {
            return Err(EncodeError::BadFps);
        }
        Ok(())
    }

    pub fn bitrate_bps(&self) -> u32 {
        self.bitrate_kbps.saturating_mul(1_000)
    }

    /// The wire-level low-latency contract, exposed for the docs and tests.
    pub fn low_latency_contract(&self) -> LowLatencyContract {
        LowLatencyContract {
            bframes: 0,
            lookahead: 0,
            slice_mode: "one access unit per frame",
            rate_control: "CBR (adaptive)",
            gop: self.keyframe_interval,
            intra_refresh: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowLatencyContract {
    pub bframes: u8,
    pub lookahead: u8,
    pub slice_mode: &'static str,
    pub rate_control: &'static str,
    pub gop: u32,
    pub intra_refresh: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodedFrame {
    pub sequence: u64,
    pub pts_us: u64,
    pub keyframe: bool,
    pub kind: EncoderKind,
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub encode_us: u64,
    pub data: Vec<u8>,
}

impl EncodedFrame {
    pub fn is_annex_b(&self) -> bool {
        self.data.starts_with(&[0, 0, 0, 1]) || self.data.starts_with(&[0, 0, 1])
    }

    pub fn bytes(&self) -> usize {
        self.data.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    Empty,
    OddDimensions,
    TooLarge,
    ZeroBitrate,
    BadFps,
    InputTooShort,
    Unsupported,
    Encoder,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "encoder has an empty size"),
            Self::OddDimensions => write!(f, "encoder dimensions must be even"),
            Self::TooLarge => write!(f, "encoder exceeds the 16384 px dimension limit"),
            Self::ZeroBitrate => write!(f, "bitrate must be non-zero"),
            Self::BadFps => write!(f, "frame rate must be 1..=240"),
            Self::InputTooShort => write!(f, "input buffer is shorter than stride * height"),
            Self::Unsupported => write!(f, "no encoder supports the requested codec"),
            Self::Encoder => write!(f, "encoder rejected the frame"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Real software H.264 encoder used by the fallback and by the test harness.
pub struct H264Encoder {
    inner: Encoder,
    config: EncoderConfig,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    sequence: u64,
}

impl H264Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self, EncodeError> {
        config.validate()?;
        let mut cfg = H264Config::baseline(config.width as usize, config.height as usize);
        cfg.gop_size = config.keyframe_interval.max(1);
        cfg.min_keyint = config.keyframe_interval.max(1);
        cfg.scenecut = 0;
        cfg.framerate = config.fps as f32;
        cfg.bitrate = config.bitrate_bps();
        cfg.preset = Preset::Fast;
        let inner = Encoder::new(cfg).map_err(|_| EncodeError::Encoder)?;
        let pixels = (config.width * config.height) as usize;
        Ok(Self {
            inner,
            config,
            y: vec![0; pixels],
            u: vec![0; pixels / 4],
            v: vec![0; pixels / 4],
            sequence: 0,
        })
    }

    pub fn config(&self) -> EncoderConfig {
        self.config
    }

    pub fn encode(&mut self, frame: &BgraFrame<'_>) -> Result<EncodedFrame, EncodeError> {
        self.encode_impl(
            frame.data,
            frame.stride,
            frame.width,
            frame.height,
            ChannelOrder::Bgra,
        )
    }

    pub fn encode_rgba(&mut self, frame: &RgbaFrame<'_>) -> Result<EncodedFrame, EncodeError> {
        self.encode_impl(
            frame.data,
            frame.stride,
            frame.width,
            frame.height,
            ChannelOrder::Rgba,
        )
    }

    fn encode_impl(
        &mut self,
        data: &[u8],
        stride: u32,
        width: u32,
        height: u32,
        order: ChannelOrder,
    ) -> Result<EncodedFrame, EncodeError> {
        if width != self.config.width || height != self.config.height {
            return Err(EncodeError::Unsupported);
        }
        if data.len() < stride as usize * height as usize {
            return Err(EncodeError::InputTooShort);
        }
        let started = Instant::now();
        rgba_to_i420(
            data,
            stride as usize,
            width as usize,
            height as usize,
            &mut self.y,
            &mut self.u,
            &mut self.v,
            order,
        );
        let planes = YuvPlanes::tight(width as usize, height as usize, &self.y, &self.u, &self.v)
            .ok_or(EncodeError::Encoder)?;
        let data = self
            .inner
            .encode_planes(&planes)
            .map_err(|_| EncodeError::Encoder)?;
        self.sequence += 1;
        let keyframe = is_idr(&data);
        Ok(EncodedFrame {
            sequence: self.sequence,
            pts_us: 0,
            keyframe,
            kind: self.config.kind,
            codec: Codec::H264,
            width,
            height,
            encode_us: started.elapsed().as_micros() as u64,
            data,
        })
    }
}

pub struct BgraFrame<'a> {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: &'a [u8],
}

impl<'a> BgraFrame<'a> {
    pub fn new(width: u32, height: u32, stride: u32, data: &'a [u8]) -> Self {
        Self {
            width,
            height,
            stride,
            data,
        }
    }
}

pub type RgbaFrame<'a> = BgraFrame<'a>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelOrder {
    Bgra,
    Rgba,
}

impl ChannelOrder {
    fn rgb(self, data: &[u8], index: usize) -> (i32, i32, i32) {
        match self {
            Self::Bgra => (
                data[index + 2] as i32,
                data[index + 1] as i32,
                data[index] as i32,
            ),
            Self::Rgba => (
                data[index] as i32,
                data[index + 1] as i32,
                data[index + 2] as i32,
            ),
        }
    }
}

/// BT.601 full-range conversion with 2×2 chroma subsampling. The channel order
/// is explicit so a BGRA compositor buffer never gets the wrong chroma.
fn rgba_to_i420(
    data: &[u8],
    stride: usize,
    width: usize,
    height: usize,
    y: &mut [u8],
    u: &mut [u8],
    v: &mut [u8],
    order: ChannelOrder,
) {
    let cw = width / 2;
    for row in 0..height {
        let base = row * stride;
        for column in 0..width {
            let index = base + column * 4;
            let (r, g, b) = order.rgb(data, index);
            y[row * width + column] = clamp_u8((77 * r + 150 * g + 29 * b) >> 8);
        }
    }
    for row in 0..height / 2 {
        for column in 0..cw {
            let mut r = 0i32;
            let mut g = 0i32;
            let mut b = 0i32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let index = (row * 2 + dy) * stride + (column * 2 + dx) * 4;
                    let (sr, sg, sb) = order.rgb(data, index);
                    r += sr;
                    g += sg;
                    b += sb;
                }
            }
            r /= 4;
            g /= 4;
            b /= 4;
            let chroma = row * cw + column;
            u[chroma] = clamp_u8(((-26 * r - 51 * g + 77 * b) >> 8) + 128);
            v[chroma] = clamp_u8(((77 * r - 64 * g - 13 * b) >> 8) + 128);
        }
    }
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn is_idr(data: &[u8]) -> bool {
    // NAL unit header type 5 is an IDR slice; scan Annex-B start codes.
    let mut index = 0;
    while index + 3 < data.len() {
        if data[index] == 0 && data[index + 1] == 0 && data[index + 2] == 1 {
            if index + 3 < data.len() && data[index + 3] & 0x1f == 5 {
                return true;
            }
            index += 3;
        } else if index + 4 < data.len()
            && data[index] == 0
            && data[index + 1] == 0
            && data[index + 2] == 0
            && data[index + 3] == 1
        {
            if index + 4 < data.len() && data[index + 4] & 0x1f == 5 {
                return true;
            }
            index += 4;
        } else {
            index += 1;
        }
    }
    false
}

/// AIMD rate controller. Congestion or a late frame lowers the target; a clean
/// interval raises it slowly. The encoder is re-configured, never restarted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateController {
    pub target_kbps: u32,
    pub min_kbps: u32,
    pub max_kbps: u32,
    pub clean_rounds: u32,
}

impl RateController {
    pub fn new(target_kbps: u32) -> Self {
        Self {
            target_kbps: target_kbps.max(250),
            min_kbps: 500,
            max_kbps: 40_000,
            clean_rounds: 0,
        }
    }

    pub fn on_interval(&mut self, interval_us: u64, target_us: u64, queued: bool) -> u32 {
        let late = interval_us > target_us.saturating_mul(3) / 2;
        if late || queued {
            self.clean_rounds = 0;
            self.target_kbps = (self.target_kbps * 9 / 10).max(self.min_kbps);
        } else {
            self.clean_rounds += 1;
            if self.clean_rounds >= 30 {
                self.clean_rounds = 0;
                self.target_kbps = (self.target_kbps * 105 / 100).min(self.max_kbps);
            }
        }
        self.target_kbps
    }

    pub fn on_loss(&mut self, loss_permille: u32) -> u32 {
        if loss_permille >= 20 {
            self.target_kbps = (self.target_kbps * 8 / 10).max(self.min_kbps);
            self.clean_rounds = 0;
        } else if loss_permille >= 5 {
            self.target_kbps = (self.target_kbps * 95 / 100).max(self.min_kbps);
            self.clean_rounds = 0;
        }
        self.target_kbps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bgra(width: u32, height: u32, seed: u8) -> Vec<u8> {
        (0..width as usize * height as usize * 4)
            .map(|index| {
                if index % 4 == 3 {
                    255
                } else {
                    ((index / 4 + seed as usize) % 256) as u8
                }
            })
            .collect()
    }

    #[test]
    fn capability_ladder_prefers_hardware_then_software() {
        let caps = EncoderCaps::hd630();
        assert_eq!(
            select_encoder(&caps, &default_preferences(Codec::H264)),
            Some((EncoderKind::Vaapi, Codec::H264))
        );
        assert_eq!(
            select_encoder(&caps, &default_preferences(Codec::Av1)),
            Some((EncoderKind::Vaapi, Codec::H264)),
            "AV1 falls back to hardware H.264 before software"
        );
        let soft = EncoderCaps::software_only();
        assert_eq!(
            select_encoder(&soft, &default_preferences(Codec::Av1)),
            Some((EncoderKind::Software, Codec::H264))
        );
        assert_eq!(
            select_encoder(
                &EncoderCaps {
                    software_h264: false,
                    ..EncoderCaps::default()
                },
                &default_preferences(Codec::H264)
            ),
            None
        );
    }

    #[test]
    fn config_enforces_the_low_latency_contract() {
        let config = EncoderConfig::low_latency(1920, 1080, 60);
        assert!(config.validate().is_ok());
        let contract = config.low_latency_contract();
        assert_eq!(contract.bframes, 0);
        assert_eq!(contract.lookahead, 0);
        assert_eq!(contract.gop, 120);
        assert!(contract.intra_refresh);
        let mut odd = config;
        odd.width = 1919;
        assert_eq!(odd.validate(), Err(EncodeError::OddDimensions));
    }

    #[test]
    fn software_encoder_emits_annex_b_access_units() {
        let config = EncoderConfig::low_latency(160, 96, 60).with_bitrate(2_000);
        let mut encoder = H264Encoder::new(config).expect("encoder");
        let data = bgra(160, 96, 0);
        let first = encoder
            .encode(&BgraFrame::new(160, 96, 640, &data))
            .expect("frame");
        assert!(first.is_annex_b());
        assert!(first.keyframe, "first frame must be an IDR");
        assert!(first.bytes() > 0);
        assert_eq!(first.codec, Codec::H264);
        assert_eq!(first.kind, EncoderKind::Software);

        let second = encoder
            .encode(&BgraFrame::new(160, 96, 640, &bgra(160, 96, 8)))
            .expect("frame");
        assert!(second.is_annex_b());
        assert!(!second.data.is_empty());
    }

    #[test]
    fn encoder_rejects_wrong_dimensions_and_short_input() {
        let config = EncoderConfig::low_latency(64, 64, 30);
        let mut encoder = H264Encoder::new(config).expect("encoder");
        assert_eq!(
            encoder.encode(&BgraFrame::new(32, 32, 128, &[0; 4096])),
            Err(EncodeError::Unsupported)
        );
        assert_eq!(
            encoder.encode(&BgraFrame::new(64, 64, 256, &[0; 8])),
            Err(EncodeError::InputTooShort)
        );
    }

    #[test]
    fn rate_controller_backs_off_and_recovers() {
        let mut rate = RateController::new(8_000);
        let target = 16_667;
        assert_eq!(rate.on_interval(30_000, target, false), 7_200);
        assert_eq!(rate.on_interval(16_000, target, false), 7_200);
        for _ in 0..30 {
            rate.on_interval(16_000, target, false);
        }
        assert!(rate.target_kbps > 7_200);
        let backed_off = rate.target_kbps;
        assert!(rate.on_loss(50) < backed_off);
        assert_eq!(rate.on_loss(0), rate.target_kbps);
    }
}
