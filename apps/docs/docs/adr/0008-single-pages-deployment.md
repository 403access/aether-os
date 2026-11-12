---
id: 0008-single-pages-deployment
title: ADR-0008 — One Pages artifact for site and docs
sidebar_position: 9
---

# ADR-0008 — One Pages artifact for site and docs

**Status:** Accepted · **Date:** 2026-11-12

## Context

The project has two static surfaces: a marketing site at `/` and documentation at
`/docs/`. GitHub Pages serves one site per repository (without custom subdomains).
Two deployments would require two repositories or a proxy.

## Decision

Build both, assemble them into one artifact (`apps/web/dist`) with docs under
`/docs/` and the design-system showcase under `/design-system/`, and deploy once on
`main` via GitHub Actions.

## Consequences

- One URL, one workflow, one rollback.
- Docs and site share a release number.
- `BASE_PATH` and `DOCS_BASE_URL` must agree; the assemble script checks the output.
- A docs-only fix redeploys the marketing site too (cheap, static).
- Local development still runs two dev servers on different ports.

## Alternatives

- **Two repositories** — clean separation, more release coordination.
- **Docs as a submodule** — worse contributor experience.
- **Custom domain with path routing** — still one Pages site; no benefit.
- **Netlify/Vercel** — extra service and account; Pages is sufficient.
