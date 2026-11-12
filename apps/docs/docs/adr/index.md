---
id: index
title: Decision records
sidebar_position: 1
description: Architecture decisions, their context and consequences.
---

# Decision records

One decision per record. Format: context → decision → consequences → alternatives.
A decision that is reversed gets a new ADR; records are not rewritten.

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](/adr/0001-nixos-base) | NixOS as the base | Accepted |
| [0002](/adr/0002-rust-smithay-compositor) | Rust + Smithay compositor | Accepted |
| [0003](/adr/0003-xfs-root) | XFS root, ext4 scratch | Accepted |
| [0004](/adr/0004-isolation-without-pku) | Process/microVM isolation, no PKU | Accepted |
| [0005](/adr/0005-shared-memory-ipc) | Shared-memory SPSC IPC | Accepted |
| [0006](/adr/0006-web-stack-bun-typescript-react) | Bun + TypeScript + React for web surfaces | Accepted |
| [0007](/adr/0007-remote-display-webrtc-webcodecs) | WebRTC + WebCodecs remote display | Proposed (P7) |
| [0008](/adr/0008-single-pages-deployment) | One Pages artifact for site + docs | Accepted |
| [0009](/adr/0009-water-design-system) | Adopt the Water design system and reference explorer | Accepted |
| [0010](/adr/0010-compositor-crate-split) | Split the compositor core into reusable crates | Accepted |
| [0011](/adr/0011-backend-neutral-shell) | Backend-neutral shell state and preview | Accepted |
| [0012](/adr/0012-smithay-standalone-backend) | Smithay backend as a standalone Linux crate | Accepted |
| [0013](/adr/0013-bitmap-text) | Bitmap text for compositor chrome | Accepted |

## When to write one

Write an ADR when a change:

- adds or removes a runtime dependency,
- changes a trust boundary or capability,
- changes a performance budget or gate,
- selects a storage, transport or rendering technology,
- narrows product scope, or
- reverses an earlier decision.

Small implementation choices belong in code comments or the relevant subsystem
page, not here.
