---
id: personas-and-jobs
title: Personas and jobs
sidebar_position: 2
description: The single user Aether is designed for, and the jobs the system must do well.
---

# Personas and jobs

Aether has exactly one persona because it has exactly one user. This is not a
limitation to work around; it is the design constraint that makes the rest
possible.

## Primary persona: the operator-developer

**Profile.** A senior engineer who owns the machine, runs local models or talks to
a hosted DeepSeek endpoint, and spends most of the day in a terminal and a browser.
They compile things, read documentation, review diffs, watch media, and delegate
work to coding agents.

**Environment.** One desk, one display (or two), wired audio plus Bluetooth
headphones, a mechanical keyboard, an NVMe drive, 32–64 GB RAM.

**Attitudes.**

- Distrusts telemetry and background daemons.
- Prefers keyboard over pointer; notices 20 ms of input lag.
- Accepts a broken system they can fix over a polished system they cannot inspect.
- Wants isolation that is observable, not a green checkmark.

## Jobs to be done

| Job | Desired outcome | Aether's answer |
| --- | --- | --- |
| Find anything | Type a few characters, hit enter, be there | One palette over apps, files, content, commands, agents |
| Delegate work | Hand a task to an agent, keep working | Isolated workers with explicit capability grants |
| Stay in flow | No spinner, no frame drop, no context switch | Compositor layers + in-memory index |
| Compare filesystems | See local vs archive state at a glance | Two-pane explorer with semantic status |
| Watch or listen | Fullscreen video, switch to headphones | libmpv + PipeWire routing |
| Read the web | Open docs, no browser bloat | WebKitGTK shell with per-tab processes |
| Trust the machine | Know what an agent can touch | Capability table + hash-chained audit |
| Recover fast | Undo a bad change | NixOS generations + immutable base |

## Anti-personas

- **The managed employee.** Needs MDM, SSO, compliance agents and remote wipe.
  Aether has no account system and no fleet control.
- **The distro hopper.** Wants to install Aether on five machines with different
  GPUs. Aether targets one machine; porting it is a fork, not a configuration.
- **The GUI-first user.** Expects discoverable menus, wizards and accessibility
  integrations at the level of GNOME. Aether is keyboard-first and dense.

## A day in the workflow

**08:40 — boot.** The machine resumes in under a second of userspace work. The
shell is already showing the last workspace; the indexer reports a warm snapshot
and starts watching roots.

**08:45 — orient.** `⌘K`, type `/standup`, open the recording from yesterday.
Audio is on headphones; Wav playing in the media player as a small floating
surface.

**09:10 — build.** Terminal, `cargo build --release`. Palette shows the three
modified files from the index. Chat is docked to the right, streaming a summary of
the failing test.

**09:30 — delegate.** `>spawn`, choose “Refactor worker”. The broker plans a
Landlock process with read access to `fs:/work/aether`, a 512 MiB cap and a
two-minute timeout. The agent reads the index, proposes a patch, and stops. It
never sees the credentials directory.

**11:00 — compare.** Explorer: left pane `/home/oliver/projects`, right pane
`/mnt/archive`. Rows show Same / Partial / Missing. One file is moved; the status
updates from the watcher without a manual refresh.

**14:00 — stream.** Start a remote session for the laptop. VA-API encodes the
desktop; the browser client decodes with WebCodecs. Glass-to-glass stays under
40 ms on LAN.

**18:00 — shut down.** Audit log exports a chained JSON file. No data left the
machine.

## Implications for the build

- Every surface must be reachable from the keyboard; pointer is optional.
- The system must be inspectable: status is shown, not hidden behind a spinner.
- Agents must fail closed. A denied capability is a feature, not an error dialog.
- There is no onboarding funnel, no settings sync, no telemetry consent screen.
