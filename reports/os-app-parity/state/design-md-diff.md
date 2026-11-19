# DESIGN.md contract diff

Source: `design/source/DESIGN.md` · Aether: `design/aether/DESIGN.md`.

| path | state | note |
| --- | --- | --- |
| `colors.surface` | modified | accepted: Aether's window fill is 98% opaque; the demo uses color-mix(surface-solid 92%, transparent). This is the open backdrop-blur item (V8). |
| `colors.line-strong` | modified | accepted: The demo's computed hairline is 77/255 after color-mix; Aether paints 76/255. A 1/255 rounding difference. |
| `spacing.dock` | modified | accepted: Aether paints 8px of inner padding; the demo's 7px plus its 1px border gives the same visible inset. |
| `components.window` | modified | accepted: Cascades from colors.surface (92% vs 98% opacity). |
| `components.divider` | modified | accepted: Cascades from colors.line-strong (77/255 vs 76/255). |
| `components.dock` | modified | accepted: Cascades from spacing.dock and the dock height difference (62 vs 64). |
| `components.dock-item` | modified | accepted: Aether ships 46px dock items; the demo uses 48px. Tracked with the dock parity work. |
| `components.dock-tile` | modified | accepted: Aether ships 38px tiles; the demo uses 48 x 0.82 = 39.36px. |
| `components.tray-calendar` | modified | accepted: Whole-pixel rounding: the demo computes 350.41px, Aether 351px. |

Lint: source 0 errors / 0 warnings · Aether 0 errors / 0 warnings.
