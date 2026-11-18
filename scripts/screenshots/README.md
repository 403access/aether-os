# Screenshot automation

Two capture pipelines feed the gallery, the docs and the README:

| Pipeline | Source | Output |
| --- | --- | --- |
| `capture-shell-screenshots.sh` | Smithay winit adapter under Xvfb | `artifacts/screenshots/shell` |
| `capture-web-screenshots.sh` | assembled site + Playwright | `artifacts/screenshots/web` |
| `publish-screenshots.ts` | curated copies + manifest | `apps/web/public/screenshots`, `apps/docs/static/img` |
| `os-parity.sh` + `os-parity.mjs` | live Water OS demo vs compositor (full frame + regions) | `artifacts/os-parity` |
| `app-parity.mjs` | source checkout vs compositor, per app/window element | `reports/os-app-parity` |

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

## Publish without recapturing

```bash
bun run screenshots:publish
```

The publish step is strict about missing files only with `--strict`; otherwise it
reports the missing capture and keeps going. That makes partial `--only` runs
useful.

## What is captured

Shell surfaces: menu bar, dock, palette, explorer, chat, audio, themes, task manager,
calendar,
remote overlay, shortcuts, window move/resize/snap/maximize, fullscreen,
workspace picker, cursor, clipboard, desktop/file/agent/index providers and the
partial-damage probe.

Browser surfaces: the full marketing page and each section, the screenshot
lightbox, the remote viewer idle/live/error states, the docs pages, the Water
component gallery and a mobile viewport.

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