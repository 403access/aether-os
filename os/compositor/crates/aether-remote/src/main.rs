//! `aether-remote` host utility.
//!
//! Subcommands are added per P7 task. `capture` is the first: it exercises the
//! descriptor and copy-audit path without a compositor.

use std::path::PathBuf;

use aether_remote::capture::{
    CaptureFrame, CaptureSource, CopyAuditor, DmaBufFrame, FramePayload, PixelFormat,
    SyntheticSource,
};
use aether_remote::encode::{
    default_preferences, select_encoder, Codec, EncoderCaps, EncoderConfig, EncoderKind,
    H264Encoder,
};
use aether_remote::ladder::ClientCaps;
use aether_remote::pipeline::{Pipeline, PipelineConfig};
use aether_remote::session::{PairingRegistry, SessionPolicy, DEFAULT_PAIRING_TTL_MS};
use aether_remote::signaling::{accept_hello, ClientHello, SignalMessage};
use aether_remote::transport::LoopbackTransport;
use aether_remote::transport::{Datagram, WebRtcConfig, WebRtcEvent, WebRtcHost};
use serde_json::json;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("capture") | None => {
            if let Err(error) = capture(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("encode") => {
            if let Err(error) = encode(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("pair") => {
            if let Err(error) = pair(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("webrtc") => {
            if let Err(error) = webrtc(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("bench") => {
            if let Err(error) = bench(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("--help" | "-h" | "help") => usage(),
        Some(other) => {
            eprintln!("aether-remote: unknown command {other}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    println!(
        "aether-remote <command>\n\
         \n\
         commands:\n\
         \x20 capture [--width N] [--height N] [--frames N] [--json PATH]  exercise the capture model\n\
         \x20 encode [--width N] [--height N] [--frames N] [--ndjson PATH]  encode synthetic frames\n\
         \x20 pair [--json PATH]                                           mint a one-shot pairing code and welcome\n\
         \x20 webrtc [--json PATH]                                         negotiate and drive a loopback str0m session\n\
         \x20 bench [--width N] [--height N] [--frames N] [--ndjson PATH]    capture→encode→transport with latency ledger\n\
         \x20 help                                                           this message"
    );
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn number<T: std::str::FromStr>(args: &[String], name: &str, fallback: T) -> T {
    flag(args, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn capture(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let width: u32 = number(args, "--width", 640);
    let height: u32 = number(args, "--height", 360);
    let frames: usize = number(args, "--frames", 4);
    let fps: u32 = number(args, "--fps", 60);
    let mut source = SyntheticSource::new(width, height, fps);
    let mut auditor = CopyAuditor::default();
    let mut captured = Vec::new();
    let mut now_us = 0u64;
    let interval = 1_000_000 / u64::from(fps.max(1));
    while captured.len() < frames {
        if let Some(frame) = source.next_frame(now_us) {
            auditor.record(&frame);
            captured.push(frame);
        }
        now_us += interval / 4;
    }
    let dma = DmaBufFrame::new(width, height, PixelFormat::Bgra, width * 4);
    let dma_frame = CaptureFrame {
        sequence: 1,
        captured_at_us: 0,
        output: 1,
        width,
        height,
        damage: vec![aether_remote::capture::DamageRect::new(0, 0, width, height)],
        payload: FramePayload::DmaBuf(dma.clone()),
    };
    let mut dma_auditor = CopyAuditor::default();
    dma_auditor.record(&dma_frame);
    let summary = json!({
        "source": "synthetic",
        "width": width,
        "height": height,
        "fps": fps,
        "software_auditor": auditor,
        "dma_buf": {
            "descriptor": dma,
            "valid": dma.validate().is_ok(),
            "auditor": dma_auditor,
        },
        "frames": captured.iter().map(|frame| json!({
            "sequence": frame.sequence,
            "captured_at_us": frame.captured_at_us,
            "format": frame.format().label(),
            "zero_copy": frame.is_zero_copy(),
            "copy_bytes": frame.copy_bytes(),
            "damage": frame.damage,
            "damage_area": frame.damage_area(),
        })).collect::<Vec<_>>(),
        "note": "synthetic source; the compositor supplies DMA-BUF planes for the same descriptor"
    });
    let text = serde_json::to_string_pretty(&summary)?;
    if let Some(path) = flag(args, "--json") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{text}\n"))?;
        println!("wrote {}", path.display());
    } else {
        println!("{text}");
    }
    println!("capture: {}", auditor.summary());
    Ok(())
}

fn bench(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let width: u32 = number(args, "--width", 320);
    let height: u32 = number(args, "--height", 180);
    let frames: usize = number(args, "--frames", 120);
    let fps: u32 = number(args, "--fps", 60);
    let bitrate: u32 = number(args, "--bitrate", 4_000);
    let config = PipelineConfig {
        encoder: EncoderConfig::low_latency(width, height, fps)
            .with_kind(aether_remote::encode::EncoderKind::Software)
            .with_bitrate(bitrate),
        tier: aether_remote::ladder::Tier::WebRtc,
        target_fps: fps,
    };
    let mut pipeline = Pipeline::new(config, LoopbackTransport::new())?;
    let mut source = SyntheticSource::new(width, height, fps);
    let interval = Duration::from_micros(1_000_000 / u64::from(fps.max(1)));
    let start = Instant::now();
    let mut records = Vec::with_capacity(frames);
    let mut previous_us = 0u64;
    for index in 0..frames {
        let target = interval * index as u32;
        let elapsed = start.elapsed();
        if target > elapsed {
            std::thread::sleep(target - elapsed);
        }
        let now_us = start.elapsed().as_micros() as u64;
        let frame = loop {
            match source.next_frame(now_us) {
                Some(frame) => break frame,
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        };
        let event = pipeline.push_capture(&frame, now_us)?;
        let sent = pipeline.tick(now_us);
        // Stand in for a client that keeps up with the stream.
        pipeline.transport_mut().deliver(now_us + 1_000);
        let wall_us = now_us.saturating_sub(previous_us);
        previous_us = now_us;
        if let aether_remote::pipeline::PipelineEvent::Encoded {
            sequence,
            bytes,
            keyframe,
            encode_us,
        } = event
        {
            records.push(json!({
                "sequence": sequence,
                "sent": sent.is_some(),
                "bytes": bytes,
                "keyframe": keyframe,
                "encode_us": encode_us,
                "wall_interval_us": wall_us,
                "captured_at_us": frame.captured_at_us,
            }));
        }
    }
    let report = pipeline.report();
    let summary = json!({
        "report": report,
        "frame_interval_target_us": 1_000_000 / u64::from(fps.max(1)),
        "note": "synthetic source + software H.264; the glass-to-glass gate needs the target capture path",
    });
    let ndjson = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    let ndjson = format!(
        "{ndjson}\n{{\"summary\":{}}}\n",
        serde_json::to_string(&summary)?
    );
    if let Some(path) = flag(args, "--ndjson") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, ndjson)?;
        println!("wrote {}", path.display());
    } else {
        print!("{ndjson}");
    }
    println!("bench: {}", report.summary_line());
    if !report.interval_budget_ok {
        return Err(format!(
            "frame interval p99 {} us exceeds the 1.5x budget ({} us)",
            report.interval_us_p99.unwrap_or(0),
            report.target_interval_us * 3 / 2
        )
        .into());
    }
    Ok(())
}

fn webrtc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut host = WebRtcHost::new(start, WebRtcConfig::default())?;
    let mut client = WebRtcHost::new(start, WebRtcConfig::default())?;
    host.add_host_candidate(SocketAddr::from((Ipv4Addr::LOCALHOST, 43000)))?;
    client.add_host_candidate(SocketAddr::from((Ipv4Addr::LOCALHOST, 44000)))?;
    let offer = host.create_offer()?;
    let answer = client.accept_offer(&offer)?;
    host.accept_answer(&answer)?;

    let mut now = start;
    let mut host_queue: Vec<Datagram> = Vec::new();
    let mut client_queue: Vec<Datagram> = Vec::new();
    let mut events = Vec::new();
    let mut datagrams = 0u64;
    let mut channel_open = false;
    let mut connected_at = None;
    for _ in 0..600 {
        now += Duration::from_millis(5);
        if let Ok(poll) = host.poll(now) {
            datagrams += poll.datagrams.len() as u64;
            host_queue.extend(poll.datagrams);
            events.extend(poll.events);
        }
        if let Ok(poll) = client.poll(now) {
            datagrams += poll.datagrams.len() as u64;
            client_queue.extend(poll.datagrams);
            events.extend(poll.events);
        }
        for datagram in host_queue.drain(..) {
            if let Ok(poll) = client.handle_datagram(
                now,
                datagram.source,
                datagram.destination,
                &datagram.contents,
            ) {
                datagrams += poll.datagrams.len() as u64;
                client_queue.extend(poll.datagrams);
                events.extend(poll.events);
            }
        }
        for datagram in client_queue.drain(..) {
            if let Ok(poll) = host.handle_datagram(
                now,
                datagram.source,
                datagram.destination,
                &datagram.contents,
            ) {
                datagrams += poll.datagrams.len() as u64;
                host_queue.extend(poll.datagrams);
                events.extend(poll.events);
            }
        }
        if host.is_connected() && connected_at.is_none() {
            connected_at = Some(start.elapsed().as_millis() as u64);
        }
        channel_open |= events
            .iter()
            .any(|event| matches!(event, WebRtcEvent::ChannelOpen(_)));
        if host.is_connected() && channel_open {
            break;
        }
    }
    let connected = host.is_connected() && client.is_connected();
    let mut received = Vec::new();
    let wrote_input = host.write_input(b"aether-input").unwrap_or(false);
    for _ in 0..40 {
        now += Duration::from_millis(5);
        if let Ok(poll) = host.poll(now) {
            host_queue.extend(poll.datagrams);
        }
        if let Ok(poll) = client.poll(now) {
            client_queue.extend(poll.datagrams);
        }
        for datagram in host_queue.drain(..) {
            if let Ok(poll) = client.handle_datagram(
                now,
                datagram.source,
                datagram.destination,
                &datagram.contents,
            ) {
                client_queue.extend(poll.datagrams);
                for event in poll.events {
                    if let WebRtcEvent::Input(data) = event {
                        received.push(String::from_utf8_lossy(&data).to_string());
                    }
                }
            }
        }
        for datagram in client_queue.drain(..) {
            if let Ok(poll) = host.handle_datagram(
                now,
                datagram.source,
                datagram.destination,
                &datagram.contents,
            ) {
                host_queue.extend(poll.datagrams);
            }
        }
    }
    let wrote_video = host
        .write_video(now, 16_667, &[0, 0, 0, 1, 9])
        .unwrap_or(false);
    let summary = json!({
        "connected": connected,
        "connected_after_ms": connected_at,
        "host_fingerprint": host.fingerprint(),
        "client_fingerprint": client.fingerprint(),
        "channel_open": channel_open,
        "input_written": wrote_input,
        "input_echo": received,
        "video_written": wrote_video,
        "datagrams": datagrams,
        "offer_bytes": offer.len(),
        "answer_bytes": answer.len(),
        "note": "loopback peers; no UDP socket or browser is involved",
    });
    let text = serde_json::to_string_pretty(&summary)?;
    if let Some(path) = flag(args, "--json") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{text}\n"))?;
        println!("wrote {}", path.display());
    } else {
        println!("{text}");
    }
    println!(
        "webrtc: connected={connected} channel={channel_open} input={:?} datagrams={datagrams}",
        received
    );
    if !connected || !channel_open || !wrote_video {
        return Err("str0m loopback did not complete".into());
    }
    Ok(())
}

fn pair(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    let mut registry = PairingRegistry::new(DEFAULT_PAIRING_TTL_MS);
    let code = registry.issue(now_ms);
    let hello = ClientHello::new(code.display(), ClientCaps::modern());
    let fingerprint = {
        let mut rtc = str0m::Rtc::new(std::time::Instant::now());
        rtc.direct_api().local_dtls_fingerprint().to_string()
    };
    let (session, welcome) = accept_hello(
        &mut registry,
        now_ms,
        &hello,
        &[Codec::H264],
        &fingerprint,
        SessionPolicy::default(),
    )?;
    let summary = json!({
        "pairing_code": code.display(),
        "expires_in_ms": code.expires_at_ms.saturating_sub(now_ms),
        "session": {
            "id": session.id,
            "tier": welcome.tier,
            "codec": welcome.codec,
            "expires_at_ms": welcome.expires_at_ms,
            "dtls_fingerprint": welcome.dtls_fingerprint,
            "policy": welcome.policy,
            "token_prefix": &welcome.token[..8],
        },
        "hello": SignalMessage::Hello(hello),
        "welcome": SignalMessage::Welcome(welcome),
        "audit": session.audit,
        "audit_valid": session.audit.verify(),
    });
    let text = serde_json::to_string_pretty(&summary)?;
    if let Some(path) = flag(args, "--json") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{text}\n"))?;
        println!("wrote {}", path.display());
    } else {
        println!("{text}");
    }
    println!(
        "pair: {}  tier={}  codec={}  session={}",
        code.display(),
        session.tier.short(),
        session.codec.label(),
        session.id
    );
    Ok(())
}

fn encode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let width: u32 = number(args, "--width", 320);
    let height: u32 = number(args, "--height", 180);
    let frames: usize = number(args, "--frames", 30);
    let fps: u32 = number(args, "--fps", 60);
    let bitrate: u32 = number(args, "--bitrate", 4_000);

    let caps = EncoderCaps::software_only();
    let (kind, codec) = select_encoder(
        &caps,
        &default_preferences(aether_remote::encode::Codec::H264),
    )
    .ok_or("no encoder available")?;
    let config = EncoderConfig::low_latency(width, height, fps)
        .with_kind(kind)
        .with_bitrate(bitrate);
    let mut encoder = H264Encoder::new(config)?;
    let mut source = SyntheticSource::new(width, height, fps);
    let mut auditor = CopyAuditor::default();
    let mut records = Vec::new();
    let mut now_us = 0u64;
    let interval = 1_000_000 / u64::from(fps.max(1));
    let mut encoded = 0usize;
    let mut total_bytes = 0u64;
    let mut max_encode_us = 0u64;
    let mut keyframes = 0usize;
    while encoded < frames {
        let Some(frame) = source.next_frame(now_us) else {
            now_us += interval / 4;
            continue;
        };
        auditor.record(&frame);
        let FramePayload::Cpu(cpu) = &frame.payload else {
            continue;
        };
        let encoded_frame = encoder.encode(&aether_remote::encode::BgraFrame::new(
            frame.width,
            frame.height,
            cpu.stride,
            &cpu.data,
        ))?;
        let bytes = encoded_frame.bytes() as u64;
        total_bytes += bytes;
        max_encode_us = max_encode_us.max(encoded_frame.encode_us);
        if encoded_frame.keyframe {
            keyframes += 1;
        }
        records.push(json!({
            "sequence": encoded_frame.sequence,
            "pts_us": frame.captured_at_us,
            "bytes": bytes,
            "keyframe": encoded_frame.keyframe,
            "annex_b": encoded_frame.is_annex_b(),
            "encode_us": encoded_frame.encode_us,
            "codec": encoded_frame.codec.label(),
            "kind": encoded_frame.kind.label(),
        }));
        encoded += 1;
        now_us += interval;
    }
    let summary = json!({
        "width": width,
        "height": height,
        "fps": fps,
        "frames": encoded,
        "keyframes": keyframes,
        "total_bytes": total_bytes,
        "max_encode_us": max_encode_us,
        "contract": encoder.config().low_latency_contract(),
        "capture": auditor,
        "software": kind == EncoderKind::Software,
    });
    let ndjson = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    let ndjson = format!(
        "{ndjson}\n{{\"summary\":{}}}\n",
        serde_json::to_string(&summary)?
    );
    if let Some(path) = flag(args, "--ndjson") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, ndjson)?;
        println!("wrote {}", path.display());
    } else {
        print!("{ndjson}");
    }
    println!(
        "encode: {kind:?}/{codec:?} {frames} frames, {total_bytes} bytes, max {max_encode_us} us"
    );
    Ok(())
}
