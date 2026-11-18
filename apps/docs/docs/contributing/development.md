---
id: development
title: Development setup
sidebar_position: 1
description: Prerequisites, commands, repo layout, conventions and troubleshooting.
---

# Development setup

## Prerequisites

| Tool | Version | Why |
| --- | --- | --- |
| Bun | ≥ 1.3 | Install, tests, build, scripts |
| Node | 26 (optional; Bun is the primary runtime) | Some tooling and IDE integrations expect Node. `.node-version` records it. |
| Rust | 1.99.1 (pinned in `rust-toolchain.toml`; MSRV 1.85) | Compositor/shell core |
| Git | any recent | History |
| Playwright browsers | installed via `bunx playwright install chromium` | E2E |
| Nix | optional | OS image, `nix flake check` |

macOS is fine for the TypeScript/Rust work. The NixOS image requires Linux.

## First run

```bash
bun install
bun run dev:web      # http://localhost:4321
bun run dev:docs     # http://localhost:4322/docs/
bun run test:unit
bun run typecheck
bun run os:test
./scripts/vm-build.sh   # optional: build the aarch64 VM (OrbStack/Nix)
./scripts/vm-run.sh
```

The docs dev server uses `baseUrl: /docs/`, so open
`http://localhost:4322/docs/` rather than the bare port.

## Scripts

| Script | Effect |
| --- | --- |
| `bun run build` | Build web, docs, assemble into `apps/web/dist` |
| `bun run verify` | typecheck + unit tests + build |
| `bun run verify:ci` | Replay the GitHub workflow jobs locally (`--list`, `--job NAME`, `--all`) |
| `bun run lint:workflows` | `actionlint` + ShellCheck over `.github/workflows` |
| `bun run test:e2e` | Playwright against built sites |
| `bun run bench` | Benchmarks (`--check` gates) |
| `bun run os:test` | `cargo test` for the compositor core |
| `./scripts/vm-browser.sh` | Boot/reuse the dev VM and open its desktop in a browser (noVNC); `--stop` ends the bridge |
| `./scripts/clean.sh` | Remove build/test artifacts; `--all` also removes dependencies |

## Checking CI before pushing

A red run costs a push and a queue slot. Both checks below run the same
instructions the workflows run, so a local pass is evidence and a local failure
is the real error, with the log in front of you.

```bash
bun run lint:workflows              # milliseconds: YAML, expressions, run: shell
bun run verify:ci                   # verify, rust, e2e, bench, pages
bun run verify:ci --job rust        # one job while iterating
bun run verify:ci --all             # also the Nix-only smithay job
bun run verify:ci --list            # job names and the workflow each mirrors
```

`lint:workflows` wraps `actionlint` (install with
`brew install actionlint shellcheck`). It catches what only shows up after a
push: bad `${{ }}` expressions, unknown action inputs, dangling `needs`
references, runner labels and ShellCheck findings inside `run:` blocks. Custom
self-hosted labels are declared in `.github/actionlint.yaml`, not ignored.

`verify:ci` (`scripts/ci-local.sh`) replays each step verbatim and keeps going
after a failed job, so one invocation shows every broken job. Three jobs are
deliberately out of scope because the runner is the point of them: `drm-vm`
needs a labelled arm64 KVM host, `screenshots` needs the Nix smithay shell plus
Xvfb, and `nix` evaluates a machine-specific flake.

The `rust` job is only authoritative on Linux. Parts of the compositor are
cfg-gated per target (`SystemMonitor::sample_linux` and friends), so clippy on a
Mac compiles stub branches that CI never sees and reports dead code. Off Linux
the script runs `cargo fmt` and skips clippy/test with that reason; `--native`
forces them anyway.

`act` (also from `brew install act`) executes the YAML in a Linux container,
which is useful when you have changed workflow wiring itself:

```bash
act -l                      # jobs, no containers
act -j verify               # one job in Docker
act -j verify -n            # dry run: show the plan
act -j verify --container-architecture linux/amd64
```

It cannot reproduce the Nix jobs, GitHub Pages, or the self-hosted runners, and
its container is still not `ubuntu-latest`, so treat it as a wiring check rather
than a parity guarantee.

## Cleaning generated state

