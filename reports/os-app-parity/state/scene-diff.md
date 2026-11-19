# Scene diff · source DOM vs Aether render chunk

Text coverage: **455/824** runs across 28 surfaces (55.2 %).

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
| `settings-general` | 44 | 41 | 32 | 73 % | `Demo device`, `Concept build · Water design system`, `Chip`, `Unified memory`, `Serial number` |
| `settings-appearance` | 48 | 39 | 29 | 60 % | `Demo device`, `System`, `Light`, `Dark`, `Water effects are active. Drips follow your cursor.` |
| `settings-displays` | 42 | 30 | 24 | 57 % | `Demo device`, `Displays`, `Aether Display`, `Brightness`, `Adjust the built-in display.` |
| `settings-desktop` | 46 | 36 | 26 | 56 % | `Demo device`, `Windows`, `Window layout`, `Split shares the desktop between open apps; drag a title bar onto another tile to swap places. Overlap keeps their own positions and sizes.`, `Stage Manager` |
| `settings-commands` | 47 | 35 | 26 | 55 % | `Demo device`, `Search & Commands`, `Show commands`, `Navigation, view, appearance, and system actions.`, `Search local files` |
| `settings-accessibility` | 46 | 34 | 26 | 56 % | `Demo device`, `Accessibility`, `Scale the desktop and every app.`, `Increase contrast`, `Strengthen borders and secondary text.` |
| `settings-network` | 45 | 31 | 28 | 62 % | `Demo device`, `Connection`, `Connected`, `Wi-Fi`, `Discover and join nearby networks.` |
| `settings-bluetooth` | 39 | 30 | 25 | 64 % | `Demo device`, `Bluetooth`, `Bluetooth`, `Let nearby devices find this Mac.`, `My devices` |
| `settings-sound` | 37 | 32 | 25 | 68 % | `Demo device`, `Output device`, `Where sound should play.`, `Volume`, `Currently using .` |
| `settings-battery` | 42 | 31 | 26 | 62 % | `Demo device`, `Battery`, `Energy`, `Low power`, `High performance` |
| `settings-notifications` | 39 | 37 | 31 | 80 % | `Demo device`, `Allow notifications`, `Show banners and alerts from apps.`, `What appears before you unlock the screen.`, `Silence all notifications until turned off.` |
| `settings-security` | 47 | 32 | 24 | 51 % | `Demo device`, `Security`, `FileVault`, `Encrypt the startup disk. Recovery key is stored in Aether ID.`, `Firewall` |
| `settings-storage` | 38 | 31 | 25 | 66 % | `Demo device`, `Macintosh HD`, `Archive snapshots`, `Applications`, `Media` |
| `menubar` | 10 | 10 | 9 | 90 % | `Desktop` |
| `menu` | 10 | 10 | 5 | 50 % | `New Explorer window`, `New Data Cloud Compare window`, `Verify snapshot`, `Toggle dotfiles`, `Settings` |
| `tray-network` | 10 | 9 | 5 | 50 % | `Connected`, `Known`, `Lighthouse`, `Known`, `VPN` |
| `tray-devices` | 7 | 7 | 3 | 43 % | `Connected`, `Tide Mouse`, `Harbour Keyboard`, `Wireless` |
| `tray-battery` | 10 | 9 | 6 | 60 % | `On battery`, `Low power mode`, `Reduce background activity.`, `Health` |
| `tray-sound` | 10 | 6 | 3 | 30 % | `Now playing`, `Deep Current`, `Harbour Speakers`, `Analog · analog-stereo`, `Internal` |
| `tray-calendar` | 9 | 3 | 1 | 11 % | `Sun`, `Mon`, `Tue`, `Wed`, `Thu` |

Missing text is the actionable half: every entry is a source label, row
description or group heading that the Aether frame does not paint.
