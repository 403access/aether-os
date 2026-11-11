# Water — Aether's design system

This directory is **vendored from the Data Cloud Compare project**
(`/Users/olivermolnar/Developer/jottacloud-comparison/design-system`) and is the
single source of truth for colour, type, spacing, motion and component
primitives across every Aether surface: the marketing site, the documentation,
the React UI kit, and the explorer application.

Reference implementation (live at time of import):
**http://localhost:8787/** — the `Tank ⇄ Jottacloud` two-pane explorer.

## Why this design system

It was built for exactly the interaction model Aether needs: dense file lists,
a command palette, glass chrome over a deep-sea background, semantic status
pills, and slow ambient motion that never competes with the work. Reusing it
keeps the OS shell, the browser-based remote session, and the web surfaces
visually identical — one product, one language.

## Rules carried over

1. **Water is the default brand theme.** Light is a "shallow water" variant, not
   a separate identity.
2. **One accent current.** Cyan `--ds-accent`; status colours are semantic.
3. **Ambient motion is slow** (14–58 s), interface motion is fast (160–340 ms).
4. **Every effect can be switched off**; `prefers-reduced-motion` disables it.
5. **Colour never carries meaning alone.** Status pills always pair a colour
   with a text label and a dot.
6. **Dense by default.** 13 px base, 30 px rows, 62 px header. This is a
   workstation, not a tablet.

## Files

| File | Purpose |
| --- | --- |
| `tokens.css` | `--ds-*` colour, type, space, radius, elevation, motion, water tokens |
| `components.css` | `ds-` prefixed primitives: buttons, cards, badges, panes, command bar, tables |
| `water.js` | Dependency-free canvas caustics, drips and click ripples |
| `index.html` + `showcase.*` | Standalone component gallery, served at `/design-system/` |

## Usage

```ts
import "@aether/design-system/tokens.css";
import "@aether/design-system/components.css";
```

```html
<html data-theme="dark" data-water-tone="ocean" data-water-speed="flowing">
  <div class="ds-scope">
    <button class="ds-btn ds-btn--primary">Reveal counterpart</button>
    <span class="ds-badge ds-badge--same"><span class="ds-dot"></span>Same</span>
  </div>
</html>
```

The marketing site and docs both import these files directly; `packages/ui`
wraps the primitives for React.

## Divergence policy

Treat this folder as an upstream import. Aether-specific additions belong in
`packages/ui` or `apps/web/src/theme.css`, not here, so future re-imports stay
mechanical. If a change must happen upstream, apply it in both repositories and
record it in `apps/docs/docs/design/design-system.md`.
