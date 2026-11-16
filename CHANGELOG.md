## v0.1.0-alpha.77

- Add Mermaid rendering to the docs site (`@docusaurus/theme-mermaid`) and a
  **System diagrams** page: runtime topology, repository map, compositor and
  backend internals, a remote-connection overview and deployment targets. It is
  linked from the architecture overview and the docs sidebar.
- Expand [remote display](/architecture/remote-display) with the detailed
  browser → host connection map, the one-shot pairing sequence, the media/input
  data path (including the still-synthetic capture source and the future virtual
  seat) and the development VM's VNC → noVNC bridge.
- Verification: `bun run --cwd apps/docs build`, plus a Mermaid parse check of
  every diagram block in the two pages.

## v0.1.0-alpha.76

- Add `scripts/vm-browser.sh` to drive the development VM's real desktop from a
  browser. It boots or reuses `./scripts/vm-run.sh --gui`, fetches noVNC into
  `artifacts/novnc` on first use, proxies QEMU's VNC output to
  `http://127.0.0.1:6080/vnc.html` with websockify, and opens Chrome with the
  password pre-filled. `--no-open`, `--no-boot`, `--stop`, `--stop-vm` and
  `--stop-all` cover headless and cleanup workflows; `VM_BROWSER_PORT`,
  `VM_VNC_DISPLAY` and `VM_VNC_PASSWORD` move the endpoints. The bridge is
  loopback-only and is explicitly a tier-4 debugging path, separate from the
  `aether-remote` capture pipeline.
- Document the browser path in the VM README, the VM getting-started guide, the
  remote-display architecture page, the FAQ and the development script table.
- Verification: `bash -n` and `--help`, reuse of an existing noVNC listener, and
  a fresh start/stop on an alternate port (page served, listener PID recorded,
  port closed and pid file removed).

## v0.1.0-alpha.75

- Label the host's frame source so the synthetic test pattern is never mistaken
  for the desktop. `aether-remote serve` now publishes `source` in `/health` and
  in the status document (currently `synthetic`; `compositor` is reserved for the
  real capture binding). The live viewer derives the health URL from the signal
  URL, shows a **synthetic test source** badge over the video and a **Source**
  row in the HUD, and reads **live desktop** if a host reports `compositor`.
- Verification: Rust `aether-remote` tests, mocked viewer e2e with a stubbed
  `/health`, and the opt-in live spec.

## v0.1.0-alpha.74

- Add the **live browser viewer** at `/viewer.html`. It pairs with a real
  `aether-remote serve`: `POST /signal` hello → welcome + str0m offer, an
  `RTCPeerConnection` answer, the H.264 track attached to a `<video>` element,
  and scancodes/pointer motion over the host's `input` data channel. The HUD
  shows session, tier, codec, resolution, fps, bitrate, RTT, decoded frames and
  the channel state; Stop posts the bye and closes the peer.
- Add `HttpSignalTransport` to `@aether/remote`: fetch signaling, nested str0m
  SDP unwrapping, bounded ICE-gathering wait, track/channel/connection callbacks
  and host error surfacing. The marketing `#remote` panel stays a demo over the
  same client state machine and links to the live page.
- Fix `aether-remote serve` advertising the requested `--udp …:0` instead of the
  bound UDP port, which made every browser candidate unreachable. Media now
  waits for an active connection, and RTCP PLI requests force the next encoder
  access unit to be an IDR.
- Build the web app as two Vite entries (`index.html`, `viewer.html`).
- Verification: 22 `packages/remote` tests, Rust `aether-remote` tests (45),
  mocked viewer e2e (2), opt-in live e2e against the host in system Chrome
  (`e2e/viewer-live.spec.ts`, `AETHER_REMOTE_LIVE=1`), and the existing suite.

## v0.1.0-alpha.73

- Give every Wayland client a **server-side Aether frame**. The compositor now
  draws a 34 px title bar per toplevel — design-system mark, title, focus accent
  rule and close pill — and configures the client 34 px shorter, so the terminal
  (foot) finally opens with the same chrome, focus treatment and close
  affordance as the explorer, chat, audio, themes and sandboxes surfaces.
  `xdg-decoration` is still not advertised; clients never draw their own bar.
