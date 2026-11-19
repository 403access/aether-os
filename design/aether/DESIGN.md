---
version: alpha
name: Aether OS
description: Chrome contract generated from the compositor's design manifest.
colors:
  bg: "#031922"
  surface: "#072733fa"
  popover: "#0d3d4b"
  menubar-fill: "#052530b8"
  text: "#e9fbff"
  text-2: "#a5dbe5"
  text-3: "#6ba9b6"
  accent: "#38d6ff"
  line: "#7fe6ff26"
  line-strong: "#7fe6ff4c"
  success: "#5ef2b0"
  primary: "#38d6ff"
typography:
  menubar-app:
    fontFamily: "Inter"
    fontSize: "12.54px"
    fontWeight: 700
    letterSpacing: "-0.13px"
  menubar-menu:
    fontFamily: "Inter"
    fontSize: "12.54px"
    fontWeight: 550
  menubar-sync:
    fontFamily: "Inter"
    fontSize: "11.4px"
    fontWeight: 400
  clock:
    fontFamily: "Inter"
    fontSize: "11.4px"
    fontWeight: 400
  clock-time:
    fontFamily: "Inter"
    fontSize: "11.4px"
    fontWeight: 400
  resource-label:
    fontFamily: "Inter"
    fontSize: "7.98px"
    fontWeight: 400
    letterSpacing: "0.32px"
  resource-value:
    fontFamily: "Inter"
    fontSize: "9.69px"
    fontWeight: 550
  window-title:
    fontFamily: "Inter"
    fontSize: "12.54px"
    fontWeight: 400
    letterSpacing: "-0.13px"
  window-subtitle:
    fontFamily: "Inter"
    fontSize: "12.54px"
    fontWeight: 400
  menu-row:
    fontFamily: "Inter"
    fontSize: "13.68px"
    fontWeight: 400
  label-caps:
    fontFamily: "Inter"
    fontSize: "9.12px"
    fontWeight: 650
    letterSpacing: "0.82px"
rounded:
  xs: "5px"
  md: "10px"
  lg: "14px"
  dock: "17px"
  pill: "999px"
spacing:
  menu: "5px"
  dock: "8px"
  bar: "10px"
---

## Overview

Aether's implementation of the same desktop. Tokens come from the compositor's design manifest, so this file describes what the renderer actually paints. The source document is the contract; differences are reviewed as token paths, not prose.

## Colors

Surfaces, text roles, accent and hairlines measured from the running interface.

## Typography

Fractional OS sizes (the demo scales design tokens by 1.14) with the menu-bar, window and popover roles.

## Layout

Menu bar and window chrome share a 26 px item rhythm; the dock sits 8 px above the bottom edge.

## Elevation & Depth

Windows use a layered drop shadow and an inset top highlight; the focused window adds the tide rim.

## Shapes

Radii run from 5 px controls to 14 px windows and 17 px dock.

## Motion & Micro-interactions

The tide rim cycles every 8 seconds (linear, no easing). Interface transitions stay under 200 ms and ease out.

## Components

Component values are the contract; variants are separate entries when they exist. Missing components in the Aether document are implementation gaps.

```yaml
components:
  menubar:
    backgroundColor: "{colors.menubar-fill}"
    height: "38px"
    padding: "10px"
  menubar-app:
    typography: "{typography.menubar-app}"
    height: "26px"
    width: "160px"
  menubar-menu:
    typography: "{typography.menubar-menu}"
    textColor: "{colors.text-2}"
    height: "26px"
    padding: "9px"
    rounded: "{rounded.xs}"
  sync-pill:
    typography: "{typography.menubar-sync}"
    height: "26px"
    rounded: "{rounded.xs}"
  resources:
    height: "26px"
    padding: "8px"
  resource-meter:
    backgroundColor: "{colors.accent}"
    width: "20px"
    height: "3px"
    rounded: "{rounded.pill}"
  clock:
    typography: "{typography.clock}"
    height: "26px"
    padding: "8px"
  desktop:
    backgroundColor: "{colors.bg}"
    width: "1280px"
    height: "800px"
  status-dot:
    backgroundColor: "{colors.success}"
    size: "6px"
    rounded: "{rounded.pill}"
  window:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
  window-titlebar:
    height: "40px"
    typography: "{typography.window-title}"
    textColor: "{colors.text}"
  titlebar-subtitle:
    typography: "{typography.window-subtitle}"
    textColor: "{colors.text-3}"
  hairline:
    backgroundColor: "{colors.line}"
    width: "1248px"
    height: "1px"
  divider:
    backgroundColor: "{colors.line-strong}"
    width: "1248px"
    height: "1px"
  settings-profile:
    width: "217px"
    height: "48px"
  settings-nav-group-label:
    typography: "{typography.label-caps}"
  settings-nav-item:
    height: "30px"
    rounded: "{rounded.md}"
  settings-footer:
    width: "237px"
    height: "35px"
  traffic-light:
    size: "11px"
    rounded: "{rounded.pill}"
  menu-popover:
    backgroundColor: "{colors.popover}"
    rounded: "{rounded.md}"
    padding: "5px"
    width: "244px"
  menu-row:
    typography: "{typography.menu-row}"
    height: "32px"
  dock:
    backgroundColor: "{colors.surface}"
    height: "62px"
    padding: "8px"
    rounded: "{rounded.dock}"
  dock-item:
    size: "46px"
  dock-tile:
    size: "38px"
    rounded: "{rounded.md}"
  tray-network:
    backgroundColor: "{colors.popover}"
    width: "300px"
    height: "328px"
    rounded: "{rounded.lg}"
  tray-devices:
    backgroundColor: "{colors.popover}"
    width: "300px"
    height: "253px"
    rounded: "{rounded.lg}"
  tray-battery:
    backgroundColor: "{colors.popover}"
    width: "286px"
    height: "270px"
    rounded: "{rounded.lg}"
  tray-sound:
    backgroundColor: "{colors.popover}"
    width: "292px"
    height: "256px"
    rounded: "{rounded.lg}"
  tray-calendar:
    backgroundColor: "{colors.popover}"
    width: "308px"
    height: "351px"
    rounded: "{rounded.lg}"
```

## Do's and Don'ts

- Do reference tokens instead of literal colours.
- Do keep item heights on the 26/30/32 px rhythm.
- Don't add a surface without declaring its component here.
