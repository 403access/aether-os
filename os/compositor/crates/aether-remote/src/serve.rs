//! Minimal LAN signaling service and synthetic host loop.
//!
//! The service speaks the same JSON messages as [`crate::signaling`], creates a
//! str0m offer with one video m-line and one input channel, and publishes a
//! status document the compositor reads for the remote overlay.
//!
//! The media source is synthetic until the compositor binds its exported
//! DMA-BUFs. The transport, signaling, pairing and status path are real.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::capture::{CaptureSource, SyntheticSource};
use crate::encode::{Codec, EncoderConfig, EncoderKind, H264Encoder};
use crate::ladder::Tier;
use crate::session::{
    PairingRegistry, RemoteSession, SessionPolicy, SessionState, DEFAULT_PAIRING_TTL_MS,
};
use crate::signaling::{accept_hello, SignalMessage};
use crate::transport::{WebRtcConfig, WebRtcHost, WebRtcPoll};
use crate::wire::WireFrame;

#[derive(Clone, Debug)]
pub struct ServeConfig {
    pub http_addr: SocketAddr,
    pub udp_addr: SocketAddr,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub status_path: Option<PathBuf>,
    pub code: Option<String>,
    pub run_for: Option<Duration>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            http_addr: SocketAddr::from(([127, 0, 0, 1], 7443)),
            udp_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            width: 640,
            height: 360,
            fps: 30,
            bitrate_kbps: 4_000,
            status_path: None,
            code: None,
            run_for: None,
        }
    }
}

struct HostState {
    session: Option<RemoteSession>,
    rtc: Option<WebRtcHost>,
    offer: Option<String>,
    input_channel: bool,
    frames: u64,
    started: Instant,
    udp_addr: SocketAddr,
}

