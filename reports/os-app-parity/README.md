# Water OS app parity

Side-by-side screenshots of every surface in the Water OS concept and the
matching Aether compositor surface. **Every image is a single window, panel,
menu, popover or chrome element — never a full-desktop frame.**

| File | Contents |
| --- | --- |
| `index.html` | The report (open it in a browser) |
| `ROADMAP.md` | Checklist of all 13 demo apps, chrome and trays with status and priority |
| `source/` | Element screenshots of the local Water OS checkout |
| `aether/` | Crops of the Aether compositor captures |

## Regenerate

```bash
# 1. Aether capture set (Docker + Xvfb). Produces desktop, menu, calendar,
#    sandboxes, explorer, themes, audio, chat, remote, palette, shortcuts,
#    task-manager and client-window screenshots.
bun run screenshots:shell

# 2. Source element screenshots + Aether crops + the report.
bun run parity:apps
```

`parity:apps` serves `../jottacloud-comparison/design-system` on a loopback
port, so it compares against the current source checkout (including uncommitted
changes) rather than the deployed Pages build. Set `OS_SOURCE_REPO` to point at
a different checkout, or `OS_REFERENCE_ORIGIN` to capture a deployed URL.

The demo clock is pinned to **2026-11-18 16:47 UTC** (`AETHER_FIXED_UNIX` for the
compositor) so the menu bars are directly comparable.

## Aether crop geometry

All crops assume a 1280 × 800 output with the 38 px menu bar:

| Surface | Source screenshot | Rect |
| --- | --- | --- |
| Menu bar | `desktop.png` | `0,0 1280x38` |
| Dock | `desktop.png` | `420,730 439x62` |
| App menus | `menu.png` | `110,0 250x215` |
| Command overlay | `palette-default.png` | `300,190 680x420` |
| Explorer | `explorer.png` | `16,54 1248x610` |
| Water settings | `themes.png` | `310,56 660x688` |
| Calendar | `calendar.png` | `290,88 700x590` |
| Task Manager | `task-manager.png` | `0,38 1280x688` |
| Terminal | `client-window.png` | `16,54 619x730` |
| Keyboard shortcuts | `shortcuts.png` | `260,120 760x560` |
| Audio (Aether only) | `audio.png` | `310,120 660x560` |
| Chat (Aether only) | `chat.png` | `864,54 400x660` |
| Agent sandboxes (Aether only) | `sandboxes.png` | `0,38 1280x688` |
| Remote (Aether only) | `remote.png` | `330,190 620x420` |
