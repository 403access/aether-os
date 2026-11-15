//! Capture descriptors and the CPU-copy audit.
//!
//! A remote frame is either a set of exported DMA-BUF planes (the product path)
//! or a CPU buffer (the documented software fallback). The descriptor is
//! deliberately transport-neutral: the compositor fills it after rendering and
//! the encoder imports the planes without touching pixels.

use serde::{Deserialize, Serialize};

/// Pixel formats the capture and encode path understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    Bgra,
    Rgba,
    Nv12,
    I420,
}

impl PixelFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bgra => "BGRA8888",
            Self::Rgba => "RGBA8888",
            Self::Nv12 => "NV12",
            Self::I420 => "I420",
        }
    }

    /// Bytes per pixel for the packed formats; planar formats report their
    /// luma bytes per pixel and are validated separately.
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Bgra | Self::Rgba => 4,
            Self::Nv12 | Self::I420 => 1,
        }
    }

    pub fn is_planar(self) -> bool {
        matches!(self, Self::Nv12 | Self::I420)
    }

    /// Minimum number of planes this format needs.
    pub fn min_planes(self) -> usize {
        match self {
            Self::Bgra | Self::Rgba => 1,
            Self::Nv12 => 2,
            Self::I420 => 3,
        }
    }
}

/// One DMA-BUF plane. `fd` is intentionally not stored: the compositor owns the
/// file descriptor and the encoder imports it through a capability-scoped handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmaBufPlane {
    pub offset: u64,
    pub stride: u32,
    pub len: u64,
}

/// DRM modifier for a linear buffer. Compositors may export tiled buffers; the
/// encoder rejects modifiers it cannot import instead of falling back silently.
pub const DRM_FORMAT_MOD_LINEAR: u64 = 0;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmaBufFrame {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub modifier: u64,
    pub planes: Vec<DmaBufPlane>,
    /// Set by the capture backend after the buffer round-trip succeeds.
    pub exported: bool,
    /// Sequence of the compositor frame this descriptor came from.
    pub compositor_sequence: u64,
}

impl DmaBufFrame {
    pub fn new(width: u32, height: u32, format: PixelFormat, stride: u32) -> Self {
        let bpp = format.bytes_per_pixel();
        let plane_len = u64::from(stride) * u64::from(height);
        Self {
            width,
            height,
            format,
            modifier: DRM_FORMAT_MOD_LINEAR,
            planes: vec![DmaBufPlane {
                offset: 0,
                stride,
                len: plane_len.max(u64::from(width) * u64::from(bpp)),
            }],
            exported: true,
            compositor_sequence: 0,
        }
    }

    /// Validate the descriptor before an import. A malformed descriptor is a
    /// capture bug; the encoder must not guess at strides.
    pub fn validate(&self) -> Result<(), CaptureError> {
        if self.width == 0 || self.height == 0 {
            return Err(CaptureError::Empty);
        }
        if self.width > 16_384 || self.height > 16_384 {
            return Err(CaptureError::TooLarge);
        }
        if self.planes.len() < self.format.min_planes() {
            return Err(CaptureError::MissingPlanes {
                expected: self.format.min_planes(),
                got: self.planes.len(),
            });
        }
        if self.format.is_planar()
            && (!self.width.is_multiple_of(2) || !self.height.is_multiple_of(2))
        {
            return Err(CaptureError::OddPlanarDimensions);
        }
        let row_bytes = self.width * self.format.bytes_per_pixel();
        for (index, plane) in self.planes.iter().enumerate() {
            if plane.stride < row_bytes && index == 0 {
                return Err(CaptureError::StrideTooSmall {
                    index,
                    stride: plane.stride,
                    row_bytes,
                });
            }
            let rows = if index == 0 {
                self.height
            } else {
                self.height.div_ceil(2)
            };
            let required = u64::from(plane.stride) * u64::from(rows);
            if plane.len < required {
                return Err(CaptureError::PlaneTooShort {
                    index,
                    len: plane.len,
                    required,
                });
            }
        }
        Ok(())
    }

