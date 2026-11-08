# 0241 — Adopt the 2026-11-08 macOS structural surface review

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-08 |
| **Release** | v0.2.0 |
| **Source** | `macos-design-review/docs/SURFACE-REVIEW.md`, `experience_measurements.json`, `REPORT.md` §3.9 |
| **Supersedes** | none |

## Context

The Liquid Glass campaign measured the materials of the Dock, menu bar,
popovers, Control Center, notifications and window chrome. It did not measure
the **structure** of the chrome around those materials. The 2026-11-08 macOS
27.0.1 review closes that gap for window title bars, About, the Applications
drawer, Notification Center, Control Center and Finder.

Several current Dragonfruit assumptions conflict with the review:

- the design system uses one 40 pt titlebar token, while the reference chrome
  is a 0/32/40/48/52/66/88 pt style system;
- the Applications drawer task still describes Launchpad pages and folders,
  but macOS 26/27 removed Launchpad and the reference surface is Spotlight
  Apps, a search-first grid/list with no pages or folders;
- About is a single dialog, while the reference has a compact 280×503 pt
  utility panel and a 740×820 pt Settings page;
- Notification Center is modelled as one material, while the reference has a
  three-band vertical ramp;
- Control Center is a curated tile list, while the reference is a 4-column
  64 pt grid with a 22-category Edit Controls catalogue;
- Finder's measured window anatomy (52 pt toolbar, 198 pt sidebar, 28/30/28 pt
  accessory bars, Get Info 328×1034 pt) is not in the Files task.

The review is measured evidence from a controlled macOS 27.0.1 machine. It is
reference evidence, not native Dragonfruit proof, and it contains explicit
OPEN rows for fullscreen chrome, RTL/long titles, dark About, drawer motion,
Notification Center variants, Control Center modules/pages, and Finder
customization/motion.

## Decision

Adopt the review as the structural design source of truth and apply it in this
order:

1. **Window chrome is a style taxonomy**, not one height. Use
   `borderless`, `titlebar`, `unifiedCompact`, `expanded`, `unified`,
   `preference` with the measured baselines 0/32/40/48/52/88 pt and the 66 pt
   unified icon-and-label variant. Traffic lights are 16×16 pt with 8 pt
   insets in a 32 pt bar, 18 pt insets in a 52 pt bar, and 23 pt spacing.
   Tab/path/status/format bars are content, not chrome.
2. **About has two surfaces**: a compact 280×503 pt, 32 pt-chrome utility
   panel and a 740×820 pt, 52 pt-chrome Settings page. The page is the
   canonical machine identity; the panel is a summary. Both use original
   artwork and host values only.
3. **The Applications drawer follows Spotlight Apps**, not Launchpad: a
   search-first, category-filtered, scrollable 7-column grid with a real
   full-height list mode; no pages, folders, or manual grid ordering.
4. **Notification Center is a right-edge 344 pt panel** with a banner stack,
   widget stack and lower panel on three separate material bands
   (0.549/0.976, 0.551/0.706, 0.407/0.852). The widget gallery is a separate
   1027×552 pt window with a 19-category sidebar.
5. **Control Center is a 4-column grid of 64 pt cells with 12 pt gaps**;
   modules span 1×1, 2×1, 2×2 and 4×1; expanded modules grow the outer panel;
   Edit Controls is a 736×900 pt catalogue with 22 categories; the outer panel
   and module capsules stay separate materials.
6. **Finder is the file-manager anatomy reference**: 52 pt toolbar, 198 pt
   sidebar, 28 pt tab bar, 30 pt path bar, 28 pt status bar, four real view
   modes, and a 328×1034 pt single-column Get Info inspector. Hiding the
   toolbar also hides the sidebar.
7. **Evidence stays honest.** The review's OPEN rows are carried into the
   experience manifest; a task may replace them with a capture or test, but
   may not silently drop them.

## Consequences

- New tasks T230a–T230d run after T230 and before the menu-bar extras:
  T230a extends the experience contract and reference inventory, T230b lands
  the chrome style system, T230c lands the two About surfaces, and T230d lands
  the Files chrome/Get Info geometry.
- T230a landed the contract extension: 64 append-only rows across `N-WIN`,
  `SET-GENERAL`, `Q-AD`, `S-NC`, `S-CC`, and the new `N-FILES` surface, plus
  the measured geometry/material thresholds in
  `docs/captures/experience-thresholds.json`. `N-FILES` is the Files window;
  `N-PREV` remains Quick Look. The 14 review `OPEN` ids are first-class rows
  with owner, rerun command, impact, and intended axes; no macOS screenshot is
  copied into Dragonfruit.
- T234, T235, T248 and T260 are rewritten to consume the measured structural
  values instead of their earlier generic descriptions.
- The existing 40 pt titlebar token remains valid only as a legacy/compact
  baseline until T230b migrates it. Do not add new hard-coded heights.
- The current Control Center tile list and Notification Center panel remain
  the implementation baseline; T234/T235 replace them without regressing the
  measured material roles.
- The current Files icon/list views remain; T230d adds the measured chrome and
  inspector, while column/gallery views stay tracked as OPEN in the Files
  design.

## References

- [macOS structural surface review](../23-macos-structural-surface-review.md)
- [Total desktop experience](../22-total-experience.md)
- [Window decorations](../05-window-decorations.md)
- [Files](../09-files.md)
- [Experience schema](../experience-schema.md)
- `macos-design-review/docs/SURFACE-REVIEW.md`
- `macos-design-review/docs/experience_measurements.json`
