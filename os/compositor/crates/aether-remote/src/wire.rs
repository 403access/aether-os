//! Binary media framing for the WebSocket fallback tiers.
//!
//! WebRTC carries the product path. When UDP is unusable the same access units
//! travel over a WebSocket inside a small fixed header. The header is deliberately
//! boring: a browser can parse it in a few instructions and hand the payload to
//! `VideoDecoder` or a JPEG tile painter.

use serde::{Deserialize, Serialize};

use crate::encode::Codec;
use crate::ladder::Tier;

pub const MEDIA_MAGIC: [u8; 4] = *b"AEM1";
pub const MEDIA_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFrame {
    pub tier: Tier,
    pub codec: Codec,
    pub keyframe: bool,
    /// `true` for a JPEG tile payload; `false` for a codec access unit.
    pub jpeg: bool,
    pub sequence: u32,
    pub pts_us: u64,
    pub payload: Vec<u8>,
}

impl WireFrame {
    pub fn new(tier: Tier, codec: Codec, sequence: u32, pts_us: u64, payload: Vec<u8>) -> Self {
        Self {
            tier,
            codec,
            keyframe: false,
            jpeg: matches!(tier, Tier::JpegTiles),
            sequence,
            pts_us,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len());
        bytes.extend_from_slice(&MEDIA_MAGIC);
        bytes.push(MEDIA_VERSION);
        bytes.push(tier_byte(self.tier));
        bytes.push(codec_byte(self.codec));
        bytes.push(if self.keyframe { 1 } else { 0 } | if self.jpeg { 2 } else { 0 });
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&self.pts_us.to_le_bytes());
        bytes.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() < HEADER_LEN {
            return Err(WireError::TooShort);
        }
        if bytes[..4] != MEDIA_MAGIC {
            return Err(WireError::Magic);
        }
        if bytes[4] != MEDIA_VERSION {
            return Err(WireError::Version(bytes[4]));
        }
        let tier = tier_from_byte(bytes[5])?;
        let codec = codec_from_byte(bytes[6])?;
        let flags = bytes[7];
        let sequence = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let pts_us = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        let length = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
        if bytes.len() < HEADER_LEN + length {
            return Err(WireError::Truncated {
                expected: HEADER_LEN + length,
                got: bytes.len(),
            });
        }
        Ok(Self {
            tier,
            codec,
            keyframe: flags & 1 != 0,
            jpeg: flags & 2 != 0,
            sequence,
            pts_us,
            payload: bytes[HEADER_LEN..HEADER_LEN + length].to_vec(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    TooShort,
    Magic,
    Version(u8),
    UnknownTier(u8),
    UnknownCodec(u8),
    Truncated { expected: usize, got: usize },
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "media frame is shorter than the header"),
            Self::Magic => write!(f, "media frame magic is not AEM1"),
            Self::Version(version) => write!(f, "unsupported media frame version {version}"),
            Self::UnknownTier(tier) => write!(f, "unknown transport tier {tier}"),
            Self::UnknownCodec(codec) => write!(f, "unknown codec {codec}"),
            Self::Truncated { expected, got } => {
                write!(f, "media frame truncated: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for WireError {}

fn tier_byte(tier: Tier) -> u8 {
    match tier {
        Tier::WebRtc => 1,
        Tier::WebSocketAccessUnits => 2,
        Tier::JpegTiles => 3,
        Tier::Vnc => 4,
    }
}

fn tier_from_byte(byte: u8) -> Result<Tier, WireError> {
    match byte {
        1 => Ok(Tier::WebRtc),
        2 => Ok(Tier::WebSocketAccessUnits),
        3 => Ok(Tier::JpegTiles),
        4 => Ok(Tier::Vnc),
        other => Err(WireError::UnknownTier(other)),
    }
}

fn codec_byte(codec: Codec) -> u8 {
    match codec {
        Codec::H264 => 1,
        Codec::Hevc => 2,
        Codec::Av1 => 3,
    }
}

fn codec_from_byte(byte: u8) -> Result<Codec, WireError> {
    match byte {
        1 => Ok(Codec::H264),
        2 => Ok(Codec::Hevc),
        3 => Ok(Codec::Av1),
        other => Err(WireError::UnknownCodec(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_an_access_unit() {
        let mut frame = WireFrame::new(
            Tier::WebSocketAccessUnits,
            Codec::H264,
            7,
            123_456,
            vec![0, 0, 0, 1, 9],
        );
        frame.keyframe = true;
        let bytes = frame.encode();
        assert_eq!(&bytes[..4], b"AEM1");
        assert_eq!(bytes.len(), HEADER_LEN + 5);
        assert_eq!(WireFrame::decode(&bytes).unwrap(), frame);
    }

    #[test]
    fn marks_jpeg_tiles_from_the_tier() {
        let frame = WireFrame::new(Tier::JpegTiles, Codec::H264, 1, 0, vec![255, 216, 255]);
        assert!(frame.jpeg);
        let decoded = WireFrame::decode(&frame.encode()).unwrap();
        assert!(decoded.jpeg);
        assert_eq!(decoded.tier, Tier::JpegTiles);
    }

    #[test]
    fn rejects_bad_magic_version_and_lengths() {
        let frame = WireFrame::new(Tier::WebRtc, Codec::H264, 1, 0, vec![1, 2, 3]);
        let mut bytes = frame.encode();
        assert_eq!(
            WireFrame::decode(&bytes[..8]).unwrap_err(),
            WireError::TooShort
        );
        bytes[0] = b'X';
        assert_eq!(WireFrame::decode(&bytes).unwrap_err(), WireError::Magic);
        let mut bytes = frame.encode();
        bytes[4] = 9;
        assert_eq!(
            WireFrame::decode(&bytes).unwrap_err(),
            WireError::Version(9)
        );
        let mut bytes = frame.encode();
        bytes[20] = 9;
        assert!(matches!(
            WireFrame::decode(&bytes).unwrap_err(),
            WireError::Truncated { .. }
        ));
    }
}