    /// Bytes that would be copied if this frame took the CPU fallback.
    pub fn copy_bytes(&self) -> u64 {
        self.planes.iter().map(|plane| plane.len).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuFrame {
    pub format: PixelFormat,
    pub stride: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FramePayload {
    DmaBuf(DmaBufFrame),
    Cpu(CpuFrame),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DamageRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub fn clamp_to(self, width: u32, height: u32) -> Option<Self> {
        let x0 = self.x.max(0) as i64;
        let y0 = self.y.max(0) as i64;
        let x1 = (self.x as i64 + self.width as i64).min(width as i64);
        let y1 = (self.y as i64 + self.height as i64).min(height as i64);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Self {
            x: x0 as i32,
            y: y0 as i32,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        })
    }

    pub fn union(self, other: Self) -> Self {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.width as i32).max(other.x + other.width as i32);
        let y1 = (self.y + self.height as i32).max(other.y + other.height as i32);
        Self {
            x: x0,
            y: y0,
            width: (x1 - x0).max(0) as u32,
            height: (y1 - y0).max(0) as u32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrame {
    pub sequence: u64,
    pub captured_at_us: u64,
    pub output: u32,
    pub width: u32,
    pub height: u32,
    pub damage: Vec<DamageRect>,
    pub payload: FramePayload,
}

impl CaptureFrame {
    pub fn is_zero_copy(&self) -> bool {
        matches!(&self.payload, FramePayload::DmaBuf(frame) if frame.exported)
    }

    pub fn copy_bytes(&self) -> u64 {
        match &self.payload {
            FramePayload::DmaBuf(frame) => {
                if frame.exported {
                    0
                } else {
                    frame.copy_bytes()
                }
            }
            FramePayload::Cpu(frame) => frame.data.len() as u64,
        }
    }

    pub fn format(&self) -> PixelFormat {
        match &self.payload {
            FramePayload::DmaBuf(frame) => frame.format,
            FramePayload::Cpu(frame) => frame.format,
        }
    }

    pub fn damage_area(&self) -> u64 {
        self.damage.iter().map(|rect| rect.area()).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureError {
    Empty,
    TooLarge,
    MissingPlanes {
        expected: usize,
        got: usize,
    },
    OddPlanarDimensions,
    StrideTooSmall {
        index: usize,
        stride: u32,
        row_bytes: u32,
    },
    PlaneTooShort {
        index: usize,
        len: u64,
        required: u64,
    },
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "capture has an empty size"),
            Self::TooLarge => write!(f, "capture exceeds the 16384 px dimension limit"),
            Self::MissingPlanes { expected, got } => {
                write!(f, "expected {expected} planes, got {got}")
            }
            Self::OddPlanarDimensions => write!(f, "planar capture dimensions must be even"),
            Self::StrideTooSmall {
                index,
                stride,
                row_bytes,
            } => write!(
                f,
                "plane {index} stride {stride} is smaller than its row ({row_bytes} bytes)"
            ),
            Self::PlaneTooShort {
                index,
                len,
                required,
            } => write!(f, "plane {index} is {len} bytes, needs {required}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// The compositor-facing capture boundary.
pub trait CaptureSource {
    fn next_frame(&mut self, now_us: u64) -> Option<CaptureFrame>;
}

/// Audit every frame that enters the pipeline. The gate is zero CPU copies in
/// the primary path; the software fallback is allowed but must stay visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyAuditor {
    pub frames: u64,
    pub dma_frames: u64,
    pub cpu_frames: u64,
    pub copied_bytes: u64,
}

impl CopyAuditor {
    pub fn record(&mut self, frame: &CaptureFrame) {
        self.frames += 1;
        if frame.is_zero_copy() {
            self.dma_frames += 1;
        } else {
            self.cpu_frames += 1;
            self.copied_bytes += frame.copy_bytes();
        }
    }

    pub fn zero_copy(&self) -> bool {
        self.cpu_frames == 0
    }

    pub fn summary(&self) -> String {
        format!(
            "{} frames: {} dma-buf, {} cpu, {} bytes copied",
            self.frames, self.dma_frames, self.cpu_frames, self.copied_bytes
        )
    }
}

/// Deterministic BGRA test source used by the harness, the browser demo and the
/// screenshots. It is explicitly a source, not a compositor capture: the real
/// backend swaps it for exported planes.
pub struct SyntheticSource {
    width: u32,
    height: u32,
    fps: u32,
    sequence: u64,
    started_us: u64,
    cache: Vec<u8>,
}

impl SyntheticSource {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        let width = (width.max(2) / 2) * 2;
        let height = (height.max(2) / 2) * 2;
        Self {
            width,
            height,
            fps: fps.max(1),
            sequence: 0,
            started_us: 0,
            cache: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render(&mut self, sequence: u64) {
        let width = self.width as usize;
        let height = self.height as usize;
        let phase = (sequence as f32 * 0.045).rem_euclid(1.0);
        for y in 0..height {
            let v = y as f32 / height as f32;
            for x in 0..width {
                let u = x as f32 / width as f32;
                let wave = ((u * 6.0 + phase * std::f32::consts::TAU).sin() * 0.5 + 0.5) * 0.25;
                let index = (y * width + x) * 4;
                self.cache[index] = ((8.0 + 24.0 * v + 40.0 * wave) as u8).max(4);
                self.cache[index + 1] = ((24.0 + 78.0 * (1.0 - v) + 26.0 * wave) as u8).max(4);
                self.cache[index + 2] = ((40.0 + 120.0 * (1.0 - v) + 60.0 * wave) as u8).max(4);
                self.cache[index + 3] = 255;
            }
        }
        // Moving highlight: makes frame-to-frame damage small and obvious.
        let band_x = ((phase * width as f32) as usize).min(width.saturating_sub(1));
        let band_w = (width / 16).max(2);
        for y in 0..height {
            for x in band_x..(band_x + band_w).min(width) {
                let index = (y * width + x) * 4;
                self.cache[index] = 56;
                self.cache[index + 1] = 214;
                self.cache[index + 2] = 255;
            }
        }
    }
}

impl CaptureSource for SyntheticSource {
    fn next_frame(&mut self, now_us: u64) -> Option<CaptureFrame> {
        if self.started_us == 0 {
            self.started_us = now_us;
        }
        let frame_interval_us = 1_000_000 / u64::from(self.fps);
        let due = self.started_us + self.sequence * frame_interval_us;
        if now_us + 1_000 < due {
            return None;
        }
        self.sequence += 1;
        self.render(self.sequence);
        let band_w = (self.width / 16).max(2);
        let band_x = (((self.sequence as f32 * 0.045).rem_euclid(1.0)) * self.width as f32) as i32;
        Some(CaptureFrame {
            sequence: self.sequence,
            captured_at_us: now_us.max(due),
            output: 1,
            width: self.width,
            height: self.height,
            damage: vec![DamageRect::new(band_x, 0, band_w, self.height)],
            payload: FramePayload::Cpu(CpuFrame {
                format: PixelFormat::Bgra,
                stride: self.width * 4,
                data: self.cache.clone(),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_a_linear_bgra_descriptor() {
        let frame = DmaBufFrame::new(640, 360, PixelFormat::Bgra, 640 * 4);
        assert!(frame.validate().is_ok());
        assert_eq!(frame.copy_bytes(), 640 * 360 * 4);
        assert_eq!(frame.planes.len(), 1);
    }

    #[test]
    fn rejects_a_short_stride_and_plane() {
        let mut frame = DmaBufFrame::new(640, 360, PixelFormat::Bgra, 640 * 4);
        frame.planes[0].stride = 128;
        assert!(matches!(
            frame.validate(),
            Err(CaptureError::StrideTooSmall { index: 0, .. })
        ));

        let mut frame = DmaBufFrame::new(640, 360, PixelFormat::Bgra, 640 * 4);
        frame.planes[0].len = 100;
        assert!(matches!(
            frame.validate(),
            Err(CaptureError::PlaneTooShort { index: 0, .. })
        ));
    }

    #[test]
    fn requires_even_planar_dimensions() {
        let mut frame = DmaBufFrame::new(641, 360, PixelFormat::Nv12, 641);
        frame.planes.push(DmaBufPlane {
            offset: 0,
            stride: 641,
            len: 641 * 180,
        });
        assert_eq!(frame.validate(), Err(CaptureError::OddPlanarDimensions));
    }

    #[test]
    fn copy_audit_distinguishes_dma_and_cpu_frames() {
        let mut auditor = CopyAuditor::default();
        let dma = CaptureFrame {
            sequence: 1,
            captured_at_us: 0,
            output: 1,
            width: 4,
            height: 2,
            damage: vec![],
            payload: FramePayload::DmaBuf(DmaBufFrame::new(4, 2, PixelFormat::Bgra, 16)),
        };
        let cpu = CaptureFrame {
            sequence: 2,
            captured_at_us: 1,
            output: 1,
            width: 4,
            height: 2,
            damage: vec![],
            payload: FramePayload::Cpu(CpuFrame {
                format: PixelFormat::Bgra,
                stride: 16,
                data: vec![0; 32],
            }),
        };
        auditor.record(&dma);
        assert!(auditor.zero_copy());
        auditor.record(&cpu);
        assert!(!auditor.zero_copy());
        assert_eq!(auditor.dma_frames, 1);
        assert_eq!(auditor.cpu_frames, 1);
        assert_eq!(auditor.copied_bytes, 32);
    }

    #[test]
    fn synthetic_source_is_deterministic_and_paced() {
        let mut source = SyntheticSource::new(64, 36, 60);
        assert!(source.next_frame(0).is_some());
        assert!(source.next_frame(1).is_none());
        let frame = source.next_frame(16_667).expect("second frame");
        assert_eq!(frame.width, 64);
        assert_eq!(frame.height, 36);
        assert!(frame.damage_area() > 0);
        assert!(!frame.is_zero_copy());
        assert_ne!(frame.payload, source.next_frame(33_334).unwrap().payload);
    }

    #[test]
    fn damage_rects_clamp_and_union() {
        let clipped = DamageRect::new(-4, -3, 10, 8).clamp_to(100, 100).unwrap();
        assert_eq!(clipped, DamageRect::new(0, 0, 6, 5));
        assert!(DamageRect::new(120, 10, 10, 10)
            .clamp_to(100, 100)
            .is_none());
        let union = DamageRect::new(0, 0, 10, 10).union(DamageRect::new(5, 5, 10, 10));
        assert_eq!(union, DamageRect::new(0, 0, 15, 15));
    }
}
