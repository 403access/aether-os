---
id: diagrams
title: System diagrams
sidebar_position: 2
description: Rendered maps of the repository, runtime topology, compositor internals, deployment targets and the remote connection.
---

# System diagrams

These are the maintained maps of Aether OS. The diagrams are Mermaid source, so
they render on this page and stay reviewable in the repository. Prose and
evidence for each subsystem live in the linked pages; this page is the visual
index.

- [Architecture overview](/architecture/overview) — the four-layer model, process
  table and trust boundaries in prose.
- [Remote display](/architecture/remote-display) — the detailed connection map,
  pairing sequence and media/input path.
- [Smithay backend](/architecture/smithay-backend) — the Wayland client
  compositor, server-side window frames and input routing.
- [Development VM](/getting-started/virtual-machine#browser-access) — the noVNC
  bridge that puts the VM desktop in a browser.

## Runtime topology

What runs in a session, and what crosses its boundaries.

```mermaid
flowchart TB
  U["User<br/>keyboard · pointer · touchpad"]

  subgraph Session["Aether user session"]
    SHELL["aether-shell<br/>compositor · layout · scene · input"]
    SURF["In-shell surfaces<br/>palette · chat · explorer · audio · themes · remote · task manager · sandboxes"]
    CLIENT["Client surfaces<br/>foot terminal · WebKit · libmpv"]
    INDEX["aether-index<br/>BM25F over a Unix socket"]
    REMOTE["aether-remote serve<br/>signaling · WebRTC · H.264"]
    BASE["NixOS base<br/>systemd · PipeWire · udev · XFS/ext4"]
  end

  DS["Local DeepSeek endpoint<br/>Unix socket"]
  BROWSER["Browser tab<br/>viewer.html or noVNC"]
  AGENT["Agents<br/>separate UIDs · Landlock"]

  U --> SHELL
  SHELL -->|"renders"| SURF
  SHELL -->|"input"| CLIENT
  CLIENT -->|"buffers"| SHELL
  SHELL -->|"queries"| INDEX
  INDEX -.->|"ranked hits"| SHELL
  DS -->|"token stream"| SHELL
  REMOTE -->|"status JSON"| SHELL
  REMOTE -.->|"H.264 + input channel"| BROWSER
  AGENT -.->|"capability-checked IPC"| SHELL
  SHELL --> BASE
  CLIENT --> BASE
  INDEX --> BASE
  REMOTE --> BASE
```

First-party surfaces are shell layers in the compositor process, not external
clients. External windows are Wayland clients (foot, WebKit, libmpv) wrapped in
the compositor-drawn title bar. `aetherd`, the capability broker, is planned;
today the index and sandbox services are the session-level daemons.

## Repository map

Where each piece lives and what it depends on.

### Where things live

```mermaid
flowchart TB
  subgraph Apps["apps/"]
    direction LR
    WEB["web<br/>marketing + live viewer"]
    DOCS["docs<br/>Docusaurus"]
    IDXAPP["indexer-service<br/>Bun + Unix socket"]
  end

  subgraph Packages["packages/"]
    direction LR
    PROTO["protocol"]
    INDEXPKG["indexer"]
    IPC["ipc"]
    AGENTS["agents"]
    PALETTE["palette"]
    UI["ui"]
    DESIGN["design-system"]
    REMOTE["remote<br/>browser client"]
    BENCH["bench"]
  end

  subgraph OS["os/"]
    direction LR
    NIXOS["nixos<br/>hardware target"]
    VM["vm<br/>aarch64 dev image"]
    LIVE["live"]
    OINDEX["index<br/>aether-index"]
    COMP["compositor<br/>Rust workspace"]
  end

  subgraph Tooling["tooling"]
    direction LR
    SCRIPTS["scripts/<br/>vm-browser · vm-run · assemble"]
    E2E["e2e/<br/>Playwright"]
  end
```

### Web and package flow

```mermaid
flowchart TB
  E2E["e2e/"] --> WEB["apps/web"]
  WEB --> UI["packages/ui"]
  WEB --> REMOTE["packages/remote"]
  WEB --> PROTO["packages/protocol"]
  DOCS["apps/docs"] --> DESIGN["packages/design-system"]
  UI --> DESIGN
  IDXAPP["apps/indexer-service"] --> INDEXPKG["packages/indexer"]
  INDEXPKG --> PROTO
  AGENTS["packages/agents"] --> PROTO
  PALETTE["packages/palette"] --> PROTO
```

`packages/ipc` and `packages/bench` are supporting libraries (ring buffer and
benchmark gate); they do not sit on the UI path shown here.

### OS flow

```mermaid
flowchart LR
  NIXOS["os/nixos<br/>hardware target"] --> COMP["os/compositor"]
  VM["os/vm<br/>aarch64 dev"] --> COMP
  LIVE["os/live"] --> NIXOS
  COMP --> OINDEX["os/index/aether-index"]
  COMP --> REMOTE["aether-remote crate"]
  SCRIPTS["scripts/"] --> VM
```

## Compositor and backends

One backend-neutral shell and three entry points, followed by the Wayland
render/input path that turns shell state into a frame.

### Shell core and backends

```mermaid
flowchart LR
  WL["aether-shell-wayland<br/>Wayland client compositor"] --> POLICY["shell policy<br/>focus · floating · snap · fullscreen"]
  SM["aether-shell-smithay<br/>winit dev host"] --> POLICY
  DRM["aether-shell-drm<br/>KMS/DRM session"] --> POLICY
  POLICY --> MODEL["model<br/>windows · outputs · workspaces"]
  POLICY --> LAYOUT["aether-layout<br/>tile · work_area · top_gap"]
  POLICY --> SCENE["aether-scene<br/>z-order · hit testing · damage"]
  POLICY --> OVERLAYS["overlay models<br/>palette · chat · explorer · audio · themes · remote · task manager · sandboxes"]
  OVERLAYS --> THEME["theme<br/>Water tokens"]
```

### Wayland render and input path

```mermaid
flowchart LR
  WL["aether-shell-wayland"] --> COMPOSE["compose<br/>surfaces · chrome · cursor"]
  WL --> INPUT["input<br/>pointer · keyboard · IME"]
  WL --> DAMAGEH["damage<br/>content rects · clipped commits"]
  COMPOSE --> FRAME["window_frame<br/>title · focus · close"]
  COMPOSE --> SURFACES["surfaces/<br/>one module per shell surface"]
  COMPOSE --> GEO["aether-geometry"]
  COMPOSE --> TEXT["aether-text"]
  COMPOSE --> CACHE["aether-cache"]
  INPUT --> CURSOR["aether-cursor"]
  INPUT --> PROTOCOL["aether-protocol"]
  DAMAGEH --> DAMAGE["aether-damage"]
```

The remaining shared crates are small and single-purpose:

| Crate | Role |
| --- | --- |
| `aether-pacing` | Frame pacing and present timing |
| `aether-preview` | Software renderer for a `FramePlan` (preview binary, docs) |
| `aether-files`, `aether-desktop` | Palette file and desktop-entry providers |
| `aether-audio` | PipeWire route model shared by the CLI, overlay and tests |
| `aether-system` | `/proc` sampler for the task manager |
| `aether-chat` | Chat token stream parsing and markdown segmentation |
| `aether-mcp` | Model Context Protocol server |
| `aether-sandbox` | Kernel-enforced agent sandbox (Landlock/seccomp/UID) |
| `aether-protocol` | Wire types shared by the shell and local services |

The Wayland backend is the product compositor. The Smithay winit adapter is a
development/CI host that runs the same shell and client handling under
Xvfb. The DRM backend owns the real KMS session on the hardware target and the
VM. Render modules (`compose`, `window_frame`, `surfaces`) live in the Wayland
crate because that crate also owns the shared composition path used by the DRM
backend.

## Remote connection at a glance

The browser talks to `aether-remote serve` over two channels: HTTP signaling for
pairing and WebRTC for media plus input. The detailed map, sequence and data
path are on [remote display](/architecture/remote-display#connection-map).

```mermaid
flowchart LR
  V["apps/web/viewer.html<br/>@aether/remote client"]
  S["aether-remote serve<br/>127.0.0.1:7443"]

  V -->|"POST /signal hello"| S
  S -->|"welcome + str0m offer"| V
  V -->|"SDP answer"| S
  S <-->|"DTLS-SRTP · H.264"| V
  V -->|"input data channel"| S
  S -->|"status JSON"| O["aether-shell<br/>remote overlay · Super+R"]
  C["Compositor output"] -.->|"DMA-BUF capture · future"| S
```

## Deployment targets

The same shell state runs on three bodies, and the browser bridge is a fourth
way to look at it.

```mermaid
flowchart LR
  subgraph Target["Hardware target · i7-7700K"]
    T1["aether-shell-drm<br/>KMS/DRM"]
    T2["custom Kaby Lake kernel"]
    T3["XFS root · ext4 scratch"]
    T1 --> T2 --> T3
  end

  subgraph DevVM["aarch64 dev VM · QEMU/HVF"]
    V1["aether-shell-drm on tty8"]
    V2["writable qcow2 overlay"]
    V3["QEMU VNC<br/>127.0.0.1:5905"]
    V1 --> V2
    V1 --> V3
  end

  subgraph LinuxDev["Linux dev host / CI"]
    L1["Xvfb + aether-shell-smithay"]
    L1 --> L2["Playwright · smoke scripts"]
  end

  subgraph Mac["Browser access"]
    M1["scripts/vm-browser.sh"]
    M2["websockify + noVNC<br/>127.0.0.1:6080"]
    M3["Chrome tab"]
    M1 --> M2 --> M3
  end

  M1 -->|"boot or reuse"| DevVM
  M2 -->|"RFB"| V3
```

## How to change these diagrams

Edit the Mermaid block in this page or in
[remote display](/architecture/remote-display), then build the docs:

```bash
bun run --cwd apps/docs build
```

Keep node names stable and labels short; link to the owning page for detail
instead of duplicating component behaviour here.