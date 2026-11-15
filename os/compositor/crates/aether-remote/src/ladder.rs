//! Transport tier and codec negotiation.
//!
//! The host picks the best tier the browser actually supports. The choice is
//! explicit and surfaced in the shell, so a degraded session is never disguised
//! as the low-latency path.

use serde::{Deserialize, Serialize};

use crate::encode::Codec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// WebRTC + WebCodecs. The product path.
    WebRtc,
    /// WebSocket + low-latency access units + WebCodecs. UDP is blocked.
    WebSocketAccessUnits,
    /// WebSocket + JPEG tiles. No WebCodecs.
    JpegTiles,
    /// VNC. A debugging tool, explicitly labelled.
    Vnc,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Self::WebRtc => "WebRTC + WebCodecs",
            Self::WebSocketAccessUnits => "WebSocket + access units",
            Self::JpegTiles => "WebSocket + JPEG tiles",
            Self::Vnc => "VNC (degraded)",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::WebRtc => "webrtc",
            Self::WebSocketAccessUnits => "ws-au",
            Self::JpegTiles => "ws-jpeg",
            Self::Vnc => "vnc",
        }
    }

    pub fn budget_ms(self) -> u32 {
        match self {
            Self::WebRtc => 40,
            Self::WebSocketAccessUnits => 120,
            Self::JpegTiles => 250,
            Self::Vnc => 400,
        }
    }

    pub fn degraded(self) -> bool {
        !matches!(self, Self::WebRtc)
    }
}

/// What the browser reports after probing its own globals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCaps {
    pub webrtc: bool,
    pub websocket: bool,
    pub webcodecs: bool,
    pub jpeg: bool,
    pub h264: bool,
    pub hevc: bool,
    pub av1: bool,
}

impl Default for ClientCaps {
    fn default() -> Self {
        Self {
            webrtc: false,
            websocket: false,
            webcodecs: false,
            jpeg: true,
            h264: false,
            hevc: false,
            av1: false,
        }
    }
}

impl ClientCaps {
    /// A current desktop Chromium/Firefox profile.
    pub fn modern() -> Self {
        Self {
            webrtc: true,
            websocket: true,
            webcodecs: true,
            jpeg: true,
            h264: true,
            hevc: false,
            av1: true,
        }
    }

    pub fn codecs(&self) -> Vec<Codec> {
        let mut codecs = Vec::new();
        if self.av1 {
            codecs.push(Codec::Av1);
        }
        if self.hevc {
            codecs.push(Codec::Hevc);
        }
        if self.h264 {
            codecs.push(Codec::H264);
        }
        codecs
    }

    fn video(&self) -> bool {
        self.webcodecs && (self.h264 || self.hevc || self.av1)
    }
}

pub fn select_tier(caps: &ClientCaps, udp_blocked: bool) -> Tier {
    if caps.webrtc && !udp_blocked && caps.video() {
        Tier::WebRtc
    } else if caps.websocket && caps.video() {
        Tier::WebSocketAccessUnits
    } else if caps.websocket && caps.jpeg {
        Tier::JpegTiles
    } else {
        Tier::Vnc
    }
}

/// Choose the host's most preferred codec the client can decode. H.264 is the
/// compatibility floor and is kept last in the preference list.
pub fn select_codec(host: &[Codec], client: &ClientCaps) -> Option<Codec> {
    let client_codecs = client.codecs();
    host.iter()
        .copied()
        .find(|codec| client_codecs.contains(codec))
}

/// Human-readable latency badge for the shell and the browser HUD.
pub fn latency_badge(tier: Tier, software: bool, p99_ms: Option<f32>) -> String {
    let mut badge = format!("{} · {}", tier.short(), tier.budget_ms());
    if software {
        badge.push_str(" · software");
    }
    if let Some(p99) = p99_ms {
        badge.push_str(&format!(" · p99 {p99:.1} ms"));
    }
    badge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_webrtc_only_when_udp_and_webcodecs_are_available() {
        let caps = ClientCaps::modern();
        assert_eq!(select_tier(&caps, false), Tier::WebRtc);
        assert_eq!(select_tier(&caps, true), Tier::WebSocketAccessUnits);
        assert!(Tier::WebRtc.budget_ms() < Tier::WebSocketAccessUnits.budget_ms());
        assert!(!Tier::WebRtc.degraded());
        assert!(Tier::JpegTiles.degraded());
    }

    #[test]
    fn degrades_to_jpeg_then_vnc() {
        let no_codec = ClientCaps {
            webcodecs: false,
            h264: false,
            hevc: false,
            av1: false,
            ..ClientCaps::modern()
        };
        assert_eq!(select_tier(&no_codec, false), Tier::JpegTiles);
        let no_ws = ClientCaps {
            websocket: false,
            ..no_codec
        };
        assert_eq!(select_tier(&no_ws, false), Tier::Vnc);
    }

    #[test]
    fn codec_negotiation_prefers_host_order_and_common_support() {
        let client = ClientCaps::modern();
        assert_eq!(
            select_codec(&[Codec::Av1, Codec::Hevc, Codec::H264], &client),
            Some(Codec::Av1)
        );
        assert_eq!(
            select_codec(&[Codec::Hevc, Codec::H264], &client),
            Some(Codec::H264)
        );
        let no_codec = ClientCaps {
            h264: false,
            hevc: false,
            av1: false,
            ..ClientCaps::modern()
        };
        assert_eq!(select_codec(&[Codec::H264], &no_codec), None);
    }

    #[test]
    fn badge_names_the_degraded_reason() {
        assert!(latency_badge(Tier::WebRtc, false, Some(18.2)).contains("p99 18.2 ms"));
        let badge = latency_badge(Tier::JpegTiles, true, None);
        assert!(badge.contains("ws-jpeg"));
        assert!(badge.contains("software"));
    }
}
