---
id: remote-display
title: Remote display
sidebar_position: 14
description: Zero-copy capture, hardware encode, WebRTC transport and WebCodecs rendering — with a fallback ladder.
---

# Remote display

Access the desktop from a browser without VNC’s latency tax. The intended pipeline
never copies a frame through the CPU.

![The remote-session overlay in the Aether shell](/img/smithay-remote.png)

The P7 code path exists today: a DMA-BUF descriptor + copy audit, a real low-latency
H.264 software encoder, str0m WebRTC transport, a virtual-seat input channel, a
signaling/pairing service, the browser client and a WebSocket/JPEG fallback frame
format. The compositor still supplies a synthetic source to the harness; binding
the compositor’s exported DMA-BUFs and measuring glass-to-glass on the 7700K are
the remaining P7 work. See [P7 completion](/roadmap/p7-completion).

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

### What is in the repository

| Piece | Code | State |
| --- | --- | --- |
| Capture descriptor, DMA-BUF planes, damage | `os/compositor/crates/aether-remote/src/capture.rs` | ✅ validated + copy audit |
| Low-latency H.264 (Annex-B, no B-frames, no lookahead) | `encode.rs` | ✅ software path; VA-API/NVENC selection |
| Pairing, tokens, DTLS fingerprint, audit chain | `session.rs`, `signaling.rs` | ✅ |
| str0m WebRTC + input data channel | `transport.rs` | ✅ loopback-tested |
| Virtual seat: scancodes, coalesced motion, rate limit | `input.rs` | ✅ |
| No-queue pipeline + latency ledger | `pipeline.rs`, `stats.rs` | ✅ |
| Browser client, WebCodecs and JPEG sinks, tier ladder | `packages/remote` | ✅ unit-tested |
| Signaling + status service | `aether-remote serve` | ✅ local-only, opt-in systemd unit |
| Compositor overlay + recording indicator | `aether-shell`, `aether-shell-wayland` | ✅ `Super+R` |

The host reads the service’s status document from
`$XDG_RUNTIME_DIR/aether/remote-session.json` and renders the pairing code,
latency, bitrate, viewer count and the audited input/clipboard toggles.

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
- **Fallback:** if UDP is blocked, WebSocket + access units + WebCodecs, or JPEG
  tiles when WebCodecs is unavailable. Higher latency, still no CPU pixel copies.

### Fallback wire format

Tier 2 and tier 3 share a 24-byte binary header (`AEM1`): version, tier, codec,
flags, sequence, presentation timestamp and payload length. WebCodecs consumes the
payload as an access unit; tier 3 decodes each payload as a complete JPEG tile. The
Rust encoder and the TypeScript client carry the same constants and have round-trip
tests, so a browser can parse a packet without a demuxer.

## Browser client

![The browser client with the live session HUD](/img/remote-viewer.png)

- Plain TypeScript, no framework required (`@aether/remote`).
- `VideoDecoder` feeds a `VideoFrame` directly into the canvas; JPEG tiles use the
  platform image decoder.
- Pointer Lock and `Keyboard.lock()` capture raw input; keys map to Linux evdev
  scancodes, never to commands.
- Input is rate-limited and coalesced (mouse motion) to avoid flooding the host.
- The client shows latency, bitrate, codec and tier; no hidden quality degradation.
- The session state machine survives a downgrade (`fallback()`) and a transport
  disconnect without restarting the session.

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
   overlay capture. **Budget** until the target run.
2. p99 frame interval < 1.5× target at 60 fps. Dev-host synthetic harness passes;
   target run pending.
3. No CPU pixel copies in the encode path (verified with `perf` and DMA-BUF traces).
   The audit is implemented; compositor import is target-only.
4. Input round trip < 20 ms. The data channel is loopback-tested; target timing is
   pending.
5. Failed client decode falls back without restarting the session. The client state
   machine covers the transition and has unit tests.

Runbook and evidence: [P7 completion](/roadmap/p7-completion). The service is
local-only by default: `aether-remote serve` binds `127.0.0.1:7443` for signaling
and an ephemeral loopback UDP port for media. `systemctl --user start aether-remote`
starts it; no firewall port is opened. `scripts/remote-smoke.sh` exercises
`/health`, pairing, the welcome/offer exchange and the published status document.
