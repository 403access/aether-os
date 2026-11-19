# Water OS visual spec · source vs Aether

This document formalises both implementations of the Water desktop — the
reference concept in `jottacloud-comparison/design-system/os` and the Aether
compositor — and records the differences that remain after the parity work.

Everything is measured at **1280 × 800**, with the demo clock pinned to
`2026-11-18 16:47 UTC` (`AETHER_FIXED_UNIX` on the Aether side). The OS concept
scales its type by `--os-font-scale: 1.14`, so a token like `--ds-text-xs`
(`0.6875rem`, 11 px) renders at **12.54 px** and `--ds-text-2xs` at **11.4 px**.

## 1. Method

| Side | How it is measured |
| --- | --- |
| Source | Playwright at 1280 × 800: `getBoundingClientRect`, `Range.getBoundingClientRect` for text boxes, `getComputedStyle` for fonts/padding/effects, computed `::before` for the tide ring |
| Target | Aether capture under Xvfb at 1280 × 800, plus the geometry constants in `aether-shell` / `aether-shell-wayland`; text is compared by **rasterised ink bands** (first/last row containing text pixels inside a known item rect) |

The menu-bar table below is an example of the measurement used throughout:

| Element | Source font / ink | Source ink rows | Aether ink rows (after fix) |
| --- | --- | --- | --- |
| App title | 12.54 px / 720 | 12–24 | 12–21 |
| File, View | 12.54 px / 550 | 13–22 | 13–22 |
| Synced | 11.4 px / 400 | 14–21 | 14–23 |
| Clock date | 11.4 px / 400 | 14–21 | 14–23 |
| CPU label | 7.98 px / 650 | 17–19 | 16–21 |
| CPU value | 9.69 px / 600 | 15–20 | 16–22 |

## 2. Source model

### 2.1 Tokens

- **Type:** `--ds-text-2xs` 11 px → 12.54 px OS, `--ds-text-2xs` 10 px →
  11.4 px OS, body 13 px. Inter Variable throughout.
- **Colour:** `--ds-bg #031922`, `--ds-bg-soft #052530`,
  `--ds-surface-solid #072733`, `--ds-popover #0D3D4B`, `--ds-text #E9FBFF`,
  `--ds-text-2 #A5DBE5`, `--ds-text-3 #6BA9B6`, `--ds-accent #38D6FF`,
  `--ds-line` accent @ 15 %, `--ds-line-strong` @ 30 %.
- **Shape:** radius xs 5, md 10, lg 14, 2xl 26; shadows soft / base / pop.
- **Water:** tide cycle 8 s (`--os-wave-cycle`), ring 2.5 px, peak opacity 0.85,
  displacement filter scale 5, two conic highlight lobes.

### 2.2 Menu bar (`.os-menubar`, `css/menubar.css`)

- 38 px tall, padding `0 10px`, `justify-content: space-between`, gap 18 px.
- Background `color-mix(bg-soft 72%)` + `backdrop-filter: blur(22px) saturate(160%)`,
  1 px bottom border `--ds-line`.
- Logo 28 × 26 at x = 10, svg 21 px (ink 16 × 21 at 16, 8).
- App title 12.54 px / 720, max 160 px, padding `0 8px 0 2px`.
- Menus: 26 px tall, padding `0 9px`, 12.54 px / 550, gap 1 px; hover/open
  background `--ds-hover`.
- Sync pill 26 px, 11.4 px, 1 px line, 6 px dot.
- Resources: 26 px group, caption 7.98 px / 650 (`0.04em`), meter 20 × 3,
  value 9.69 px / 600, separators 1 px line.
- Clock: 26 px, 11.4 px, date `--ds-text-3`, time `--ds-text` / 650, gap 7 px.
- Icons: 15 px in 26 px buttons (tray buttons 31 px wide, battery 62 px).

### 2.3 Windows (`css/windows.css`)

- Radius 14, border `--os-rim`, background `color-mix(surface-solid 92%)`.
- Focused rim `color-mix(accent 64%, line)`, glow 18 %; hovered 34 %.
- Shadow `0 34px 90px rgba(0,8,14,.62), 0 3px 12px rgba(0,8,14,.4), 0 0 0 1px
  rgba(accent,.2), 0 0 44px rgba(accent,.18), inset 0 1px 0 rgba(255,255,255,.07)`.
- Title bar 40 px, padding `0 10px`, gap 10, lights 11 px with 6 px gaps,
  app mark 23 px (icon 13), title 12.54 px / 650, subtitle 11.4 px `--ds-text-3`.
- **Tide ring:** `::before` inset 0, padding 2.5 px, masked to the border,
  `conic-gradient` accent 45 % / white–accent 52° / accent, opacity 0.85 when
  focused, filter `url(#os-window-wave)` (fractal noise, displacement scale 5),
  animation `os-window-tide 8s linear infinite`.

### 2.4 Other chrome

- Dock: 46 px items, 38 px tiles radius 10, gradient tile fill, hover lift
  `translateY(-7px) scale(1.08)`, label on hover, running dot 4 px.
