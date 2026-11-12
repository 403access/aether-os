---
id: remote-display
title: Remote display
sidebar_position: 14
description: Zero-copy capture, hardware encode, WebRTC transport and WebCodecs rendering — with a fallback ladder.
---

# Remote display

Access the desktop from a browser without VNC’s latency tax. The intended pipeline
never copies a frame through the CPU.

## Pipeline

```text
compositor (DMA-BUF)
   → VA-API / NVENC (zero-copy import)
   → H.264 / HEVC / AV1 (low-latency settings)
   → str0m (WebRTC SRTP)
   → browser
   → WebCodecs VideoDecoder
   → WebGL/WebGPU canvas
```

Input returns over a WebRTC data channel, is validated as scancodes/relative motion,
and enters the compositor’s virtual seat.

## Latency budget

| Stage | Budget |
| --- | --- |
| Capture + import | 2 ms |
| Encode | 4–8 ms |
| Network (LAN) | 1–3 ms |
| Jitter buffer | 0–5 ms |
| Decode | 2–5 ms |
| Canvas present | 1–2 ms |
| **Glass-to-glass** | **< 40 ms** |

At 60 fps the frame interval is 16.7 ms; the target is 2–3 frames end to end. At
120 fps the budget tightens and the encoder must not queue.

## Encoder settings

| Setting | Value | Why |
| --- | --- | --- |
| Rate control | CBR (or low-latency VBR) | Predictable pacing |
| B-frames | 0 | No reordering delay |
| GOP | Intra-refresh, no IDR | Eliminates keyframe spikes |
| Lookahead | 0 | Latency |
| Slice mode | One slice per frame | Parallel encode without frame delay |
| Profile | H.264 high (compat), HEVC/AV1 (preferred) | Client capability negotiation |
| Bitrate | 8–25 Mbps LAN, adaptive | Quality vs. congestion |

Adaptive bitrate uses WebRTC’s congestion control; the encoder is re-configured, not
restarted.

## Transport

- **WebRTC** over UDP (str0m or webrtc-rs). Built-in RTP, congestion control, packet
  loss concealment and data channels.
- **Signaling** is a minimal local service: one offer/answer exchange, no TURN by
  default (LAN/VPN only), explicit ICE candidates.
- **Security:** DTLS-SRTP, a per-session bearer token, and a one-shot pairing code.
  No inbound port is opened on the public internet.
- **Fallback:** if UDP is blocked, WebSocket + fragmented MP4 + WebCodecs. Higher
  latency, still no CPU pixel copies.

## Browser client

- Plain TypeScript, no framework required.
- `VideoDecoder` feeds a `VideoFrame` directly into a WebGL texture.
- Pointer Lock and `Keyboard.lock()` capture raw input.
- Input is rate-limited and coalesced (mouse motion) to avoid flooding the host.
- The client shows latency, bitrate and codec; no hidden quality degradation.

## Fallback ladder

| Tier | Transport | Latency | When |
| --- | --- | --- | --- |
| 1 | WebRTC + WebCodecs | < 40 ms | Default on LAN |
| 2 | WebSocket + fMP4 + WebCodecs | 60–120 ms | UDP blocked |
| 3 | WebSocket + JPEG tiles | 120–250 ms | No WebCodecs |
| 4 | VNC | 200 ms+ | Last resort; explicitly labelled |

Aether ships tier 1. Tiers 2–3 exist for degraded networks; tier 4 is a debugging
tool, not the product.

## AF_XDP: conditional, not default

Kernel-bypass networking is only worth the complexity if measurement shows the
kernel stack is the bottleneck. The plan:

1. Measure tier 1 with `perf` and socket traces.
2. If packet processing is < 10% of frame latency, do nothing.
3. If it dominates, prototype AF_XDP for the SRTP path and compare p99.

The architecture document promises a measurement, not a technology. See
[performance engineering](/architecture/performance-engineering).

## Security

| Concern | Mitigation |
| --- | --- |
| Unauthorized viewer | Per-session token + pairing code + DTLS |
| Input injection | Scancodes only; no shell command channel |
| Clipboard | Off by default; explicit per-session toggle |
| Screen content | The user starts the session; a recording indicator is always visible |
| Network exposure | No public listener; VPN/LAN or an authenticated relay |
| Agent access | Agents cannot start a remote session; the capability is host-only |

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Encoder unavailable | Fall back to software encode with a latency badge |
| Client decode fails | Negotiate H.264; if still failing, tier 2 |
| Congestion | Lower bitrate/framerate; never queue frames |
| Packet loss | Intra-refresh repairs without a keyframe stall |
| Client disconnects | Session ends; compositor returns to local output |
| Host GPU reset | Session drops; client shows reconnect |

## Verification (P7)

1. Glass-to-glass < 40 ms on LAN, measured with a photodiode or a timestamped
   overlay capture.
2. p99 frame interval < 1.5× target at 60 fps.
3. No CPU pixel copies in the encode path (verified with `perf` and DMA-BUF traces).
4. Input round trip < 20 ms.
5. Failed client decode falls back without restarting the session.
