# 0095 — The menu-broker resolves the global menu; the fixed application menu owns live hide-verb state

## Status

accepted — implements the broker side of [06-global-menu.md](../06-global-menu.md) and ADR [0041](0041-native-menu-model-publication-shape.md)

## Context

[06-global-menu.md](../06-global-menu.md) defines a three-tier global menu
(native publication → DBusMenu → no exporter) and a fixed application menu
whose Hide/Hide Others/Show All verbs are "mapped to compositor window state".
ADR [0041](0041-native-menu-model-publication-shape.md) froze the native
publication payload (the design-system entry shape) and explicitly left the
cross-process channel open; T-09.6a publishes Settings' model in that shape but
does not transport it. `services/menu-broker` was a scaffold stub, and the
shell synthesized a static application menu (no live state) plus a demo app
menu.

## Decision

- **D-Bus is the native publication channel.**
  `services/menu-broker` serves `org.dragonfruit.MenuBroker1` at
  `/org/dragonfruit/MenuBroker1`. First-party apps call `Publish(appId, model)`
  with the ADR-0041 shape and `Withdraw(appId)`; the shell is the window-state
  authority and calls `SetFocusedApp(appId)` and `SetWindowStates(json)`, then
  reads `Resolve`/`ResolveFocused`. `Policy(appId)` exposes the Tier-3 fixed
  menu alone. `Changed` is emitted after any mutation, so consumers re-read
  lazily and never poll. Payloads are flat JSON strings, matching the sibling
  services.
- **The broker owns the fixed application menu and its live state.** The hide
  verbs' enabled flags are derived from the window list: Hide `<App>` requires
  the focused app to have a visible (non-minimized) window; Hide Others
  requires another visible app; Show All requires at least one fully-hidden
  app. A publisher's `applicationMenuItems` is used verbatim except for those
  three rows' `enabled` fields, which the broker rewrites. An unpublished app
  gets the standard synthesized menu.
- **The shell mirrors the rule in a pure policy, not a second protocol.**
  `shell/src/menubrokerpolicy.{h,cpp}` maps the Dock running projection onto
  the same flags so the demo bar is live while the broker is absent; the broker
  remains the single owner of the model, and the shell consumes its resolved
  JSON once the transport is wired. The system menu stays shell/session-owned
  and is not part of the broker's model.

## Consequences

- Accelerators and the global-menu on/off toggle (T-14.2b) extend the same
  interface additively rather than inventing a new channel; they also wire the
  shell to push window state and consume `Resolve`.
- The DBusMenu/AppMenu bridge (T-14.4) plugs in as the `DbusMenu` tier without
  changing the payload shape.
- The demo bar shows live Hide/Hide Others/Show All state with no service
  running; a session that runs the broker gets the same answer from the
  service.
- Actual hide/hide-others/show-all *actions* (minimizing the affected windows)
  still need a compositor window-state request; T-14.2a only reflects the live
  state.