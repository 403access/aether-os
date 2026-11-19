# Scene diff · source DOM vs Aether render chunk

Text coverage: **364/824** runs across 28 surfaces (44.2 %).

| Surface | Source runs | Aether runs | Matched | Coverage | Missing (first) |
| --- | ---: | ---: | ---: | ---: | --- |
| `about` | 9 | 11 | 5 | 56 % | `A desktop concept drawn from the Data Cloud Compare design system. Every surface, badge, and control here is a token or component from the gallery.`, `Open the design system`, `Try the real Data Cloud Compare`, `Built with plain CSS and JavaScript. No images, no frameworks.` |
| `explorer` | 57 | 30 | 13 | 23 % | `/mnt/tank`, `Local`, `Local index`, `Projects`, `_LikedSongs` |
| `network` | 22 | 18 | 15 | 68 % | `Known`, `Lighthouse`, `Known`, `Tidepool Guest`, `Nearby` |
| `battery` | 25 | 24 | 17 | 68 % | `On battery`, `Battery`, `Normal`, `Not charging`, `Data Cloud Compare` |
| `devices` | 21 | 11 | 7 | 33 % | `Let nearby devices find this Mac.`, `Tide Mouse`, `Connected`, `Battery`, `Disconnect` |
| `radio` | 3 | 5 | 3 | 100 % |  |
| `calendar` | 12 | 12 | 12 | 100 % |  |
| `taskmanager` | 49 | 47 | 4 | 8 % | `Live`, `Memory`, `Storage`, `Usage over time`, `CPU` |
| `settings-general` | 44 | 33 | 25 | 57 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-appearance` | 48 | 31 | 22 | 46 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-displays` | 42 | 22 | 17 | 41 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-desktop` | 46 | 28 | 19 | 41 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-commands` | 47 | 27 | 19 | 40 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-accessibility` | 46 | 26 | 19 | 41 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-network` | 45 | 23 | 21 | 47 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-bluetooth` | 39 | 22 | 18 | 46 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-sound` | 37 | 24 | 18 | 49 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-battery` | 42 | 23 | 19 | 45 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-notifications` | 39 | 29 | 24 | 62 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-security` | 47 | 24 | 17 | 36 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `settings-storage` | 38 | 23 | 18 | 47 % | `Demo device`, `OM`, `Oliver Molnar`, `Aether ID · Administrator`, `System` |
| `menubar` | 10 | 10 | 9 | 90 % | `Desktop` |
| `menu` | 10 | 10 | 5 | 50 % | `New Explorer window`, `New Data Cloud Compare window`, `Verify snapshot`, `Toggle dotfiles`, `Settings` |
| `tray-network` | 10 | 9 | 5 | 50 % | `Connected`, `Known`, `Lighthouse`, `Known`, `VPN` |
| `tray-devices` | 7 | 7 | 3 | 43 % | `Connected`, `Tide Mouse`, `Harbour Keyboard`, `Wireless` |
| `tray-battery` | 10 | 9 | 6 | 60 % | `On battery`, `Low power mode`, `Reduce background activity.`, `Health` |
| `tray-sound` | 10 | 6 | 3 | 30 % | `Now playing`, `Deep Current`, `Harbour Speakers`, `Analog · analog-stereo`, `Internal` |
| `tray-calendar` | 9 | 3 | 1 | 11 % | `Sun`, `Mon`, `Tue`, `Wed`, `Thu` |

Missing text is the actionable half: every entry is a source label, row
description or group heading that the Aether frame does not paint.