- Dragging a title bar moves its window, promoting a tile to floating on the
  first movement; the close pill calls `xdg_toplevel.close`. Fullscreen windows
  drop the frame. Pointer hover on the frame no longer forwards client motion,
  and close/body hit-testing uses the content rect.
- Reserve a 48 px `top_gap` in `aether-layout` so tiled, floating and maximized
  clients start below the shortcut dock. Maximize, half-snap and floating
  placement now use the work area instead of the full output; fullscreen still
  covers everything.
- Damage from client commits is clipped and translated against the content
  rect. Focus changes, title updates and window destruction repaint title bars
  rather than relying on the client to redraw compositor chrome.
- Verification: layout work-area test, app frame/content/hit tests, updated
  shell maximize and snap assertions, `cargo fmt` and `clippy -D warnings`.

## v0.1.0-alpha.72

- Add the **Agent sandboxes** surface (`Super+B`, palette **Agents**, top-dock
  **Agents**): registry rows with label, id, trust, tier, PID, UID, CPU/RSS and
  running/stale status; a detail pane with enforcement, limits, capabilities,
  read/write paths, warnings and log path.
- `aether-sandbox run` now publishes a record to `/run/aether/sandboxes`
  (override `AETHER_SANDBOX_REGISTRY`), with new `--id`, `--label`, `--trust`,
  `--capability`, `--log` and `--no-record` options. Records are removed on
  exit; stale records are prunable from the UI. A per-sandbox Unix socket lets
  the shell stop a child running under a reserved UID.
- Stop (`K`), force-stop (`Shift+K`), open log (`L`/`Enter`), refresh (`R`) and
  prune (`C`) actions; records are advisory and a missing PID is never shown as
  running.
- Verification: 4 registry tests, 3 panel-state tests, shortcut/topbar/palette
  tests, clippy `-D warnings`, and a headless Xvfb screenshot run with two live
  sandboxes plus a stale record.

## v0.1.0-alpha.71

- Add `aether-mcp`, a stdio MCP server for the running system: 13 audited tools
  covering status, processes, indexed files, applications, windows, workspaces,
  audio, notifications and shell actions. Read tools are unrestricted; window
  close requires `confirm: true`; every call lands in a rotating JSONL audit log.
- Add the shell control socket (`AETHER_CONTROL_SOCKET`): a listener thread
  queues requests and the compositor executes them with a per-frame budget, so
  clients cannot stall presentation.
- Make the chat drawer promptable: the composer accepts text, Enter sends, and
  tool-call traces render inline. `aether-chat` owns the model loop and MCP
  client, connects to `AETHER_CHAT_SOCKET`, and speaks an OpenAI-compatible
  endpoint (`http://` or `unix:`) or a deterministic script.
- Package `aether-mcp`/`aether-chat`, export the session sockets, add the user
  service and document the trust boundary in `architecture/system-mcp`.
- Verification: 22 new Rust tests (MCP protocol, stdio, chat loop, composer,
  audit), `clippy -D warnings` for both crates, and a headless Xvfb smoke that
  drives the chat drawer and the confirmation gate end to end.

## v0.1.0-alpha.70

- Add the compositor **Themes** app (`Super+T`, `Ctrl+Alt+T` in the Simple
  profile): theme mode, the five Water effect switches, flow speed, drip energy,
  background, tone and glass. The panel mirrors the reference explorer's Water
  settings, including status copy, `Use Water`, reset and arrow-key controls.
- Persist theme state to `$XDG_CONFIG_HOME/aether/themes.json` with the reference
  app's `clickRipples` / `rowMotion` fields; `AETHER_THEME_CONFIG` overrides the
  path. Theme changes apply immediately.
- Derive compositor chrome tokens from theme mode and tone (Water, Dark and Light
  palettes, light-mode accent variants), honour the glass switch in panel fills
  and borders, and rebuild the backdrop from mode, background, caustics and speed.
  The app previews drips, click ripples, waves and caustics.
