---
id: 0006-web-stack-bun-typescript-react
title: ADR-0006 — Bun, TypeScript and React for web surfaces
sidebar_position: 7
---

# ADR-0006 — Bun, TypeScript and React for web surfaces

**Status:** Accepted · **Date:** 2026-11-12

## Context

The project needs a marketing site, documentation, a live palette demo, a component
kit and shared domain types. The OS itself is Rust/Nix, but the web surfaces must be
developed and tested quickly, and they must not smuggle a runtime into the shell.

## Decision

Use Bun workspaces with strict TypeScript. React 19 for components, Vite for the
marketing site, Docusaurus for docs, and a headless TypeScript palette engine shared
between the demo and the UI kit. No framework in the compositor.

## Consequences

- One lockfile, one test runner, fast installs.
- Domain types are shared: the web demo exercises the same ranking code the shell
  will use.
- Bun is young; CI pins the version and the lockfile.
- The docs app is heavier than the marketing site; it is built separately and
  assembled into one artifact.
- React is used only in browser surfaces, never in the compositor.

## Alternatives

- **Node + pnpm + Next.js** — mature, but heavier and more configuration for a
  static site.
- **Rust/WASM UI** — interesting, but slows iteration on docs and marketing and
  duplicates the compositor's work.
- **Astro** — excellent for static content, but the live palette needs client React
  anyway; Vite + React is simpler.
- **Svelte** — smaller runtime, but the component kit and docs ecosystem are React.
