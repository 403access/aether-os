# Scene diff · source DOM vs Aether render chunk

Text coverage: **434/842** runs across 29 surfaces (51.5 %). Geometry: **45.4 %** of matched boxes within tolerance.

| Surface | Source runs | Aether runs | Matched | Coverage | Aligned | Missing (first) |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `about` | 9 | 11 | 5 | 56 % | 0 % | `A desktop concept drawn from the Data Cloud Compare design system. Every surface, badge, and control here is a token or component from the gallery.`, `Open the design system`, `Try the real Data Cloud Compare`, `Built with plain CSS and JavaScript. No images, no frameworks.` |
| `explorer` | 57 | 30 | 8 | 14 % | 13 % | `/mnt/tank`, `Local`, `Local index`, `Projects`, `_LikedSongs` |
| `network` | 22 | 18 | 8 | 36 % | 25 % | `Known`, `Lighthouse`, `Known`, `Tidepool Guest`, `Nearby` |
| `battery` | 25 | 24 | 17 | 68 % | 12 % | `On battery`, `Battery`, `Normal`, `Not charging`, `Data Cloud Compare` |
| `devices` | 21 | 11 | 7 | 33 % | 29 % | `Let nearby devices find this Mac.`, `Tide Mouse`, `Connected`, `Battery`, `Disconnect` |
| `radio` | 3 | 5 | 3 | 100 % | 33 % |  |
| `calendar` | 12 | 12 | 12 | 100 % | 58 % |  |
| `taskmanager` | 49 | 47 | 2 | 4 % | 100 % | `Live`, `CPU`, `Memory`, `Storage`, `Usage over time` |
| `settings-general` | 44 | 46 | 37 | 84 % | 41 % | `Demo device`, `Chip`, `Serial number`, `Demo device`, `About This Mac` |
| `settings-appearance` | 48 | 42 | 27 | 56 % | 44 % | `Appearance`, `Demo device`, `System`, `Light`, `Dark` |
| `settings-displays` | 42 | 30 | 23 | 55 % | 52 % | `Displays`, `Demo device`, `Aether Display`, `Brightness`, `Adjust the built-in display.` |
| `settings-desktop` | 46 | 38 | 25 | 54 % | 48 % | `Desktop & Dock`, `Demo device`, `Windows`, `Window layout`, `Split shares the desktop between open apps; drag a title bar onto another tile to swap places. Overlap keeps their own positions and sizes.` |
| `settings-commands` | 47 | 35 | 24 | 51 % | 54 % | `Search & Commands`, `Demo device`, `Show commands`, `Navigation, view, appearance, and system actions.`, `Search local files` |
| `settings-accessibility` | 46 | 35 | 25 | 54 % | 48 % | `Accessibility`, `Demo device`, `Scale the desktop and every app.`, `Increase contrast`, `Strengthen borders and secondary text.` |
| `settings-network` | 45 | 31 | 23 | 51 % | 52 % | `Network`, `Demo device`, `Connection`, `Connected`, `Wi-Fi` |
| `settings-bluetooth` | 39 | 30 | 25 | 64 % | 52 % | `Bluetooth`, `Demo device`, `Bluetooth`, `Let nearby devices find this Mac.`, `My devices` |
| `settings-sound` | 37 | 32 | 24 | 65 % | 50 % | `Sound`, `Demo device`, `Output device`, `Where sound should play.`, `Volume` |
| `settings-battery` | 42 | 31 | 24 | 57 % | 50 % | `Battery`, `Demo device`, `Energy`, `Low power`, `Balanced` |
| `settings-notifications` | 39 | 43 | 37 | 95 % | 70 % | `Notifications`, `Demo device` |
| `settings-security` | 47 | 32 | 23 | 49 % | 52 % | `Privacy & Security`, `Demo device`, `Security`, `FileVault`, `Encrypt the startup disk. Recovery key is stored in Aether ID.` |
| `settings-storage` | 38 | 31 | 24 | 63 % | 50 % | `Storage`, `Demo device`, `Macintosh HD`, `Archive snapshots`, `Applications` |
| `command` | 18 | 0 | 0 | 0 % | — | `Focus local pane`, `Move keyboard focus to the local index`, `Focus Jottacloud pane`, `Move keyboard focus to the remote snapshot`, `Go to local root` |
| `menubar` | 10 | 10 | 10 | 100 % | 10 % |  |
| `menu` | 10 | 10 | 5 | 50 % | 0 % | `New Explorer window`, `New Data Cloud Compare window`, `Verify snapshot`, `Toggle dotfiles`, `Settings` |
| `tray-network` | 10 | 9 | 4 | 40 % | 25 % | `Connected`, `Known`, `Lighthouse`, `Known`, `VPN` |
| `tray-devices` | 7 | 7 | 3 | 43 % | 33 % | `Connected`, `Tide Mouse`, `Harbour Keyboard`, `Wireless` |
| `tray-battery` | 10 | 9 | 6 | 60 % | 17 % | `On battery`, `Low power mode`, `Reduce background activity.`, `Health` |
| `tray-sound` | 10 | 6 | 2 | 20 % | 50 % | `Harbour Speakers`, `Now playing`, `Deep Current`, `Analog · analog-stereo`, `Output` |
| `tray-calendar` | 9 | 3 | 1 | 11 % | 0 % | `Sun`, `Mon`, `Tue`, `Wed`, `Thu` |

Missing text is the actionable half: every entry is a source label, row
description or group heading that the Aether frame does not paint.