pub fn serve(config: ServeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(config.http_addr)?;
    listener.set_nonblocking(true)?;
    let http_addr = listener.local_addr()?;
    let socket = UdpSocket::bind(config.udp_addr)?;
    socket.set_nonblocking(true)?;
    let udp_addr = socket.local_addr()?;

    let mut registry = PairingRegistry::new(DEFAULT_PAIRING_TTL_MS);
    let now_ms = unix_ms();
    let pairing = match config.code.clone() {
        Some(code) => {
            // Deterministic code for the smoke test. It is redeemed like any
            // other code; the alphabet check still applies.
            let mut registry_code = registry.issue(now_ms);
            registry_code.code = code;
            registry_code
        }
        None => registry.issue(now_ms),
    };
    // Keep the registry in sync with a supplied code.
    if config.code.is_some() {
        registry = PairingRegistry::new(DEFAULT_PAIRING_TTL_MS);
        let mut code = pairing.clone();
        code.used = false;
        // `issue` is the only mutation path; seed a matching entry through a
        // tiny deterministic registry shim.
        registry.seed(code);
    }

    let mut encoder = {
        let mut encoder_config =
            EncoderConfig::low_latency(config.width, config.height, config.fps)
                .with_kind(EncoderKind::Software)
                .with_bitrate(config.bitrate_kbps);
        encoder_config.keyframe_interval = config.fps * 2;
        H264Encoder::new(encoder_config)?
    };
    let mut source = SyntheticSource::new(config.width, config.height, config.fps);
    let mut state = HostState {
        session: None,
        rtc: None,
        offer: None,
        input_channel: false,
        frames: 0,
        started: Instant::now(),
        udp_addr,
    };
    let mut last_frame = Instant::now();
    let frame_interval = Duration::from_micros(1_000_000 / u64::from(config.fps.max(1)));

    println!(
        "aether-remote: signaling http://{http_addr}/signal  udp {udp_addr}  code {}",
        pairing.display()
    );
    write_status(&config, &pairing, &state)?;
    let mut last_status = Instant::now();

    loop {
        if let Some(run_for) = config.run_for {
            if state.started.elapsed() >= run_for {
                break;
            }
        }
        // HTTP signaling.
        match listener.accept() {
            Ok((stream, _peer)) => {
                if let Err(error) =
                    handle_http(stream, &mut registry, &mut state, &config, &pairing)
                {
                    eprintln!("aether-remote: signaling error: {error}");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }

        // UDP media.
        let mut buf = [0u8; 2048];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((length, source)) => {
                    if let Some(rtc) = state.rtc.as_mut() {
                        let now = Instant::now();
                        match rtc.handle_datagram(now, source, udp_addr, &buf[..length]) {
                            Ok(poll) => {
                                for datagram in poll.datagrams {
                                    let _ =
                                        socket.send_to(&datagram.contents, datagram.destination);
                                }
                                for event in poll.events {
                                    match event {
                                        crate::transport::WebRtcEvent::Input(data) => {
                                            handle_input(&mut state, &data);
                                        }
                                        crate::transport::WebRtcEvent::ChannelOpen(_) => {
                                            state.input_channel = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error.into()),
            }
        }

        // Poll timers and send queued RTC datagrams.
        if state.rtc.is_some() {
            let (poll, connected) = {
                let rtc = state.rtc.as_mut().expect("checked above");
                (rtc.poll(Instant::now())?, rtc.is_connected())
            };
            drain_poll(&socket, poll, &mut state);
            if connected {
                if let Some(session) = state.session.as_mut() {
                    session.activate(unix_ms());
                }
            }
        }

        // Synthetic capture → H.264 → RTP.
        if last_frame.elapsed() >= frame_interval {
            last_frame = Instant::now();
            let now_us = state.started.elapsed().as_micros() as u64;
            if let Some(frame) = source.next_frame(now_us) {
                if let crate::capture::FramePayload::Cpu(cpu) = &frame.payload {
                    let encoded = encoder.encode(&crate::encode::BgraFrame::new(
                        frame.width,
                        frame.height,
                        cpu.stride,
                        &cpu.data,
                    ))?;
                    if let Some(rtc) = state.rtc.as_mut() {
                        let _ =
                            rtc.write_video(Instant::now(), frame.captured_at_us, &encoded.data);
                    }
                    state.frames += 1;
                }
            }
        }

        if last_status.elapsed() >= Duration::from_secs(1) {
            write_status(&config, &pairing, &state)?;
            last_status = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(2));
    }

    if let Some(session) = state.session.as_mut() {
        session.close(unix_ms(), "service stopped");
    }
    write_status(&config, &pairing, &state)?;
    Ok(())
}

fn drain_poll(socket: &UdpSocket, poll: WebRtcPoll, state: &mut HostState) {
    for datagram in poll.datagrams {
        let _ = socket.send_to(&datagram.contents, datagram.destination);
    }
    for event in poll.events {
        match event {
            crate::transport::WebRtcEvent::Connected => {
                if let Some(session) = state.session.as_mut() {
                    session.activate(unix_ms());
                }
            }
            crate::transport::WebRtcEvent::ChannelOpen(_) => state.input_channel = true,
            crate::transport::WebRtcEvent::Input(data) => handle_input(state, &data),
            _ => {}
        }
    }
}

fn handle_input(state: &mut HostState, data: &[u8]) {
    // Accept either the compact wire frame or raw JSON input events.
    if let Ok(frame) = WireFrame::decode(data) {
        println!(
            "aether-remote: input frame tier={:?} bytes={}",
            frame.tier,
            frame.payload.len()
        );
        return;
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(kind) = value.get("kind").and_then(|value| value.as_str()) {
                println!("aether-remote: input {kind}");
            }
        }
    }
    if let Some(session) = state.session.as_mut() {
        let _ = session.authorize_input(unix_ms(), &session.token.0.clone());
    }
}

fn handle_http(
    mut stream: TcpStream,
    registry: &mut PairingRegistry,
    state: &mut HostState,
    config: &ServeConfig,
    pairing: &crate::session::PairingCode,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = find_header_end(&buffer) {
            let header = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let content_length = header
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(0);
            if buffer.len() >= header_end + 4 + content_length {
                let body = &buffer[header_end + 4..header_end + 4 + content_length];
                return respond(&mut stream, &header, body, registry, state, config, pairing);
            }
        }
        if buffer.len() > 256 * 1024 {
            return Ok(());
        }
    }
    Ok(())
}

fn respond(
    stream: &mut TcpStream,
    header: &str,
    body: &[u8],
    registry: &mut PairingRegistry,
    state: &mut HostState,
    config: &ServeConfig,
    pairing: &crate::session::PairingCode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lines = header.lines();
    let request = lines.next().unwrap_or_default();
    let method = request.split_whitespace().next().unwrap_or_default();
    let path = request.split_whitespace().nth(1).unwrap_or_default();
    if method == "OPTIONS" {
        return write_http(stream, 204, "");
    }
    if method == "GET" && path == "/health" {
        let body = json!({
            "ok": true,
            "code": pairing.display(),
            "session": state.session.as_ref().map(|session| session.id.clone()),
            "frames": state.frames,
        });
        return write_http(stream, 200, &serde_json::to_string(&body)?);
    }
    if method != "POST" || path != "/signal" {
        return write_http(stream, 404, "{\"error\":\"not found\"}");
    }
    let text = String::from_utf8_lossy(body);
    let message = match SignalMessage::from_json(&text) {
        Ok(message) => message,
        Err(error) => {
            return write_http(
                stream,
                400,
                &serde_json::to_string(&json!({ "error": error.to_string() }))?,
            );
        }
    };
    match message {
        SignalMessage::Hello(hello) => {
            let now_ms = unix_ms();
            let mut rtc = WebRtcHost::new(Instant::now(), WebRtcConfig::default())?;
            rtc.add_host_candidate(config.udp_addr)?;
            let fingerprint = rtc.fingerprint().to_string();
            let (session, welcome) = match accept_hello(
                registry,
                now_ms,
                &hello,
                &[Codec::H264],
                &fingerprint,
                SessionPolicy {
                    max_bitrate_kbps: config.bitrate_kbps,
                    ..SessionPolicy::default()
                },
            ) {
                Ok(value) => value,
                Err(error) => {
                    return write_http(
                        stream,
                        403,
                        &serde_json::to_string(&json!({ "error": error.to_string() }))?,
                    );
                }
            };
            let offer = rtc.create_offer()?;
            state.offer = Some(offer.clone());
            state.session = Some(session);
            state.rtc = Some(rtc);
            write_status(config, pairing, state)?;
            let response = json!({
                "welcome": welcome,
                "offer": offer,
            });
            write_http(stream, 200, &serde_json::to_string(&response)?)?;
            println!(
                "aether-remote: viewer paired ({} {})",
                hello.client_name,
                hello.caps.codecs().len()
            );
        }
        SignalMessage::Answer { sdp } => {
            if let Some(rtc) = state.rtc.as_mut() {
                rtc.accept_answer(&sdp)?;
                write_http(stream, 200, "{\"ok\":true}")?;
                println!("aether-remote: answer accepted");
            } else {
                write_http(stream, 409, "{\"error\":\"no pending offer\"}")?;
            }
        }
        SignalMessage::Bye { reason } => {
            if let Some(session) = state.session.as_mut() {
                session.close(unix_ms(), &reason);
            }
            state.rtc = None;
            write_http(stream, 200, "{\"ok\":true}")?;
        }
        other => {
            let response = json!({ "ignored": other.to_json() });
            write_http(stream, 200, &serde_json::to_string(&response)?)?;
        }
    }
    Ok(())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_http(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn write_status(
    config: &ServeConfig,
    pairing: &crate::session::PairingCode,
    state: &HostState,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = config.status_path.as_ref() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let (session_id, state_label, tier, codec, latency, viewers): (
        String,
        SessionState,
        Tier,
        Codec,
        Option<f32>,
        u8,
    ) = match state.session.as_ref() {
        Some(session) => (
            session.id.clone(),
            session.state,
            session.tier,
            session.codec,
            None,
            u8::from(session.state == SessionState::Active),
        ),
        None => (
            "—".to_string(),
            SessionState::Waiting,
            Tier::WebRtc,
            Codec::H264,
            None,
            0,
        ),
    };
    let document = json!({
        "session_id": session_id,
        "pairing_code": pairing.display(),
        "tier": tier.short(),
        "codec": codec.label(),
        "state": match state_label {
            SessionState::Waiting => "waiting",
            SessionState::Active => "streaming",
            SessionState::Closed => "closed",
        },
        "latency_ms": latency,
        "bitrate_kbps": config.bitrate_kbps,
        "viewers": viewers,
        "input": true,
        "clipboard": false,
        "software": true,
        "frames": state.frames,
        "udp": state.udp_addr.to_string(),
    });
    std::fs::write(path, serde_json::to_string_pretty(&document)? + "\n")?;
    Ok(())
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_header_boundary() {
        let request = b"POST /signal HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}";
        let end = find_header_end(request).unwrap();
        assert_eq!(&request[end..end + 4], b"\r\n\r\n");
        assert_eq!(&request[end + 4..], b"{}");
    }

    #[test]
    fn default_service_is_local_only() {
        let config = ServeConfig::default();
        assert!(config.http_addr.ip().is_loopback());
        assert_eq!(config.fps, 30);
        assert_eq!(config.width, 640);
    }
}
