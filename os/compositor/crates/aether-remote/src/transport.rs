//! Transport boundary: a loopback double for tests and a real str0m host.
//!
//! The pipeline never waits for the network. A frame is offered to the transport
//! and the transport either accepts it or drops the stale one; queueing is a
//! latency bug, not a feature.

use std::net::SocketAddr;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::media::{Direction, MediaKind, MediaTime, Mid};
use str0m::net::{Protocol, Receive};
use str0m::{Event, Input, Output, Rtc, RtcError};

use crate::encode::EncodedFrame;
use crate::input::InputEvent;
use crate::session::SessionError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportStats {
    pub frames_sent: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub input_sent: u64,
    pub queued: u32,
    pub max_queue: u32,
}

pub trait Transport {
    fn send_frame(&mut self, frame: &EncodedFrame, now_us: u64) -> Result<(), TransportError>;
    fn send_input(&mut self, event: &InputEvent) -> Result<(), TransportError>;
    fn stats(&self) -> TransportStats;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    Closed,
    QueueFull,
    Rtc,
    Encode,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "transport is closed"),
            Self::QueueFull => write!(f, "transport queue is full"),
            Self::Rtc => write!(f, "WebRTC transport error"),
            Self::Encode => write!(f, "transport encode error"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<SessionError> for TransportError {
    fn from(_: SessionError) -> Self {
        Self::Closed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveredFrame {
    pub sequence: u64,
    pub delivered_at_us: u64,
    pub data: Vec<u8>,
}

/// In-process transport used by the harness and unit tests.
#[derive(Clone, Debug, Default)]
pub struct LoopbackTransport {
    stats: TransportStats,
    pending: Option<(u64, Vec<u8>, u64)>,
    delivered: Vec<DeliveredFrame>,
}

impl LoopbackTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Deliver the pending frame when its simulated one-way latency expires.
    pub fn deliver(&mut self, now_us: u64) {
        let Some((sequence, data, due_us)) = self.pending.take() else {
            return;
        };
        if now_us < due_us {
            self.pending = Some((sequence, data, due_us));
            return;
        }
        self.delivered.push(DeliveredFrame {
            sequence,
            delivered_at_us: now_us,
            data,
        });
        self.stats.queued = 0;
    }

    pub fn delivered(&self) -> &[DeliveredFrame] {
        &self.delivered
    }

    pub fn drain(&mut self) -> Vec<DeliveredFrame> {
        std::mem::take(&mut self.delivered)
    }
}

impl Transport for LoopbackTransport {
    fn send_frame(&mut self, frame: &EncodedFrame, now_us: u64) -> Result<(), TransportError> {
        if self.pending.is_some() {
            self.stats.frames_dropped += 1;
        }
        self.pending = Some((frame.sequence, frame.data.clone(), now_us + 1_000));
        self.stats.frames_sent += 1;
        self.stats.bytes_sent += frame.data.len() as u64;
        self.stats.queued = 1;
        self.stats.max_queue = self.stats.max_queue.max(1);
        Ok(())
    }

    fn send_input(&mut self, _event: &InputEvent) -> Result<(), TransportError> {
        self.stats.input_sent += 1;
        Ok(())
    }

    fn stats(&self) -> TransportStats {
        self.stats
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Datagram {
    pub source: SocketAddr,
    pub destination: SocketAddr,
    pub contents: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebRtcEvent {
    Connected,
    IceFailed,
    ChannelOpen(String),
    Input(Vec<u8>),
    KeyframeRequest,
    Closed,
}

#[derive(Clone, Debug, Default)]
pub struct WebRtcPoll {
    pub datagrams: Vec<Datagram>,
    pub events: Vec<WebRtcEvent>,
    pub timeout: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WebRtcConfig {
    pub ice_lite: bool,
}

/// One side of a str0m session. The host offers video and the `input` data
/// channel; the browser answers. DTLS/SRTP is owned by str0m.
pub struct WebRtcHost {
    rtc: Rtc,
    pending: Option<SdpPendingOffer>,
    video_mid: Option<Mid>,
    channel: Option<str0m::channel::ChannelId>,
    fingerprint: String,
    connected: bool,
}

impl WebRtcHost {
    pub fn new(now: Instant, config: WebRtcConfig) -> Result<Self, RtcError> {
        let mut rtc = Rtc::builder().set_ice_lite(config.ice_lite).build(now);
        let fingerprint = rtc.direct_api().local_dtls_fingerprint().to_string();
        Ok(Self {
            rtc,
            pending: None,
            video_mid: None,
            channel: None,
            fingerprint,
            connected: false,
        })
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.rtc.is_connected()
    }

    pub fn add_host_candidate(&mut self, address: SocketAddr) -> Result<(), RtcError> {
        let candidate = str0m::Candidate::host(address, "udp")?;
        self.rtc.add_local_candidate(candidate);
        Ok(())
    }

    /// Host side: create the offer with one video m-line and one input channel.
    pub fn create_offer(&mut self) -> Result<String, WebRtcError> {
        let mut changes = self.rtc.sdp_api();
        let mid = changes.add_media(MediaKind::Video, Direction::SendOnly, None, None, None);
        let channel = changes.add_channel("input".to_string());
        let (offer, pending) = changes.apply().ok_or(WebRtcError::NoVideo)?;
        self.video_mid = Some(mid);
        self.channel = Some(channel);
        self.pending = Some(pending);
        serde_json::to_string(&offer).map_err(WebRtcError::Json)
    }

    /// Browser side used by the loopback test and the Rust demo harness.
    pub fn accept_offer(&mut self, sdp_json: &str) -> Result<String, WebRtcError> {
        let offer: SdpOffer = serde_json::from_str(sdp_json).map_err(WebRtcError::Json)?;
        let answer = self
            .rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcError::Rtc)?;
        serde_json::to_string(&answer).map_err(WebRtcError::Json)
    }

    pub fn accept_answer(&mut self, sdp_json: &str) -> Result<(), WebRtcError> {
        let answer: SdpAnswer = serde_json::from_str(sdp_json).map_err(WebRtcError::Json)?;
        let pending = self.pending.take().ok_or(WebRtcError::NoPendingOffer)?;
        self.rtc
            .sdp_api()
            .accept_answer(pending, answer)
            .map_err(WebRtcError::Rtc)
    }

    pub fn poll(&mut self, now: Instant) -> Result<WebRtcPoll, WebRtcError> {
        self.rtc
            .handle_input(Input::Timeout(now))
            .map_err(WebRtcError::Rtc)?;
        self.collect()
    }

    pub fn handle_datagram(
        &mut self,
        now: Instant,
        source: SocketAddr,
        destination: SocketAddr,
        contents: &[u8],
    ) -> Result<WebRtcPoll, WebRtcError> {
        let receive = Receive::new(Protocol::Udp, source, destination, contents)
            .map_err(|_| WebRtcError::Datagram)?;
        self.rtc
            .handle_input(Input::Receive(now, receive))
            .map_err(WebRtcError::Rtc)?;
        self.collect()
    }

    fn collect(&mut self) -> Result<WebRtcPoll, WebRtcError> {
        let mut poll = WebRtcPoll::default();
        loop {
            match self.rtc.poll_output().map_err(WebRtcError::Rtc)? {
                Output::Timeout(timeout) => {
                    poll.timeout = Some(timeout);
                    break;
                }
                Output::Transmit(transmit) => poll.datagrams.push(Datagram {
                    source: transmit.source,
                    destination: transmit.destination,
                    contents: transmit.contents.to_vec(),
                }),
                Output::Event(event) => match event {
                    Event::Connected => {
                        self.connected = true;
                        poll.events.push(WebRtcEvent::Connected);
                    }
                    Event::IceConnectionStateChange(state) => {
                        if format!("{state:?}").contains("Failed") {
                            poll.events.push(WebRtcEvent::IceFailed);
                        }
                    }
                    Event::ChannelOpen(id, label) => {
                        self.channel = Some(id);
                        poll.events.push(WebRtcEvent::ChannelOpen(label));
                    }
                    Event::ChannelData(data) => poll.events.push(WebRtcEvent::Input(data.data)),
                    Event::KeyframeRequest(_) => poll.events.push(WebRtcEvent::KeyframeRequest),
                    Event::Closed => {
                        self.connected = false;
                        poll.events.push(WebRtcEvent::Closed);
                    }
                    _ => {}
                },
            }
        }
        Ok(poll)
    }

    pub fn write_video(
        &mut self,
        now: Instant,
        pts_us: u64,
        data: &[u8],
    ) -> Result<bool, WebRtcError> {
        let mid = self.video_mid.ok_or(WebRtcError::NoVideo)?;
        let params = self
            .rtc
            .codec_config()
            .find(|params| params.spec().codec == str0m::format::Codec::H264)
            .cloned()
            .ok_or(WebRtcError::NoVideo)?;
        let writer = self.rtc.writer(mid).ok_or(WebRtcError::NoVideo)?;
        writer
            .write(
                params.pt(),
                now,
                MediaTime::from_micros(pts_us),
                data.to_vec(),
            )
            .map_err(WebRtcError::Rtc)?;
        Ok(true)
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<bool, WebRtcError> {
        let channel = self.channel.ok_or(WebRtcError::NoChannel)?;
        let mut handle = self.rtc.channel(channel).ok_or(WebRtcError::NoChannel)?;
        handle.write(true, data).map_err(WebRtcError::Rtc)
    }
}

#[derive(Debug)]
pub enum WebRtcError {
    Rtc(RtcError),
    Json(serde_json::Error),
    Datagram,
    NoPendingOffer,
    NoVideo,
    NoChannel,
}

impl std::fmt::Display for WebRtcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rtc(error) => write!(f, "str0m: {error}"),
            Self::Json(error) => write!(f, "signaling JSON: {error}"),
            Self::Datagram => write!(f, "malformed datagram"),
            Self::NoPendingOffer => write!(f, "no pending SDP offer"),
            Self::NoVideo => write!(f, "video m-line is not negotiated"),
            Self::NoChannel => write!(f, "input channel is not open"),
        }
    }
}

impl std::error::Error for WebRtcError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{Codec, EncoderKind};
    use std::net::Ipv4Addr;

    fn frame(sequence: u64) -> EncodedFrame {
        EncodedFrame {
            sequence,
            pts_us: sequence * 16_667,
            keyframe: sequence == 1,
            kind: EncoderKind::Software,
            codec: Codec::H264,
            width: 320,
            height: 180,
            encode_us: 100,
            data: vec![0, 0, 0, 1, sequence as u8],
        }
    }

    #[test]
    fn loopback_drops_the_stale_frame_instead_of_queueing() {
        let mut transport = LoopbackTransport::new();
        transport.send_frame(&frame(1), 0).unwrap();
        transport.send_frame(&frame(2), 1_000).unwrap();
        assert_eq!(transport.stats().frames_sent, 2);
        assert_eq!(transport.stats().frames_dropped, 1);
        assert_eq!(transport.stats().queued, 1);
        transport.deliver(2_000);
        let delivered = transport.drain();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].sequence, 2);
    }

    struct Pair {
        host: WebRtcHost,
        client: WebRtcHost,
        now: Instant,
        host_queue: Vec<Datagram>,
        client_queue: Vec<Datagram>,
    }

    impl Pair {
        fn new() -> Self {
            let now = Instant::now();
            let mut host = WebRtcHost::new(now, WebRtcConfig::default()).unwrap();
            let mut client = WebRtcHost::new(now, WebRtcConfig::default()).unwrap();
            host.add_host_candidate(SocketAddr::from((Ipv4Addr::LOCALHOST, 41000)))
                .unwrap();
            client
                .add_host_candidate(SocketAddr::from((Ipv4Addr::LOCALHOST, 42000)))
                .unwrap();
            let offer = host.create_offer().unwrap();
            let answer = client.accept_offer(&offer).unwrap();
            host.accept_answer(&answer).unwrap();
            Self {
                host,
                client,
                now,
                host_queue: Vec::new(),
                client_queue: Vec::new(),
            }
        }

        fn step(&mut self) -> Vec<WebRtcEvent> {
            self.now += std::time::Duration::from_millis(5);
            let mut events = Vec::new();
            if let Ok(poll) = self.host.poll(self.now) {
                self.host_queue.extend(poll.datagrams);
                events.extend(poll.events);
            }
            if let Ok(poll) = self.client.poll(self.now) {
                self.client_queue.extend(poll.datagrams);
                events.extend(poll.events);
            }
            for datagram in self.host_queue.drain(..) {
                if let Ok(poll) = self.client.handle_datagram(
                    self.now,
                    datagram.source,
                    datagram.destination,
                    &datagram.contents,
                ) {
                    self.client_queue.extend(poll.datagrams);
                    events.extend(poll.events);
                }
            }
            for datagram in self.client_queue.drain(..) {
                if let Ok(poll) = self.host.handle_datagram(
                    self.now,
                    datagram.source,
                    datagram.destination,
                    &datagram.contents,
                ) {
                    self.host_queue.extend(poll.datagrams);
                    events.extend(poll.events);
                }
            }
            events
        }

        fn connect(&mut self) {
            for _ in 0..500 {
                self.step();
                if self.host.is_connected() && self.client.is_connected() {
                    return;
                }
            }
            panic!("str0m peers did not connect");
        }
    }

    #[test]
    fn str0m_peers_connect_and_exchange_input() {
        let mut pair = Pair::new();
        pair.connect();
        assert!(pair.host.fingerprint().starts_with("sha-256"));
        let mut saw_channel = false;
        for _ in 0..40 {
            let events = pair.step();
            if events
                .iter()
                .any(|event| matches!(event, WebRtcEvent::ChannelOpen(_)))
            {
                saw_channel = true;
            }
        }
        assert!(saw_channel, "data channel never opened");
        pair.host.write_input(b"hello").unwrap();
        let mut received = Vec::new();
        for _ in 0..40 {
            for event in pair.step() {
                if let WebRtcEvent::Input(data) = event {
                    received.push(data);
                }
            }
        }
        assert_eq!(received, vec![b"hello".to_vec()]);
    }

    #[test]
    fn offer_carries_the_host_candidate_port() {
        let mut host = WebRtcHost::new(Instant::now(), WebRtcConfig::default()).unwrap();
        host.add_host_candidate("127.0.0.1:54321".parse().unwrap())
            .unwrap();
        let offer: serde_json::Value = serde_json::from_str(&host.create_offer().unwrap()).unwrap();
        let sdp = offer["sdp"].as_str().expect("sdp string");
        assert!(
            sdp.contains(" 54321 typ host"),
            "offer does not advertise the bound port:\n{sdp}"
        );
    }

    #[test]
    fn writes_video_after_negotiation() {
        let mut pair = Pair::new();
        pair.connect();
        for _ in 0..10 {
            pair.step();
        }
        assert!(pair
            .host
            .write_video(pair.now, 16_667, &[0, 0, 0, 1, 9])
            .unwrap());
    }
}
