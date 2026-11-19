# Screenshot automation

Two capture pipelines feed the gallery, the docs and the README:

| Pipeline | Source | Output |
| --- | --- | --- |
| `capture-shell-screenshots.sh` | Smithay winit adapter under Xvfb | `artifacts/screenshots/shell` |
| `capture-web-screenshots.sh` | assembled site + Playwright | `artifacts/screenshots/web` |
| `publish-screenshots.ts` | curated copies + manifest | `apps/web/public/screenshots`, `apps/docs/static/img` |
| `os-parity.sh` + `os-parity.mjs` | live Water OS demo vs compositor (full frame + regions) | `artifacts/os-parity` |
| `app-parity.mjs` | source checkout vs compositor, per app/window element | `reports/os-app-parity` |
| `design-state.mjs` | source computed styles vs `design-manifest` (tokens, type, geometry) | `reports/os-app-parity/state` |
| `scene-diff.mjs` | source DOM text runs vs the Aether render chunk | `artifacts/screenshots/shell/scenes`, `state/scene-*` |
| `design-docs.mjs` | DESIGN.md token/component contract for both sides | `design/`, `state/design-md-*` |

The measured distance to a strict parity target, the rubric and the remaining
work are documented in
[`reports/os-app-parity/PARITY-100.md`](../../reports/os-app-parity/PARITY-100.md).

## Run everything

```bash
bun run screenshots
```

The script:

1. Captures the compositor surfaces. On macOS it builds the Rust adapter in a
   disposable Linux container; on Linux it runs natively.
2. Builds the site and captures the marketing pages, remote viewer, docs and
   design-system gallery with Playwright.
3. Publishes the curated images and writes
   `artifacts/screenshots/{index.html,manifest.json}`.
4. Rebuilds `apps/web/dist` so the local preview uses the new images.

## Capture one pipeline

```bash
bun run screenshots:shell
bun run screenshots:web
```

Useful flags:

```bash
# Only the surfaces you changed
./scripts/capture-shell-screenshots.sh --only palette,explorer,chat

# Browser only, reusing the current dist build
./scripts/capture-web-screenshots.sh --no-build

# Refresh the Docker capture image explicitly
./scripts/capture-shell-screenshots.sh --docker --rebuild
```

`capture-shell-screenshots.sh --list` prints every surface name.

## Design state

Pixels catch shape; the design-state pass catches values. Both projects emit a
flat key/value document and the differ compares them property by property:

```bash
bun run parity:state       # extract source + Aether, diff, write state/
bun run parity:state:diff  # re-diff existing JSON (no browser/container)
```

- The demo side is read with Chromium's `getComputedStyle`, so cascade,
  `color-mix()`, custom properties and the 1.14 OS font scale are resolved.
- The Aether side is `aether-shell-wayland --bin design-manifest`, which calls
  the same layout and token code the renderer paints with.
- Numbers allow 0.75 px, colours 2/255 per channel.
- Deliberate differences are pinned in `state/baseline.json` with a reason.
  Anything else exits non-zero, so drift fails the check.

## Scene and contract

The state pass compares values; the scene pass compares what was laid out:

```bash
bun run parity:scene   # diff source DOM text runs against the Aether scene
bun run parity:design  # regenerate both DESIGN.md files and diff the contract
```

- Source scenes are written by `app-parity.mjs` from a visible-element walk of
  the demo (`getBoundingClientRect` + `getComputedStyle` per element).
- Aether scenes are written by the compositor when it is built with the
  **debug-only `scene-dump` feature** and `AETHER_SCENE_DUMP` is set: the
  semantic node tree (role, rect, text, style) plus the raw render chunk's
  chrome, labels, icons and panels. Capture images build with the feature;
  production builds leave it off, so none of the recorder ships.
- `scene-diff.mjs` matches non-numeric text runs by text and nearest position,
  records geometry offsets, and stores per-surface coverage in
  `state/scene-baseline.json`; a drop fails the run.
- `design-docs.mjs` writes `design/source/DESIGN.md` (the contract) and
  `design/aether/DESIGN.md`, lints both, and diffs them. Deliberate differences
  live in `design/accepted.json`; known missing UI lives in `design/open.json`
  and is reported as debt, not parity.

## Publish without recapturing

```bash
bun run screenshots:publish
```

The publish step is strict about missing files only with `--strict`; otherwise it
reports the missing capture and keeps going. That makes partial `--only` runs
useful.

## What is captured

Shell surfaces: menu bar, dock, palette, explorer, chat, audio, themes, task manager,
calendar, the first-party apps (About, Network, Battery, Devices, Tide Radio,
Settings — every pane) and their menu-bar trays,
remote overlay, shortcuts, window move/resize/snap/maximize, fullscreen,
workspace picker, cursor, clipboard, desktop/file/agent/index providers and the
partial-damage probe.

Browser surfaces: the full marketing page and each section, the screenshot
lightbox, the remote viewer idle/live/error states, the docs pages, the Water
component gallery and a mobile viewport.

The compositor boots with the dock hidden, matching the demo. The `topbar` and
`os-parity` smokes set `AETHER_SHOW_DOCK=1` because they click the dock;
`AETHER_HIDE_DOCK` still force-hides it for app-only crops.

The index/agent-IPC/indexer smokes need Bun and the JavaScript indexer. The
native Nix shell and the Docker image both provide them. Clipboard persistence is
native-only because Debian does not package `wl-clip-persist`; the regular
clipboard capture still runs in Docker.

## CI

`.github/workflows/screenshots.yml` runs weekly and on demand. It captures both
pipelines, uploads the PNGs and contact sheet, and opens a review PR with only
the curated images under `apps/web/public/screenshots` and
`apps/docs/static/img`.

## Requirements

- Web: Bun, Playwright Chromium (`bunx playwright install chromium`), and the
  assembled site (`bun run build`).
- Shell on Linux: `nix develop .#smithay` (Xvfb, xdotool, ImageMagick, foot,
  Wayland/Mesa libraries).
- Shell on macOS: Docker. The capture image is built from
  `scripts/screenshots/Dockerfile` from the repository root. The build is
  cached; `--rebuild` forces a fresh image.