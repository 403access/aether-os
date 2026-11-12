---
id: build-and-release
title: Build and release
sidebar_position: 1
description: Repo scripts, CI stages, GitHub Pages assembly, versioning and rollback.
---

# Build and release

Everything is built from a clean checkout by one command. The OS image and the web
surfaces share the same repository and version number.

## Commands

| Command | What it does |
| --- | --- |
| `bun install` | Install workspaces (one lockfile) |
| `bun run typecheck` | Strict TypeScript across the monorepo |
| `bun run test:unit` | All package tests (`bun test`) |
| `bun run test:e2e` | Playwright against both built sites |
| `bun run build` | Web → docs → assemble into `apps/web/dist` |
| `bun run verify` | typecheck + unit tests + build |
| `bun run bench` | Benchmarks; `--check` enforces the baseline |
| `bun run os:test` | Rust compositor-core tests |
| `nix build .#iso` | Bootable image (P1) |

## Output layout

```text
apps/web/dist/
├── index.html                 # marketing site
├── assets/                    # JS + CSS
├── favicon.svg
├── design-system/             # standalone Water gallery
│   ├── index.html
│   ├── tokens.css
│   ├── components.css
│   └── …
└── docs/                      # Docusaurus build
    ├── index.html
    ├── assets/
    └── …/                     # doc routes
```

One artifact, one deployment. The marketing site owns `/`, the docs own `/docs/`,
the gallery owns `/design-system/`.

## CI pipeline

```yaml
jobs:
  verify:      typecheck, unit tests, Rust tests, build
  e2e:         install chromium, build, playwright
  bench:       bun run bench --check
  deploy:      main only → GitHub Pages
```

Rules:

- `verify` is required for every PR.
- `e2e` runs against the **built** sites, not the dev server.
- `bench` is required but tolerant (25% threshold).
- `deploy` runs only on `main`, after `verify`.
- No secrets in PR workflows. No telemetry actions.

## Pages assembly

`scripts/assemble.ts`:

1. Copies `apps/docs/build` → `apps/web/dist/docs`.
2. Copies `packages/design-system` (minus package metadata) → `dist/design-system`.
3. Rewrites the showcase's favicon path.
4. Writes a `404.html` that redirects to the site root (SPA-safe fallback).
5. Fails if `dist/index.html` or `dist/docs/index.html` is missing.

Environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `BASE_PATH` | `/` | Vite base for the marketing site |
| `DOCS_BASE_URL` | `/docs/` | Docusaurus base URL |

For GitHub Pages at `https://<user>.github.io/aether-os/`:

```bash
BASE_PATH=/aether-os/ DOCS_BASE_URL=/aether-os/docs/ bun run build
```

## Versioning

- One version for the monorepo: `0.1.0-alpha.0` today.
- Release codenames: birds. `0.1` = **Kestrel**.
- Docs and site display the version; the bench baseline records it.
- Breaking protocol changes bump `PROTOCOL_VERSION` in `packages/protocol`.
- Tags: `v0.1.0` after the release checklist passes.

## Release checklist

- [ ] `bun run verify` green.
- [ ] `bun run test:e2e` green on the built artifact.
- [ ] `bun run bench --check` green.
- [ ] `cargo test` green.
- [ ] Docs build with no broken links.
- [ ] Claims & evidence updated with new measurements.
- [ ] Roadmap status updated (phase, gate).
- [ ] `CHANGELOG.md` entry (Keep a Changelog format).
- [ ] Tag and publish.

## Rollback

### Web surfaces

Pages keeps the previous deployment; re-run the workflow on the previous commit, or
use the Pages history. There is no database and no migration.

### OS image

NixOS generations are the rollback mechanism:

1. Select the previous generation in systemd-boot.
2. `nixos-rebuild switch --rollback` for the service set.
3. Data volumes are untouched.

### Protocol

`PROTOCOL_VERSION` mismatch is a hard error, not a silent downgrade. Mixed versions
are not supported across a reboot; the shell and daemons start together.

## Reproducibility

- `bun.lock` and `Cargo.lock` are committed.
- `flake.lock` pins nixpkgs and the Rust toolchain.
- CI uses the same Bun version as the lockfile (`packageManager` field).
- Web builds are deterministic given the same inputs; timestamps are not injected
  into the bundle.
- Benchmarks are **not** reproducible across hosts by design; they are compared on
  one host and gated relatively.

## Publishing to GitHub

The environment used to build v0.1 did not have an authenticated `gh`. The publish
script handles the first push:

```bash
gh auth login
./scripts/publish.sh
```

It creates the repository, pushes `main`, enables Pages with the GitHub Actions
source, and prints the expected URLs. It refuses to run if the working tree is dirty.