Generated state is never committed, but it grows quickly: the Rust targets,
Docusaurus output, screenshots and VM images can exceed 10 GiB.

```bash
./scripts/clean.sh --dry-run   # show sizes and paths
./scripts/clean.sh --yes       # remove builds, artifacts, Nix results
./scripts/clean.sh --all --yes # also node_modules and local caches
./scripts/clean.sh --docker --yes # also remove the aether-vm-nix volume
```

The default removes `artifacts/`, every Rust `target/`, `apps/web/dist`,
`apps/docs/build`, `.docusaurus`, reports and Nix result symlinks. It never
touches source, docs, committed screenshots or the git tree. `--docker` is
explicit because it removes the shared Nix store volume used by VM builds.

## Repository layout

```text
apps/web              Vite + React marketing site and live demos
apps/docs             Docusaurus documentation
packages/protocol     types + validators (no deps)
packages/indexer      search engine + filesystem ingestion
packages/ipc          ring buffer + capability table
packages/agents       isolation planner, scheduler, collaboration, audit
packages/palette      headless palette + index adapter
packages/ui           React components on the design system
packages/bench        benchmark harness + baseline
packages/design-system  vendored Water tokens/components/effects
os/common             the only Nix module shared by both targets
os/nixos              reference hardware target and installer
os/vm                 separate aarch64 development VM
tests                 architectural boundary tests
os/compositor         Rust crates: geometry/damage/layout/pacing/scene + facade
e2e                   Playwright specs
scripts               assemble, publish, verify helpers
```

## Conventions

### TypeScript

- Strict mode, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`.
- No `any`. Use `unknown` and a parser.
- Pure logic in packages; DOM only in `apps/web` and `packages/ui`.
- Inject clocks (`now: () => number`) instead of calling `Date.now()` in logic.
- Prefer immutable data and explicit option objects.

### Rust

- `#![forbid(unsafe_code)]` in the pure core crates.
- Tests in the same file or `tests/` for integration.
- No GPU dependencies in the pure crates; keep CI fast.
- Format with `cargo fmt`; clippy warnings fail CI.

### Commits

Conventional commits, one logical change per commit:

```text
feat(indexer): add prefix expansion
fix(ipc): copy payload before releasing the slot
docs(adr): record XFS decision
test(ui): cover palette keyboard navigation
chore: bump bun lockfile
```

The history is intentionally granular; each commit should build and test.

### Documentation

- Behaviour changes update the subsystem page and, if relevant, the PRD.
- Performance claims update [claims & evidence](/reference/claims-and-evidence).
- Decisions get an ADR. Do not edit accepted ADRs; add a new one.
- The marketing site must not introduce a number that is not in the ledger.

## Adding a package

1. Create `packages/<name>/package.json` with `"exports": { ".": "./src/index.ts" }`.
2. Depend on workspace packages with `"workspace:*"`.
3. Add a `test` script (`bun test`) and tests next to the source.
4. Add the alias to the root `tsconfig.json` if other packages import it.
5. Run `bun install` to link it.
6. Add it to the repo map in `README.md` and this page.

## Design system changes

- Do not edit `packages/design-system` for Aether-only needs. Put additions in
  `packages/ui/src/ui.css` or the app.
- If the upstream reference must change, apply it in both repositories and note it in
  [design system](/design/design-system).
- New colours need tokens; new motion needs reduced-motion fallbacks.

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `Module "node:fs" externalized` | Importing the Node fs entry in the browser | Import from `@aether/indexer`, not `@aether/indexer/fs` |
| Docusaurus 404 on `/` | Docs are served under `/docs/` | Open `http://localhost:4322/docs/` |
| Playwright browser missing | Not installed | `bunx playwright install chromium` |
| React duplicate copies | Mixed versions | `bun install`, check `bun pm ls react` |
| Type errors in docs | Docusaurus has its own TS setup | Run `bun run --cwd apps/docs build` |
| Benchmark fails after a laptop sleep | Thermal/clock change | Re-run; if persistent, investigate |

## Working on the OS image

Requires Linux and Nix:

```bash
nix develop ./os/nixos
nix build .#iso
nixos-rebuild switch --flake .#kestrel
```

Never test an unbootable change on the only machine. Keep a known-good generation
and a live USB.
