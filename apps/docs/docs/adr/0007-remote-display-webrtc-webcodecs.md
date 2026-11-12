---
id: 0007-remote-display-webrtc-webcodecs
title: ADR-0007 — WebRTC + WebCodecs remote display
sidebar_position: 8
---

# ADR-0007 — WebRTC + WebCodecs remote display

**Status:** Proposed (P7) · **Date:** 2026-11-12

## Context

The desktop should be accessible from a browser with interactive latency. VNC and
MJPEG add frame buffering and CPU copies; HTTP video adds seconds of buffering.

## Decision

Capture DMA-BUFs from the compositor, encode with VA-API/NVENC using low-latency
settings, transport over WebRTC (str0m), render with WebCodecs into a WebGL canvas,
and send input over a data channel. Ship a fallback ladder for blocked UDP or missing
WebCodecs.

## Consequences

- Sub-40 ms LAN latency is plausible, but unproven until P7.
- Encoder/driver matrix becomes a support surface.
- WebCodecs support varies by browser; H.264 is the compatibility floor.
- Security needs DTLS, a session token and a pairing code; no public listener.
- AF_XDP is deferred until measurement shows the kernel stack dominates.

## Alternatives

- **VNC** — simple, high latency, CPU-bound. Rejected as the primary path.
- **Sunshine/Moonlight** — proven, but an external dependency with its own UX and
  protocol assumptions. A viable fallback, not the architecture.
- **WebSocket + fMP4** — good fallback; more latency than WebRTC.
- **Custom UDP protocol** — reinvents congestion control and loss recovery.
