# Path to 100% parity

This document is the dedicated answer to: **what is needed to reach 100% parity?**

It records the current measured state, defines what “100%” must mean, lists the
remaining measurement and product work, and calls out the decisions that change
the target. It is a plan, not a claim that parity has been reached.

## 0. Current snapshot

Measured with `bun run parity:scene`, `bun run parity:state` and
`bun run parity:design:check` at 1280 × 800.

| Metric | Current | Target |
| --- | ---: | ---: |
| Paired surfaces | 29 | all in scope |
| Surfaces complete by text | 3 (`radio`, `calendar`, `menubar`) | all |
| Text runs matched | 434 / 842 (51.5 %) | 100 % minus a declared data allowlist |
| Matched text within tolerance | 197 / 434 (45.4 %) | 100 % within 1 px |
| Design-state keys matching | 101 / 108 | 100 % |
| State differences accepted | 7 | 0 outside out-of-scope |
| DESIGN.md paths accepted | 9 | 0 outside out-of-scope |
| DESIGN.md paths open | 0 | 0 |
| Pixel comparison | 6.70 % differ across 24 same-size surfaces; 13 size mismatches | per-surface pixel threshold |

The first two gates from the plan are now in place: the scene baseline stores
both text coverage and geometry alignment, and `parity:pixels` compares every
same-size crop with pixelmatch and fails on a regression. The overall pixel
ratio is **6.87 %**; the 13 skipped pairs are crops whose source and Aether
sizes still differ (dock, menu, command and the Aether-only overlays).

Geometry uses x ≤ 2 px and y ≤ 6 px for text boxes. Source scenes now measure
the text node with `Range.getBoundingClientRect`, not the full-width container
(which made centred weekday labels look 43 px off); Aether uses its text draw
origin. The 1 px target applies once the remaining font-metric differences are
removed.

Two examples show why text coverage alone is not parity:

- **Calendar** matches 100 % of its text; after measuring source text nodes
  instead of their full-width containers, 7 of 12 runs align (weekday columns
  now compare text-to-text).
- **Menu bar** matches 100 % of its text, but only **1 of 10** runs aligns: the
  resource captions are **6–7 px** right and the menus **3–4 px** left, a
  remaining packing difference now that the variable weights themselves match.

## 1. What “100 %” must mean

A defensible target needs a rubric, not a single percentage:

| Dimension | Pass condition |
| --- | --- |
| Structure | Every declared component/role exists; no missing nodes or surfaces |
| Content | Every text run matches, except an explicit live-data allowlist |
| Geometry | Every matched box within **1 px** in x/y and width/height |
| Style | Colours exact or ΔE ≤ 1; radii, borders, shadows and opacity match; blur implemented |
| Pixels | Per surface, **≤ 0.5 %** of pixels differ by more than 8/255 using a perceptual or pixel diff |
| States | Default, hover, focus, menus/trays open, water/light/dark, compact/comfortable/large, 1280/1440 |
| Scope | No accepted differences outside surfaces explicitly declared out of scope |

Without this rubric, “100 %” is undefined: the reference is a web concept with
variable fonts, backdrop blur and demo data; Aether is a real OS with live data.

## 2. Measurement still missing

| Gap | Why it matters |
| --- | --- |
| Panels carry origin only | Cards, shadows, gradients and backgrounds have no measured size, colour or radius |
| No z-order or clipping | Overlap, occlusion and scroll clipping are unverified |
| No client-surface comparison | Terminal/client content is invisible to the scene tree |
| No icon/asset diff | Logo, tray glyphs and tab icons differ but score nothing |
| ~~Geometry is reported, not gated~~ | **Done:** `state/scene-baseline.json` stores coverage and aligned ratio; a drop fails. Current: 43.2 % aligned |
| ~~No per-surface pixel diff~~ | **Done:** `parity:pixels` compares same-size crops and baselines the ratio. Current: 6.70 % pixels differ |
| No state matrix | Hover/focus/themes/font sizes/breakpoints are not captured |
| Live data pollutes content | Process/file/IP/user differences are counted as missing text |

The scene tree is a good start: it already records roles, boxes and text for
window chrome, Settings, Explorer, Task Manager, Calendar, trays, menu bar,
dock and the command palette, and it proves the extraction is possible. It is
not yet a complete visual-tree differ.

## 3. Product work required

