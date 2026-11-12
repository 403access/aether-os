# Changelog

All notable changes to Aether OS. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project uses
semantic versioning with release codenames (0.1 = “Kestrel”).

## [Unreleased] — installable base

### Added

- **Backend-neutral shell state** — `aether-shell` owns windows, workspaces, focus,
  floating windows, the overlay and `FramePlan`, with no Wayland/GPU dependency.
  `aether-preview` renders frame plans to PNG (`aether-shell-preview`), providing
  visual verification before the Smithay backend (ADR-0011).
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