- Menu popover: `top: calc(100% + 7px)`, min-width 244 px, padding 5 px,
  1 px strong border, radius md, `--ds-shadow-pop`; rows min-height 30 px,
  padding `5px 8px`, gap 9 px.
- Command overlay: same window body + tide ring, always on.
- Trays: 301 × 329 (Wi-Fi), 301 × 253 (Bluetooth), 287 × 271 (battery),
  293 × 256 (sound), 308 × 351 (calendar).

## 3. Target model

| Source rule | Aether implementation |
| --- | --- |
| `--ds-*` tokens | `crates/aether-shell/src/theme/tokens.rs` + `design.rs`, verified by `bun run test:design` |
| `chrome::panel/rounded/top/bottom/outline` | tiny-skia buffers in `render/chrome/primitives.rs`, cached in byte-bounded `BudgetCache`s |
| Text | `aether-text` (fontdue + InterVariable), scales 0–6 = 11/13/20/16/9/10/8 px |
| Menu bar | `app/overlays/menubar/{layout,menubar}.rs` + `render/surfaces/status.rs` |
| Windows | `render/window_frame.rs` (clients), `render/surfaces/system_app.rs` (first-party), `explorer.rs`, `calendar.rs`, `task_manager.rs` |
| Shadows | `primitives::shadow` — three stacked rounded layers, 28 px pad |
| Tide ring | `primitives::wave_rim` — 2.5 px rounded-rect ring, sine displacement, two conic lobes, 24 phase steps |
| Menus/trays | `app/overlays/menubar/layout.rs`, `app/tray.rs`, `render/surfaces/tray.rs` |

## 4. Difference register

Status: **fixed** (landed), **approximated** (visually close, different
technique), **open** (visible gap, not implemented).

| # | Area | Source | Aether | Status |
| --- | --- | --- | --- | --- |
| V1 | Menu bar text alignment | 12.54/11.4 px, ink rows 12–24 | rendered 11 px bold at the wrong offsets | **fixed** (`07c8216`) |
| V2 | Resource meters | captions/values centred with the meter | 4–5 px too high | **fixed** (`07c8216`) |
| V3 | Brand mark | 21 px svg, ink at (16, 8) | 20 px at (14, 3) | **fixed** (`07c8216`) |
| V4 | Window shadow | 34 px/90 px, 62 % | no shadow | **fixed** (`8c02033`) |
| V5 | Tide rim | 2.5 px conic ring + noise displacement | sine-displaced ring, two lobes | **approximated** (`8c02033`) |
| V6 | Tide motion | continuous 8 s rotation | phase advances on rendered frames only | **open** |
| V7 | Menu popover geometry | top +7, min-width 244, padding 5, pop shadow | matched | **fixed** (`8c02033`) |
| V8 | Glass blur | `backdrop-filter: blur(22px) saturate(160%)` | flat translucent fills | **open** |
| V9 | Window inset highlight | `inset 0 1px 0 rgba(255,255,255,.07)` | 1 px white 7 % line inside the top edge | **fixed** (`65564da`) |
| V10 | Hover states | rim 34 % → 80 %, shadow grows, dock lift + label | dock lifts to 41 px with a label; window rims brighten to 84 % on hover | **fixed (rim + dock)** (`09aa929`, `HEAD`) |
| V11 | Dock tiles | gradient + 1 px tile border | cached 160° gradient + border | **fixed** (`e978f96`) |
| V12 | Type scale | 12.54 / 11.4 / 9.69 / 7.98 px | exact OS scales in chrome; other surfaces still integer | **fixed (chrome)** (`2125be4`) |
| V13 | Letter spacing | −0.01em titles, 0.04em captions | renderer supports spacing; applied to titles/captions | **fixed** (`2125be4`) |
| V14 | Menu row type | 14.82 px (`--ds-text-sm`) | 14.82 px | **fixed** (`2125be4`) |
| V15 | Command overlay | grouped command surface | provider palette with rim | **open** |
| V16 | Resize handles / grip | 24 px grip icon | three diagonal ticks at the bottom-right | **fixed** (`6782bf3`) |

## 5. Verification

- Ink-band measurement of the menu bar before/after (V1–V3).
- Element screenshots per app/tray in `reports/os-app-parity`.
- `cargo test` for `aether-shell` and `aether-shell-wayland`, clippy `-D
  warnings`, and the Xvfb smokes.
- Regenerate captures with `bun run screenshots:shell`, then
  `bun run parity:apps` for the report.

## 6. Next tasks (visual)

1. **V6** — schedule the tide phase at ~12 fps while a window is focused
   (needs a deadline in the winit dispatch loop, which currently blocks on
   events).
2. **V8** — backdrop blur for the bar, popovers and windows (read the backdrop
   region into a buffer and blur it before compositing).
3. **V10** — shadow growth on hover (rim and dock hover are done).
4. **V15** — regroup the palette to the demo's command surface.