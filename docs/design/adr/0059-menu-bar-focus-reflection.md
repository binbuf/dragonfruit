# 0059 — Menu-bar Focus/DND reflection

## Status

accepted

## Context

T-11.2a froze the service-owned Focus/DND policy and its shell-facing
`FocusPolicy()` JSON (`mode`, `allowList`, `batchedCount`)
([0058](0058-focus-dnd-policy-semantics.md)). The shell had a hidden `focus`
status slot (T-09) but no reader. T-11.3a's Control Center and the later
Settings Focus section need one agreed way to render the three-way state, and
the menu bar is the first consumer.

## Decision

- **One reader, one decode.** `NotificationClient` gains a `FocusPolicy()`
  read (re-read by `refresh()` on the service's `Changed`) emitting
  `focusPolicyChanged`; `NotificationModel::applyFocusPolicyJson` decodes it.
  No shell component polls.
- **One mapping, in `shell/src/focusstatus.{h,cpp}`.** The `focus` menu-bar
  item is a pure function of the policy map:
  - `off` (or absent/unknown) — hidden;
  - `focus` — visible, unselected crescent;
  - `dnd` — visible, `selected` (accent-tinted) crescent.
  The `batchedCount` is the item's label when non-zero and is always part of
  the accessible name.
- **A missing service hides the item.** An empty/malformed policy clears the
  model, so the bar never shows stale Focus state.

## Consequences

- The Control Center (T-11.3a) and Settings bind to the same
  `focusPolicyChanged` view and the same three mode names; only their chrome
  differs.
- `selected` is the frozen visual convention for "Do Not Disturb is active";
  the same predicate should drive the Control Center's Focus tile.
- Clicking the item is not a toggle yet: the Control Center owns the active
  controls (T-11.3a). The reflection itself is complete.
