# Water OS app parity · roadmap

Source: `e86b239 Add drag-and-drop tile swapping to split layout` (local design-system/ checkout)
Visual spec (source vs target, difference register): [`VISUAL-SPEC.md`](VISUAL-SPEC.md).
Parity target and remaining work: [`PARITY-100.md`](PARITY-100.md).

Aether: `4feb824 fix(capture): poll the palette log instead of fixed sleeps`

Implemented **17** · partial **15** · missing **0** · out of scope **2** · Aether-only **4**.

**Status key** — implemented: matches the source surface · partial: present but a different scope or layout · missing: no Aether surface · out of scope: intentionally not an Aether app · Aether only: no source counterpart.

| Surface | Group | Status | Priority | Where | Notes |
| --- | --- | --- | --- | --- | --- |
| Menu bar | System chrome | Implemented | P0 · done | `app/overlays/menubar/` | Brand, active app, five menus, sync pill, CPU/RAM/STORAGE meters, tray and clock. Geometry within 2-3 px; icon glyphs differ. |
| Dock | System chrome | Implemented | P0 · done | `app/overlays/dock.rs` | Same 46 px items, 38 px tinted tiles, separator and running dot. The demo hides its dock below 1360 px, so the source capture is at 1440; Aether keeps it at 1280. The demo has ten tiles (adding Projects and App Studio); Aether ships eight. |
| App menus | System chrome | Implemented | P0 · done | `app/overlays/menubar/layout.rs` | File/View/Go/Window/Help open popovers that run real commands. The Aether crop includes the bar segment above the popover. |
| Command overlay | System chrome | Partial | P2 | `crates/aether-shell/src/shell/palette.rs` | Both are ⌘K surfaces. The demo uses the explorer's command palette (groups, filters, previews); Aether uses its own providers and row language. |
| Data Cloud Compare | Apps | Out of scope | — | product-specific | Ignored by decision: the Data Cloud Compare product surface (cloud-vs-index comparator) is not an Aether app. |
| Explorer | Apps | Implemented | P1 | `render/surfaces/explorer.rs` | Sidebar (favourites, locations, read-only notice), toolbar with breadcrumb and hidden switch, in-pane search row, Name/Type/Size/Modified columns, footer and status strip. Hidden/Everywhere/Filter are still visual-only. |
| Settings · General | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Host, software update and language/region sections. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Appearance | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Theme and Water effect switches drive the shell live. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Displays | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Output name, resolution, refresh rate and scale. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Desktop & Dock | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Working Show dock switch (off by default, like the demo), running indicators, clock and meters. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Search & Commands | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Palette shortcut, prefixes and the search provider. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Accessibility | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Reduce motion, text size, focus ring and contrast. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Network | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Interface, MAC, IP, router and DNS from the host. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Bluetooth | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Adapter, discoverable switch and paired devices. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Sound | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Output, presets and microphone input. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Battery | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Charge, condition and energy mode. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Notifications | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Allow/preview/Do Not Disturb plus per-app rows. Aether now ships the demo's 13 panes in the demo order. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Privacy & Security | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Agent isolation, telemetry and the audit log. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Settings · Storage | Apps | Partial | P1 | `app/system.rs` + `render/surfaces/system_app.rs` | Root filesystem, index and free space. Remaining gap: per-row descriptions. The sidebar profile card, nav groups, pane header and footer are drawn. |
| Network | Apps | Implemented | P0 · done | `app/system.rs` + `app/system_info.rs` | Interfaces from /sys/class/net, IP from fib_trie, router from /proc/net/route and DNS from resolv.conf, in the demo's header/switch/list/details card layout. |
| Battery | Apps | Implemented | P0 · done | `app/system.rs` + `app/system_info.rs` | Charge and estimate, energy mode, low-power and optimised-charging switches, health table and largest CPU consumers. Shows an AC-power state when no battery exists. |
| Devices | Apps | Implemented | P0 · done | `app/system.rs` + `app/system_info.rs` | Bluetooth summary, accessory switches and paired-device list over /sys/class/bluetooth, with an explicit no-adapter state. |
| Calendar | Apps | Implemented | P0 · done | `app/overlays/calendar.rs` | Six-row month grid, today fill, selected rim, event dots and agenda column. Keyboard navigation implemented. |
| Task Manager | Apps | Implemented | P1 | `app/overlays/task_manager.rs` | Live /proc sampling, tabs, a sorted table, task-count badge and status strip in the demo's floating window box. |
| Terminal | Apps | Implemented | P0 · done | `render/window_frame.rs` | Aether hosts real Wayland clients (foot) with the compositor's Water title bar; the demo renders a mock terminal inside its own window. |
| Tide Radio | Apps | Implemented | P0 · done | `app/system.rs` | Player window with artwork, track metadata, progress, transport controls and a five-track queue; Space toggles playback. |
| Keyboard shortcuts | Apps | Partial | P2 | `app/overlays/shortcuts.rs` | Both are Super+/ cheatsheets. The demo groups the explorer key map; Aether lists the compositor's own surfaces and window actions. |
| About Aether OS | Apps | Implemented | P0 · done | `app/system.rs` + `render/surfaces/system_app.rs` | Version, build, hardware and licence. The menu-bar logo currently opens the cheatsheet. |
| App Studio | Apps | Out of scope | — | product-specific | Ignored by decision: the demo's GUI builder is a product surface, not part of the Aether desktop. |
| Wi-Fi tray | System trays | Implemented | P0 · done | `render/surfaces/tray.rs` | Wi-Fi switch, connected interface, network list and a Network Settings link. Panel size matches the demo (300 x 328). |
| Bluetooth tray | System trays | Implemented | P0 · done | `render/surfaces/tray.rs` | Bluetooth switch, paired-device area and a Bluetooth Settings link (300 x 253). |
| Battery tray | System trays | Implemented | P0 · done | `render/surfaces/tray.rs` | Charge, estimate, level bar, three-way energy mode and a Battery Settings link (286 x 270). |
| Sound tray | System trays | Implemented | P0 · done | `render/surfaces/tray.rs` | Output name, volume bar and a Sound Settings link (292 x 256). Preset switching stays in the audio overlay. |
| Calendar tray | System trays | Implemented | P0 · done | `render/surfaces/tray.rs` | Month popover from the clock with today filled and an Open Calendar link (308 x 351). |
| Audio routes | Aether-only | Aether only | P0 · done | `render/surfaces/audio.rs` | 660 x 560 overlay for outputs, inputs, presets and the input meter. |
| Chat drawer | Aether-only | Aether only | P0 · done | `render/surfaces/chat.rs` | Streaming markdown with MCP tool traces. |
| Agent sandboxes | Aether-only | Aether only | P0 · done | `render/surfaces/sandboxes.rs` | Isolation tiers and per-agent resource use. |
| Remote display | Aether-only | Aether only | P0 · done | `render/surfaces/remote.rs` | Pairing code, WebRTC transport and the recording indicator. |

## Next up

Data Cloud Compare and App Studio are **out of scope by decision** and excluded from the gap.

1. **Explorer wiring** (P1): make Hidden, Everywhere, Filter and the navigation buttons functional.
2. **Command overlay** (P2): align the palette with the demo's grouped command surface.
3. **Keyboard shortcuts** (P2): group the cheatsheet by app like the demo.

## Regenerate

```bash
bun run screenshots:shell   # Aether capture set (Docker Xvfb)
bun run parity:apps         # source element shots + Aether crops + this report
```

`parity:apps` serves `../jottacloud-comparison/design-system` on a loopback port,
so it always compares against the current source checkout rather than the
deployed Pages build. Set `OS_SOURCE_REPO` to point at another checkout.