- Add the Themes app to the palette and shortcut table, and document the settings
  and storage path. Verification: 21 new `aether-shell` tests; `aether-shell`
  and the Wayland lib are `clippy -D warnings` clean.

## v0.1.0-alpha.69

- Add P7 remote display: `aether-remote` (DMA-BUF capture descriptors + copy
  audit, low-latency H.264 software encode, pairing/tokens/audit, virtual-seat
  input, str0m WebRTC transport, no-queue pipeline, latency ledger and the
  `aether-remote` bench/serve CLI).
- Add `@aether/remote`: browser client with WebCodecs and JPEG sinks, capability
  probe, tier ladder, scancode input batching and session stats; plus a marketing
  viewer with e2e coverage.
- Add the `Super+R` remote overlay, recording indicator, status-bar session chip
  and preview screenshots.
- Package `aether-remote`, add the opt-in local-only systemd user unit and iHD
  VA-API packages. Target glass-to-glass and zero-copy gates remain budgets.
- Verification: 42 `aether-remote` Rust tests, 175 workspace Rust tests, 231
  TypeScript tests, 17 Playwright specs. See the P7 completion ledger.

## v0.1.0-alpha.68

- Add design-token constants, a TS/Rust parity check (`test:design`) and
  screenshot pixel probes for the explorer. Align explorer rows with the 30 px
  reference and update the P3 roadmap/ledger.
- Status-bar audit: startup sampling populates CPU/memory immediately.

## v0.1.0-alpha.67

- Centralise shortcuts in a tested keymap table and add a 32-entry MRU focus
  stack with `Super+Tab`/`Super+Shift+Tab`. Focus history updates on client map,
  direct focus and close.

## v0.1.0-alpha.66

- Add the `Super+C` chat drawer: streaming markdown headings, lists, bold and
  inline code, incremental parsing, autoscroll and bounded visible rows.
  `AETHER_CHAT_DEMO=1` streams a sample; the smoke records continuous frames.
- Populate the status bar from a startup sample.

## v0.1.0-alpha.65

- Add the virtualised explorer layer (`Super+E`): cached filtered indices,
  bounded 14-row composition, keyboard/pointer/wheel navigation and file open.
  Release 10k scroll p95 is 0.004 ms; add explorer screenshot and evidence.
- Sample the status bar during composition so memory/CPU values populate.

## v0.1.0-alpha.64

- Add P3 palette modes (`>`, `@`, `/`, `~`), subsequence ranking, a shared
  TS/Rust parity fixture and a 10k-file benchmark (release p95 1.11 ms).
- Strip palette prefixes before sending index queries.
- Add the Rust `aether-index` daemon with inotify, socket queries, a file-backed
  event ring, Nix package and systemd services.

## v0.1.0-alpha.63

- Deduplicate unchanged overlay damage on palette keystrokes; typing damage-area
  p50 is now 289,696 px (overlay + cursor) instead of a full output. Add
  damage-area percentiles to `frame-stats.py` and regenerate typing evidence.

## v0.1.0-alpha.62

- Fix input-path damage precision: palette typing damages the overlay rect,
  pointer motion damages old/new cursor and moved window rects, shortcuts and
  layout changes mark full, and forwarded scroll/keys add no compositor damage.
  Removes the blanket full-scene repaint per input event (winit and DRM).
- Log `cleared_area` in DRM frame stats and regenerate typing/evidence logs.
- Correct the P2 ledger: G1 is implemented but unverified until G5; the
  historical blocker list is marked fixed.

## v0.1.0-alpha.61

- Gate instrumentation: per-output pacing deadlines, damage-aware composition,
  NDJSON frame stats, text caching, partial clears and an async coalescing index
  worker; `scripts/frame-stats.py` summarises the gate metrics.
- Status bar with agents, CPU, memory, audio and frame time; sampling stops when
  hidden/fullscreen and does not create idle frames.
- SVG icons, icon-theme inheritance and D-Bus activation via `busctl`.
- CI builds/tests all three backends and runs curated smokes; full suite nightly;
  a labelled arm64 KVM runner workflow runs the virtio-gpu DRM smoke.
