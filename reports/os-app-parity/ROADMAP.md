# Water OS app parity · roadmap

Source: `283b31c Add a sidebar toggle to the Explorer (+ local changes)` (local design-system/ checkout)

Aether: `8343d3e Merge pull request #2 from 403access/perf/ci-caching`

Implemented **5** · partial **6** · missing **11** · Aether-only **4**.

**Status key** — implemented: matches the source surface · partial: present but a different scope or layout · missing: no Aether surface · Aether only: no source counterpart.

| Surface | Group | Status | Priority | Where | Notes |
| --- | --- | --- | --- | --- | --- |
| Menu bar | System chrome | Implemented | P0 · done | `app/overlays/menubar/` | Brand, active app, five menus, sync pill, CPU/RAM/STORAGE meters, tray and clock. Geometry within 2-3 px; icon glyphs differ. |
| Dock | System chrome | Implemented | P0 · done | `app/overlays/dock.rs` | Same 46 px items, 38 px tinted tiles, separator and running dot. The demo dock has ten tiles (adding Projects and App Studio); Aether ships eight for its own apps. |
| App menus | System chrome | Implemented | P0 · done | `app/overlays/menubar/layout.rs` | File/View/Go/Window/Help open popovers that run real commands. The Aether crop includes the bar segment above the popover. |
| Command overlay | System chrome | Partial | P2 | `crates/aether-shell/src/shell/palette.rs` | Both are ⌘K surfaces. The demo uses the explorer's command palette (groups, filters, previews); Aether uses its own providers and row language. |
| Data Cloud Compare | Apps | Missing | — | product-specific | The product's two-pane cloud-vs-index comparator with match badges, legend and command bar. Aether has no cloud archive model. |
| Explorer | Apps | Partial | P2 | `render/surfaces/explorer.rs` | Window geometry, title bar, app bar, pane card, rows and status strip match. The demo explorer is a single-pane file browser with a breadcrumb and search row; Aether's is a single indexed pane. |
| Settings | Apps | Partial | P2 | `render/surfaces/themes.rs` | The demo Settings app is tabbed: General, Appearance, Displays, Desktop & Dock, Search, Widgets, Accessibility, Network, Bluetooth, Sound, Battery, Privacy, Screen Time and Storage. Aether only ships the Water settings panel (Themes). |
| Network | Apps | Missing | P1 | — | Wi-Fi, VPN and known networks. The menu bar Wi-Fi tray is a stub that logs. |
| Battery | Apps | Missing | P1 | — | Charge, health, energy mode and history. The menu bar battery item is a stub. |
| Devices | Apps | Missing | P1 | — | Bluetooth accessories, pairing and battery levels. The tray item is a stub. |
| Calendar | Apps | Implemented | P0 · done | `app/overlays/calendar.rs` | Six-row month grid, today fill, selected rim, event dots and agenda column. Keyboard navigation implemented. |
| Task Manager | Apps | Partial | P1 | `app/overlays/task_manager.rs` | Live /proc sampling, tabs and a sorted table in both. Aether runs it as a full-width panel below the menu bar; the demo uses a floating window with its own title bar. |
| Terminal | Apps | Implemented | P0 · done | `render/window_frame.rs` | Aether hosts real Wayland clients (foot) with the compositor's Water title bar; the demo renders a mock terminal inside its own window. |
| Tide Radio | Apps | Missing | P3 | — | Playback, queue and presets. Aether has an audio-route overlay but no player. |
| Keyboard shortcuts | Apps | Partial | P2 | `app/overlays/shortcuts.rs` | Both are Super+/ cheatsheets. The demo groups the explorer key map; Aether lists the compositor's own surfaces and window actions. |
| About Aether OS | Apps | Missing | P2 | — | Version, build, hardware and licence. The menu-bar logo currently opens the cheatsheet. |
| App Studio | Apps | Missing | — | product-specific | The demo's GUI builder for app surfaces. No Aether equivalent. |
| Wi-Fi tray | System trays | Missing | P1 | — | Connected network, known networks, VPN state, link to Network. |
| Bluetooth tray | System trays | Missing | P1 | — | Accessory list and toggle, link to Devices. |
| Battery tray | System trays | Missing | P1 | — | Charge figure, energy mode and time remaining. |
| Sound tray | System trays | Missing | P3 | — | Output selection and volume. Aether's audio overlay covers the routing UI but not this tray. |
| Calendar tray | System trays | Partial | P2 | — | The demo opens a month popover from the clock and offers Open Calendar. Aether opens the Calendar window directly. |
| Audio routes | Aether-only | Aether only | P0 · done | `render/surfaces/audio.rs` | 660 x 560 overlay for outputs, inputs, presets and the input meter. |
| Chat drawer | Aether-only | Aether only | P0 · done | `render/surfaces/chat.rs` | Streaming markdown with MCP tool traces. |
| Agent sandboxes | Aether-only | Aether only | P0 · done | `render/surfaces/sandboxes.rs` | Isolation tiers and per-agent resource use. |
| Remote display | Aether-only | Aether only | P0 · done | `render/surfaces/remote.rs` | Pairing code, WebRTC transport and the recording indicator. |

## Next up

1. **P1 · Network, Battery, Devices** windows and their tray popovers (menu-bar items are stubs today).
2. **P1 · Task Manager as a floating window** instead of a full-width panel.
3. **P2 · Explorer parity**: sidebar/favourites, breadcrumb, in-pane search row and size/modified columns.
4. **P2 · Settings tabs** beyond Water (Appearance, Displays, Desktop & Dock, Search, Accessibility).
5. **P2 · About Aether OS** window behind the menu-bar logo.
6. **P2 · Calendar and Sound trays** (month popover from the clock, output/volume popover).
7. **P3 · Tide Radio** player and the sound tray that opens it.
8. **Not planned**: Data Cloud Compare and App Studio are product-specific to the demo.

## Regenerate

```bash
bun run screenshots:shell   # Aether capture set (Docker Xvfb)
bun run parity:apps         # source element shots + Aether crops + this report
```

`parity:apps` serves `../jottacloud-comparison/design-system` on a loopback port,
so it always compares against the current source checkout rather than the
deployed Pages build. Set `OS_SOURCE_REPO` to point at another checkout.
