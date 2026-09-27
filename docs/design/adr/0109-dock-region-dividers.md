# 0109 — Dock region dividers are a projection of region structure

## Status

accepted

## Context

The legacy Dock design ([10-dock.md](../../tasks/legacy/10-dock.md) §4) and the
local reference `docs/reference/macos/Dock.png` show the app region split into a
**pinned prefix** and a **temporary/recent tail**, with a rule before the fixed
stacks/Trash tail. After T-14.7u the shipped Dock emitted only one divider
(between the app region and the minimized/stack/Trash region) and used the icon
`gap` on both sides of it, so pinned and temporary entries read as one
contiguous run and the rule was too short.

## Decision

- **The model owns the region structure.** `dockmodel` exposes
  `DockRegionPlan` / `planDockRegions`: the four regions are the pinned prefix,
  the temporary/recent tail (including the overflow cell), the minimized group,
  and the fixed stacks/Trash tail. A divider is a projection of the region list
  — never an entry, never draggable or reorderable.
- **A rule sits between each pair of adjacent non-empty regions.** Empty
  regions collapse away, so an empty tail or an empty pinned prefix leaves no
  orphaned or doubled rule; a Dock with no app region at all (only the fixed
  tail) draws none. `applyDockOverflow` reserves every divider it plans.
- **Two new boundary markers join the existing one.** The pinned | tail rule
  and the minimized | fixed rule are non-interactive. Only the app | right-region
  divider keeps the T-10 §5 drag resize handle and the Dock options menu, so a
  new boundary never adds a second handle.
- **The rule gets over-sized room and near-plate height.**
  `component.dock.divider.gap` (22 px) replaces the icon `gap` on both sides of
  any divider, in the resting and magnified layouts, on every Dock position;
  `divider.heightRatio` rises 0.6 → 0.82 so the hairline spans a fixed
  fraction of the plate cross-axis. A divider's layout box spans the plate
  cross-axis while its along-axis slot stays `dividerWidth`.

## Consequences

- The resting Dock reads like the reference: `[pinned] │ [temporary/recent] │
  [stacks + Trash]` with a 93 px pitch across a rule at the default icon size.
- The pinned-order drag path is unchanged in `dock.pinned` terms; the QML layout
  maps app-entry indices through `appItemIndices` because the inserted rule
  shifts the tail entries. External-drop insertion skips rules instead of
  stopping at them.
- The overflow solver counts every planned divider; the divider-gap itself is
  still approximated by the uniform icon `gap` in the fit estimate (the clamp
  remains a worst-case budget).
- Region headers, Space grouping, user-configurable divider spacing, and
  divider-as-action-affordance remain explicitly deferred.