- Reference-measurement runbook and P2 claims/evidence ledger; ADR-0016
  (GLES/wgpu) and ADR-0017 (native DRM/cage).

## v0.1.0-alpha.60

- Make chrome output-local: workspace indicators are drawn and hit-tested on
  every output at that output's origin, the palette opens centred in the output
  under the pointer and renders only there, and workspace switching/divider
  drag use the output under the pointer. Palette/indicator tests cover two
  outputs, overlay reassignment on hotplug and secondary master gaps.
- Translate extended-desktop element positions into each DRM output's local
  space; secondary outputs previously composited off-screen.
- VM harness now runs the compositor for a timed window, unbinds/rebinds the
  GPU and asserts output removal/recreation; the two-output policy is covered
  by shell tests because headless QEMU cannot attach a second display without
  stalling virtual GPU initialization.

## v0.1.0-alpha.58

- Add the shared `aether-shell-wayland` crate: `App` now owns protocol globals,
  seat, input policy, render composition, palette/agent providers and the
  listening socket; winit and DRM adapters are thin backends.
- Add per-output workspaces to `aether-shell`: windows belong to an output,
  each output shows its own workspace, tiling/clamping/maximise/fullscreen use
  that output's bounds, and hotplug reassigns orphaned windows.
- Finish the native `aether-shell-drm` backend: libseat session, udev GPU
  discovery, GBM/EGL renderer, `DrmOutputManager` per connector, VBlank
  page-flips, libinput keyboard/pointer/axis input, connector hotplug, session
  pause/resume and the shared Wayland/render path.
- Add `aether-session`: native DRM first, cage + winit adapter fallback, used by
  the target systemd unit. Package `aether-shell-drm` and `aether-session` in
  Nix; add 3 DRM unit tests, a DRM bring-up smoke script and a VM virtio-gpu
  boot check (`scripts/vm-run.sh --drm-smoke`, `scripts/vm-drm-tcg.sh`).
- Validated the native backend in the aarch64 VM on virtio-gpu: libseat device
  open, `Virtual-1` connector with 26 modes, GLES renderer, 1280x800 CRTC
  mode-set, DrmCompositor output and a presented frame, exiting
  `AETHER_VM_DRM_STATUS=0`. Input initialization is covered by libinput setup
  in the same run; hotplug is covered by shell unit tests.

# Changelog

All notable changes to Aether OS. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project uses
semantic versioning with release codenames (0.1 = “Kestrel”).

## [Unreleased] — installable base

### Added

- **First native backend** — `aether-shell-smithay` (Linux-only, standalone
  Cargo crate) opens a Smithay/winit window. It now accepts xdg-shell/shm
  clients (verified with two real `weston-terminal` instances), maps them into
  `aether-shell` windows so they tile and are configured to their rects, and
  forwards keyboard/pointer focus, buttons and axis. Adds the shell overlay above
  clients (`Super+Space`, modal, `Esc` closes), workspace shortcuts
  (`Super+1/2/3`), window moves (`Super+Shift+1/2/3`), `Super+Q` close and
  labelled workspace indicators. Text is rasterized by `aether-text` and
  uploaded as Smithay memory buffers. Adds a clickable workspace picker with
  chrome pointer capture, advertises `wl_pointer` (fixing a motion panic), titles
  the window "Aether OS" and verifies the picker with an xdotool smoke script.
  Adds interactive master/stack resizing: dragging the divider updates the split
  ratio (0.2–0.8) and reconfigures clients live, verified by an xdotool drag
  smoke. Adds an interactive command palette: typed queries filter workspace
  commands, Up/Down wraps selection, Enter switches workspace or closes, and
  Backspace edits. Input is press-only (shortcuts no longer fire twice) and the
  palette path is verified by an xdotool typing smoke. Adds window results to the
  palette (titles sync through xdg `title_changed`), clickable/hover rows, a
  sliding result window and `FocusWindow` actions that switch to the window's
  workspace. The smoke clicks a row and then focuses a filtered window. Fixes
  compositor chrome draw order so panels/rows layer correctly. Adds window
  management: `Alt+left-drag` promotes a tile to its preferred floating size and
  moves it, floating positions persist across relayouts, and `Super+M` maximizes
  with restore. Adds `Alt+right-drag` corner resizing with a 160x120 minimum
  and output-edge snapping while moving. Adds release zones: left/right snap to
  a half and the top edge maximizes; snapped halves remain floating/resizable.
  Client rendering now follows shell z-order instead of HashMap order, fixing
  raised/floating windows being covered. App/file/agent providers, IME,
  fullscreen, multi-output snapping, cursor theming, per-client damage and
  udev/DRM remain. Nix dev shell `.#smithay`, demo/client/picker/resize/palette/
  window smoke scripts, ADR-0012.
