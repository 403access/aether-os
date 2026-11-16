---
id: p7-browser-viewer
title: Browser live viewer
sidebar_position: 7
description: Plan and ledger for the real browser viewer that pairs with `aether-remote serve`.
---

# Browser live viewer

The P7 remote display path has a real host (`aether-remote serve`), a real
signaling exchange, a real str0m WebRTC session and a real input channel, but no
page that drives them. The marketing site's `#remote` surface is a demo: it uses
the real `@aether/remote` state machine over a synthetic `DemoSocket` and a
canvas painter, so it cannot pair with a host.

This plan adds the missing piece: a live viewer page that a person can open in a
browser, point at a running host, enter the one-shot pairing code and see (and
control) the synthetic host stream.

## Goal

Open `http://127.0.0.1:4321/viewer.html`, enter `http://127.0.0.1:7443/signal`
and the code printed by `aether-remote serve`, click **Connect**, and get:

- a live H.264 video track rendered by the browser;
- a HUD with session id, tier, codec, resolution, fps, bitrate and RTT;
- the always-visible recording indicator;
- keyboard, pointer and wheel input over the `input` data channel;
- a clean stop that closes the peer connection and tells the host.

## Non-goals

- Binding compositor DMA-BUFs. The host still streams its synthetic source.
- Public or LAN listeners. The default remains loopback-only.
- Trickle ICE and TURN. The host accepts one answer SDP; no candidate messages.
- WebSocket/JPEG fallback in the live page. The demo keeps that model.
- A full virtual-seat implementation. Input is validated and audited, then
  logged; seat injection is the P7 compositor task.

## Architecture

```text
viewer.html (apps/web)
  └── Viewer.tsx
        ├── @aether/remote RemoteClient (state, session, policy)
        ├── @aether/remote HttpSignalTransport
        │     ├── POST /signal  hello  → { welcome, offer }
        │     ├── RTCPeerConnection      offer → answer
        │     └── POST /signal  answer
        ├── <video>  (remote H.264 track)
        └── InputBatcher → RTCDataChannel "input" → POSTless scancodes
```

Why `<video>` instead of WebCodecs: the browser already owns H.264 decoding for
WebRTC, including jitter buffering and loss concealment. `WebCodecsSink` stays
for the WebSocket/JPEG tiers. The viewer is still capability-honest: it reports
WebRTC support in the hello and shows the tier the host selected.

## Tasks

| Task | Deliverable | Tests | Docs | Commit |
| --- | --- | --- | --- | --- |
| T1 | Host advertises the bound UDP port (not `--udp`'s requested `:0`); guard media until the session is active; forward keyframe requests to the encoder | Rust unit + serve integration test | inline | `fix(remote): advertise the bound ICE candidate` |
| T2 | `HttpSignalTransport` in `@aether/remote`: fetch signaling, str0m SDP adaptation, ICE-gather wait, track/channel callbacks, error surfacing | `packages/remote/src/transport.test.ts` | JSDoc | `feat(remote): add the HTTP/WebRTC transport` |
| T3 | `viewer.html` + `apps/web/src/viewer/*`: pairing form, video, HUD, input capture, stop, errors; Vite multi-page build | `e2e/viewer.spec.ts` with an injected peer connection | page copy | `feat(web): add the live remote viewer page` |
| T4 | Live verification: serve the built page, pair with a real host, assert connection + data channel; optional `AETHER_REMOTE_LIVE=1` spec | live Playwright run (system Chrome) | runbook | `test(remote): verify the live viewer end to end` |
| T5 | Documentation and release notes: architecture, runbook, FAQ, README, CHANGELOG | doc link check via build | docs | `docs(remote): document the browser live viewer` |

Each task lands with its tests and a green `bun run typecheck`, `bun test`, the
relevant Rust tests, and `bun run build`. Tasks are committed separately so the
roadmap doubles as the change ledger.

## Acceptance

1. `aether-remote serve --code K7QM2H4P` followed by the viewer page reaches
   **streaming** and paints video frames.
2. The offer's `a=candidate` line carries the port the UDP socket actually bound.
3. The HUD updates at least once per second while streaming.
4. Pressing `Esc`/**Stop** closes the session and the host logs the close.
5. Input events arrive on the `input` channel and the host logs each kind.
6. No browser console errors in the e2e run.

## Risks

| Risk | Mitigation |
| --- | --- |
| Browser build lacks H.264 | The page shows the negotiated track state and a decode hint; live test uses system Chrome |
| Host sends no frame before the first IDR | T1 forwards PLI to the encoder; the software path also emits an IDR every two seconds |
| ICE candidates are not trickled | Wait for `iceGatheringState === "complete"` (bounded) before posting the answer |
| Cross-origin fetch | The host already returns permissive CORS headers |
