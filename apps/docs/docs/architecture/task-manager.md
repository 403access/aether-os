---
id: task-manager
title: Task manager
sidebar_position: 9
description: "The live system monitor: process tree, users, CPU, memory, network, disk and refresh behavior."
---

# Task manager

The task manager is a full-output shell surface opened from the command palette:

```text
Super+Space → type "task" → Enter
Ctrl+Alt+Space → type "task" → Enter   # Simple/VNC profile
```

![Task manager processes tab](/img/task-manager.png)

## Data

`aether-system` samples `/proc` once per second while the surface is open:

| Source | Used for |
| --- | --- |
| `/proc/[pid]/status` | name, state, UID, threads, RSS |
| `/proc/[pid]/stat` | parent PID and CPU ticks |
| `/proc/[pid]/cmdline` | command line |
| `/etc/passwd` | user names |
| `/proc/stat` | total and per-core CPU |
| `/proc/meminfo` | memory and swap |
| `/proc/net/dev` | interface counters and rates |
| `/proc/diskstats` | device read/write counters and rates |
| `/proc/loadavg`, `/proc/uptime` | load and uptime |

CPU, network and disk values are delta-based, so the first sample establishes a
baseline and later samples show rates.

## Tabs

### Processes

- Depth-first process tree with `▾` expanded and `▸` collapsed parents.
- PID, user, CPU%, memory, state and command columns.
- A selected row updates the footer.
- The scrollbar tracks the visible window.

### Users

![Users tab](/img/task-manager-users.png)

Aggregated by UID: process count, thread count, total CPU% and resident memory.
Rows are sorted by CPU.

### Resources

![Resources tab](/img/task-manager-resources.png)

- CPU total and per-core bars.
- Memory usage and swap.
- Interface receive/transmit rates.
- Device read/write rates, load and uptime.

## Keyboard

| Input | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Next / previous tab |
| `Up` / `Down` | Move process selection |
| `PgUp` / `PgDn` | Page the process list |
| `Space` | Collapse/expand selected parent |
| `Left` / `Right` | Collapse / expand |
| `R` | Force a fresh sample |
| `Esc` | Close |

Click a row to select it, or click the top-right close control.

## Scope

- Linux only (`/proc`); the crate returns an empty snapshot on other hosts.
- It does not kill processes; `Super+Q` remains the close-window action.
- Network and disk rates are per interface/device, not per process.
- Scroll is clamped to the same visible window used by the renderer.