- **Backend-neutral shell state** — `aether-shell` owns windows, workspaces, focus,
  floating windows, the overlay and `FramePlan`, with no Wayland/GPU dependency.
  `aether-preview` renders frame plans to PNG (`aether-shell-preview`), providing
  visual verification before the Smithay backend (ADR-0011).
- **Bitmap text** — `aether-text` rasterizes 8x8 glyphs into cropped RGBA
  buffers with no fontconfig/FreeType dependency; shared by `aether-preview` and
  the Smithay backend for workspace labels and overlay copy. ADR-0013.
- **Compositor crate split** — `aether-geometry`, `aether-damage`, `aether-layout`,
  `aether-pacing`, `aether-scene` and the `aether-compositor` facade/prelude/demo.
  Apps can depend on the smallest crate they need; the P2 Smithay backend will be
  a separate crate above this workspace. ADR-0010.
- **Installer ISO** (`nixosConfigurations.installer`, `packages.iso`) — a minimal
  NixOS live environment with `git`, `disko`, `helix`, `rsync` and hardware probes.
- **Development VM** (`packages.aarch64-linux.vm-image`) — a separate `os/vm`
  target that boots in QEMU/HVF on this Mac, with build/run/smoke scripts and a
  documented separation policy from the 7700K install.
- **Shared composition layer** (`os/common/base.nix`) and an automated
  `tests/separation.test.ts` boundary: the physical and VM targets may share
  only target-agnostic settings, and neither may import the other.
- **disko layout** (`os/nixos/disko.nix`) — GPT ESP, XFS root, XFS `/nix`, ext4
  scratch, XFS `/home`, with filesystem labels matching `filesystem.nix`.
- **One-line disk target** (`os/nixos/disk.nix`) and `scripts/install.sh`, which
  validates UEFI/device, confirms destructively, runs disko and installs the flake.
- **Install documentation** for the i7-7700K: install, first boot and hardware
  notes, including firmware settings, manual commands, recovery and rollback.
- `diskoConfigurations.kestrel` flake output and a `disko` dev-shell package.
- `aether-scene` is now packaged and installed; `helix`, `tmux`, `pciutils`,
  `usbutils` and `alsa-utils` are in the system profile.
- README and docs now link the install path.

### Fixed

- The development VM image now uses NixOS's built-in image support
  (`system.build.images.qemu-efi`) instead of the deprecated `nixos-generators`
  input, removing its flake evaluation warning. The `qemu-efi` target produces
  the same qcow2 + ESP + systemd-boot image.
- The marketing “Full roadmap” CTA pointed at `/docs/roadmap/roadmap`, which does
  not exist (the page is `/docs/roadmap/`). All in-site docs links now use the
  canonical trailing slash, and a Playwright spec checks every docs link on the
  marketing page resolves.
- Flake evaluation failed because `security.polkit.enable = false` conflicted with
  NetworkManager; polkit is now left to NetworkManager.
- Removed options removed in current nixpkgs: `services.journald.extraConfig` and
  `boot.initrd.systemd.enable`.
- Renamed `boot.tmpOnTmpfs`/`boot.cleanTmpDir` to `boot.tmp.useTmpfs`/`cleanOnBoot`.
- Added `xfs` and `ext4` to the initrd module set so the root can mount.
- Moved the flake to the repository root and applied the Aether overlay to the
  NixOS `pkgs` instance; without this, `aether-scene` in `systemPackages` was
  undefined at install time.