| Area | Current gap | Work needed |
| --- | --- | --- |
| **Fonts** | ~~Demo uses variable weights 550/600/650/720; Aether has regular/bold only.~~ | **Done:** `aether-text` now uses swash and the `wght` axis; menu-bar, Settings, window and resource roles carry 550/600/650/700. |
| **Settings** | Row height 34 px vs the demo’s ~47/48 px; per-row descriptions missing; section/card spacing and per-tab content differ | Row descriptions, 47 px rows, section spacing, per-tab sections/controls, per-app switches |
| **Explorer** | Column widths, sidebar geometry, search/filter affordances and row layout differ; data is different | Match sidebar/toolbar/table geometry, row metrics, filter behaviour, deterministic file fixtures |
| **Task Manager** | Tabs, summary cards, columns and process/user data differ | Match pane/card/column layout and row metrics; fixture processes/users |
| **Calendar** | Weekday/column geometry ~43 px off; agenda spacing and event layout differ | Match grid columns, cell sizes, agenda rows, event blocks; fixture events |
| **Trays** | Section/row layout and data (SSID, devices, battery, audio) differ | Match each tray’s sections and row metrics; fixture data |
| **Menu bar & menus** | Menu entries/commands differ; font weights shift item positions | Exact menu content, variable weights, popover row metrics |
| **Command palette** | Different provider model and entries (explorer commands vs workspaces/agents/files) | Port the demo’s palette model or declare the difference intentional |
| **Dock** | Accepted drift: 46 px items vs 48, 38 px tiles vs 39.36, 8 vs 10 tiles, 8 px vs 7 px padding | Change geometry and hit targets, or formally scope the extra demo tiles out |
| **Effects** | Window glass 98 % vs 92 % opaque; no backdrop blur/saturate; gradients/shadows/tide phase differ | Backdrop blur, alpha, gradient and shadow parity (V8 in `VISUAL-SPEC.md`) |
| **Icons** | Custom SVG set differs from the demo | Port exact SVG paths for logo, tabs, trays and app marks |
| **Content/data** | Real host data vs demo fixtures | Deterministic fixture mode, or an explicit data-out-of-scope decision |

## 4. Decisions required

These change the size of “100 %”:

1. **Pixel-identical or structural?** Pixel-identical, or same structure/style
   within 1 px plus a pixel threshold?
2. **Does live data count?** If yes, Aether needs a demo-fixture mode (user,
   files, processes, IPs). If no, define a data allowlist.
3. **Do Aether-only surfaces count?** Audio, Chat, Agent sandboxes and Remote
   have no source surface. Exclude them or build source equivalents.
4. **Are Data Cloud Compare and App Studio permanently out of scope?** They are
   currently excluded by decision.
5. **Which states count?** Minimum recommendation: menus/trays open, hover,
   focus, water/light/dark, compact/comfortable/large, 1280/1440.
6. **Is the production cost acceptable?** Variable fonts and backdrop blur add
   binary size and GPU work; that conflicts with “do not ship more than needed”
   unless they are core parity requirements.

## 5. Sequencing and effort

Highest leverage first:

| Order | Work | Rough effort |
| --- | --- | --- |
| 1 | Lock the rubric; make geometry and pixels fail CI; add state matrix | 2–3 days |
| 2 | ~~Variable font weights; re-measure geometry~~ | **Done:** swash `wght` axis, 7 state waivers resolved |
| 3 | Settings row heights, descriptions, section spacing, per-tab content | 3–5 days |
| 4 | Dock geometry, exact icon set, glass/blur effects | 3–5 days |
| 5 | Explorer, Task Manager, Calendar and trays, one at a time, with fixtures | 2–4 days each |
| 6 | Per-surface pixel diff, drive accepted baselines to zero | 3–5 days |

Total: roughly **3–5 focused weeks** for the strict rubric, excluding the two
out-of-scope demo products. A looser target — chrome structure and style within
1 px, data excluded, Aether-only surfaces excluded — is substantially smaller.

## 6. Definition of done

- [ ] Every in-scope surface has a source and Aether scene and a pixel pair.
- [ ] 100 % of declared roles/components exist in both trees.
- [ ] 100 % of text runs match, or are on the data allowlist.
- [ ] 100 % of matched boxes are within 1 px in x/y/w/h.
- [ ] Colours/radii/borders/shadows/opacity match within the style tolerance.
- [ ] Each surface passes the pixel threshold in every declared state.
- [ ] `state/baseline.json`, `design/accepted.json` and `design/open.json` are
   empty except for declared out-of-scope items.
- [ ] `bun run parity:scene`, `parity:state`, `parity:design:check` and the
   pixel check all pass in CI.

## 7. Commands

```bash
bun run parity:scene         # text + geometry coverage, regressions fail
bun run parity:pixels        # pixelmatch per surface, regressions fail
bun run parity:state         # 108 design-state keys
bun run parity:design:check  # DESIGN.md contract
bun run parity:apps          # source + Aether captures and the report
```