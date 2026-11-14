---
id: testing
title: Testing strategy
sidebar_position: 2
description: The test pyramid, reusable fixtures, how to add tests and how to keep them fast.
---

# Testing strategy

Tests are the only mechanism that keeps a single-target OS honest. They are
deliberately layered so a failure points at one boundary.

## Pyramid

| Layer | Tool | Scope | Count at v0.1 |
| --- | --- | --- | --- |
| Unit / DOM / integration | `bun test` | Logic, ranking, capabilities, planners, components, cross-thread ring, filesystem | **206 total** (17 DOM, ~15 integration) |
| Architecture boundary | `bun test` | VM ↔ reference imports, security posture, flake modules | 7 (included above) |
| E2E | Playwright | Assembled marketing site, docs, install pages, VM page | **14** |
| Rust | `cargo test` | Scene, layout, damage, pacing, shell state, palette/commands, desktop entries, filename index, picker, split/move/resize/snap/maximize/fullscreen, text, cursors, preview | **88** |
| Performance | `packages/bench` | Regressions vs baseline | 6 metrics |

Run everything:

```bash
bun run typecheck
bun run test:unit
bun run test:e2e
bun run os:test
bun run bench --check
./scripts/vm-smoke.sh     # optional: boots the aarch64 VM and waits for the marker
```

## Where tests live

Tests sit next to the code (`src/*.test.ts`), not in a mirrored tree. This keeps the
fixture and the subject close and makes deletion obvious.

| Area | File | Notable coverage |
| --- | --- | --- |
| Protocol | `validate.test.ts` | Optional fields, ranges, unknown kinds, mustParse |
| Tokenizer | `tokenize.test.ts` | Unicode, camelCase, paths, fold mapping |
| Index | `search-index.test.ts` | Ranking, AND semantics, boosts, removal, snapshots |
| Filesystem | `fs.test.ts` | Temp dir, ignores, binary sniff, abort, maxFiles |
| Ring | `ring.test.ts` | FIFO, full, wrap marker, zero-copy, attach validation |
| Ring threads | `integration.test.ts` | 5,000 messages through a Worker + SharedArrayBuffer |
| Capabilities | `capabilities.test.ts` | Expiry, delegation, cascade, resource matching |
| Isolation | `isolation.test.ts` | Escalation, blockers, PKU absence, warnings |
| Scheduler | `scheduler.test.ts` | Budgets, queueing, priority, head-of-line |
| Collaboration | `collaboration.test.ts` | Membership, revocation, expiry, backpressure, audit |
| Audit | `audit.test.ts` | Chain verification, tamper detection, JSON round-trip |
| Palette | `palette.test.ts` | Modes, ranking, dedupe, stale queries, keyboard |
| Adapter | `adapters.test.ts` | Index → palette mapping, filters, empty query |
| UI | `ui.test.tsx` | ARIA, click, keyboard, explorer, meters |
| Boundary | `tests/separation.test.ts` | No VM/reference imports; no VM posture in the reference |
| Rust | `os/compositor/**` | Damage, layout, shell policy, preview, pacing |

## Reusable fixtures

- `packages/indexer/src/fs.test.ts` creates a temp directory per test and removes it
  in `afterAll`. Copy this pattern instead of committing fixtures.
- `packages/ipc/src/fixtures/producer.worker.ts` is a reusable cross-thread producer.
- `apps/web/src/data.ts` is the demo corpus, shared by the site and e2e.
- `packages/agents/src/*.test.ts` use a `spec()` factory to keep agent specs terse.
- `packages/bench` exposes `runBenchmarks({ documents: 200, ... })` for fast tests.

Prefer builders over literal objects. When a field is added to a type, one factory
should need to change.

## Determinism

- Inject clocks: `now: () => 1_000` or `Date.UTC(...)`.
- Avoid `setTimeout` in logic tests; use deferred promises.
- Sort before asserting sets.
- Use `toEqual` on plain objects, not snapshots, unless the shape is the contract.
- Don't assert exact benchmark values; assert budgets and directions.

## Architectural boundary test

`tests/separation.test.ts` reads the Nix tree and the flake. It fails if the
physical target imports the VM, the VM imports the physical target, VM-only
security posture appears in `os/nixos`, or the `kestrel` flake block includes the
VM modules. It is part of the normal unit suite, so CI enforces the boundary
without a separate job.

## Cross-thread test

The ring test spawns a Bun Worker with a `SharedArrayBuffer`:

```ts
const producer = RingBuffer.create(8 * 1024);
const consumer = RingBuffer.attach(producer.buffer);
new Worker(new URL("./fixtures/producer.worker.ts", import.meta.url).href);
```

It asserts 5,000 ordered messages, no loss, and no capability drift. This test found
a real race: the consumer advanced `tail` before copying the payload, allowing the
producer to overwrite it. The fix is documented in `ring.ts`.

## DOM tests

`bunfig.toml` preloads `packages/ui/src/test-setup.ts`, which registers happy-dom and
sets `IS_REACT_ACT_ENVIRONMENT`.

Rules:

- Use roles and labels (`getByRole`, `getByLabelText`), not CSS selectors.
- Await async results with `findBy*`/`waitFor`; avoid arbitrary sleeps.
- Clean up after each test (`afterEach(cleanup)`).
- Test behaviour, not implementation: click, type, press keys.

## E2E tests

Playwright runs against the **assembled build**, not the dev server:

```bash
bun run build
bun run test:e2e
```

Specs cover:

- Marketing hero and navigation render.
- The palette filters results and executes an action.
- The explorer selects and navigates folders.
- Docs load and the sidebar navigates.
- No console errors on either surface.

E2E tests are intentionally few and coarse. Unit tests own logic.

## Performance tests

`bun run bench --check` fails on a >25% regression in latency (lower is better) or
throughput (higher is better). The baseline is committed and reviewed with each
release.

Guidelines:

- Never update the baseline to hide a regression; fix or write an ADR.
- New metrics must include a direction, a unit and a baseline entry.
- Guard-rail tests (e.g. 5k docs under 100 ms) live in unit tests and are deliberately
  loose; the bench is the precise gate.

## Adding a test

1. Find the boundary being changed (pure logic, DOM, thread, filesystem, process).
2. Add the smallest test that fails before the change.
3. Put it next to the code; name it after the behaviour, not the function.
4. Cover the failure path (denied, expired, blocked, empty).
5. Run the full suite; do not leave a flaky test.

## Flakiness policy

- A flaky test is a bug in the test or in the code; it is fixed, not skipped.
- Timeouts are symptoms. Bound the operation and assert on the bound.
- Cross-thread tests must have deadlines and clean up workers in `afterEach`.
- Benchmarks do not fail on absolute time; only relative regressions.

## What is not tested yet

- Real Landlock/seccomp enforcement (P1/P5).
- GPU paths and frame presentation (P2).
- WebKit and mpv integration (P6).
- WebRTC/WebCodecs latency (P7).

These are listed in the roadmap gates; until then, the docs say so.
