# 0022 — The app-switcher overlay reuses the grid transform and gets its cards from the protocol

## Status

accepted

## Context

T-06.2a draws the Cmd-Tab overlay: one card per app in recency order, over the
**live** window surfaces (never thumbnails, [06-loop-v4-app-switcher.md](../tracks/06-loop-v4-app-switcher.md)).
ADR 0021 put the recency snapshot and the selection in the one compositor-owned
`app_switcher` machine, but the private `df_toplevel_manager.app_switcher` event
carried only `(active, selected app_id, direction)` — enough to render the
selection highlight, not the `N` cards the overlay needs. The shell must not
re-derive recency (it does not even see the compositor's `WindowModel`
recency), and the live previews cannot be drawn by the shell (it never sees
client buffers).

## Decision

- **Cards come from the protocol.** Add the additive `df_toplevel_manager`
  event `app_switcher_entry(index, app_id)`, emitted once per recency entry
  between the `app_switcher` event and the batch `done`. The shell accumulates
  the batch and emits one `appSwitcherChanged` projection per completed batch;
  it never sorts or re-derives the order.
- **Previews reuse the one T-04 transform.** The compositor builds one
  `GridCandidate` per entry (the entry's most-recent window and its committed
  geometry) and lays them out with the existing Mission Control
  `grid_layout`/`GridPlacement::frame` over an area that reserves the shell's
  card strip at the bottom (`overview::switcher::preview_area`). There is no
  switcher-specific layout or transform; a non-entry window is drawn with
  `alpha = 0` while the overlay owns the scene.
- **The shell draws only chrome.** A fourth offscreen QML scene
  (`shell/switcher/AppSwitcher.qml`) renders the scrim, the centered cards, the
  selection highlight, the accessible names, and the reduced-motion variant on
  a full-output `app-switcher` overlay surface; the compositor's live previews
  show through underneath.

Rejected: the shell resolving window/app recency from its own event stream
(drifts from `WindowModel`, and the Dock already documents that gap); a second
switcher layout/transform (would diverge from the T-04 scene transform and the
degrade tier); thumbnails (explicitly forbidden by the track).

## Consequences

- T-06.2b's commit and Cmd+` cycling consume the same `app_switcher` +
  `app_switcher_entry` projection and must not add a second recency source.
  The entry does not yet carry the window handle; if T-06.2b needs it to
  address a window directly, it is appended additively (a new arg or event),
  never by changing the existing signature.
- The switcher material is the same `GridMaterial` (degrade tier + scheme) the
  Mission Control grid uses, so the selected-entry full-fidelity vs
  scaled-preview split is a render concern, not a second material.
- `query switcher` reports one `switcher preview <window> <x> <y> <w> <h>
  selected=<0|1>` line per live preview, the headless seam for the transform.
- The shell card row is a non-wrapping `Row`; many apps need paging/wrapping
  (a later polish task).