- disko partition sizes are absolute (80G/120G/60G) with build-time assertions;
  disko does not support arbitrary percentages other than `100%`.
- The install script exports `NIX_CONFIG` so flakes work on the official NixOS
  minimal ISO, and checks the root flake path.
- Removed `thermald` (no DPTF on this desktop board) and unused network sysctls.
- `flake.lock` is now committed at the repository root; `flake check --no-build` passes for both
  the installed system and the installer.

### Known limitations

- Not booted on the reference machine yet; evaluation is verified, boot is not.
- No compositor/daemons at first boot (P2/P3); the session is a TTY.
- No LUKS layout and no Legacy BIOS path in the default config.

## [0.1.0-alpha.0] — 2026-11-12

Foundation release. Everything below is in the repository and tested; nothing
below is a bootable OS yet.

### Added

- **Monorepo** with Bun workspaces, strict TypeScript, a Rust workspace and a
  single lockfile.
- **protocol** — documents, capabilities, agent specs, isolation plans, the
  Kaby Lake hardware profile, IPC message kinds, and dependency-free parsers.
- **indexer** — Unicode-aware tokenizer with highlight mapping, BM25F-lite
  ranking, prefix expansion, bounded edit distance, recency/usage boosts,
  snapshot round-trips and filesystem ingestion (`@aether/indexer/fs`).
- **ipc** — 64-byte header SPSC ring with wrap markers, backpressure and a
  zero-copy pop path, plus a capability table with expiry, delegation and
  cascading revocation. Cross-thread test drives 5,000 messages through a
  Worker over a `SharedArrayBuffer`.
- **agents** — isolation planner (no-PKU policy), 4c/8t admission controller,
  priority scheduler with head-of-line avoidance, capability-gated collaboration
  bus and a hash-chained audit log.
- **palette** — headless engine with mode prefixes, concurrent providers,
  deterministic ranking, dedupe by provider priority, stale-query detection and
  keyboard navigation, plus a search-index adapter.
- **ui** — React components on the Water design system: command palette,
  streaming chat transcript with a safe markdown subset, two-pane explorer,
  status bar, badges and meters.
- **design-system** — vendored Water tokens, primitives, water effects and a
  component gallery, with provenance and a divergence policy.
- **web** — marketing site with a live palette, interactive explorer, real
  isolation plans generated by `planIsolation()`, a budget ledger and a roadmap.
- **docs** — 44 pages covering product, architecture, design, operations,
  reference and nine ADRs, including a claims-and-evidence audit.
- **bench** — reproducible harness for index, ring, capability and palette
  metrics, with a committed baseline and a regression gate.
- **os/nixos** — flake, Kaby Lake hardware module, kernel config fragment, XFS
  root / ext4 scratch / tmpfs index, PipeWire services and account model.
- **os/compositor** — dependency-free Rust crate for geometry, damage regions,
  z-ordered scene graph, tiling layout and frame pacing, with a demo binary.
- **CI** — typecheck, unit/DOM tests, Rust tests, Playwright, a benchmark smoke
  job and a GitHub Pages deployment of the assembled site.

### Fixed

- Ring consumer advanced `tail` before copying the payload, allowing the producer
  to overwrite the message being read. The slot is now released only after the
  handler returns.
- Index queries computed highlights for every candidate; moving highlight work
  after top-k reduced query p50 from 6.9 ms to 1.65 ms at 5k documents.
- Palette scored each match twice (filter + rank); removing the duplicate pass cut
  p50 by 23%.
- Docusaurus hydration mismatch caused by whitespace-generated empty paragraphs
  in the intro; replaced with a Markdown table.
- Node-only filesystem ingestion leaked into the browser bundle via the indexer
  barrel; split into `@aether/indexer` and `@aether/indexer/fs`.

### Known limitations

- No bootable image. Kernel and NixOS config are written but have not been
  booted on the reference machine.
- No Smithay/libinput integration; only the pure scene core exists.
- No Landlock/seccomp profiles running against real agents yet.
- No remote display, browser or media implementation.
- Screen-reader support is not implemented.
- Absolute performance numbers are reference-host measurements (Apple M3), not
  measurements on the i7-7700K.
