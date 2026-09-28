# 0137 — The General pane and Software Update tile ride the bridge host

- **Status:** Accepted (T-15.10b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  [12-packaging.md](../12-packaging.md),
  ADR [0136](0136-general-about-updates-adapter.md),
  ADR [0135](0135-menu-bar-pane-and-tile.md),
  ADR [0120](0120-storage-pane-and-tile.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.10a shipped `dragonfruit-update-adapter`, which projects the host stack:
the host identity (About) and the distribution update provider (Updates) behind
a `SystemSource` seam. T-15.10b must ship the Settings General pane and the
Control Center Software Update tile as one functional unit, with a documented
absence case, and must not add a second update implementation.

The host identity is a real read of `/etc/os-release`, `/proc`, and the DMI
sysfs nodes; the concrete provider is distro-specific and lands with packaging
(T-16.9/T-16.10, ADR 0136). The C++/QML shell and Settings app never link a Rust
adapter (ADR [0029](0029-system-status-bridge-host.md)), so the data has to
cross the `org.dragonfruit.SystemStatus1` session-bus bridge.

## Decision

- **The bridge host grows an `Updates` interface.** `UpdatesHost`
  (`services/system-status/src/updates.rs`) owns the adapter, projects the typed
  `SystemSnapshot` to a flat JSON view (identity plus the optional provider),
  and exposes `Check()`/`Install()`/`Reboot()` over
  `org.dragonfruit.SystemStatus1.Updates` alongside `State()`/`Refresh()`. This
  mirrors the storage (T-15.2b) and notifications (T-15.7b) halves exactly: one
  projection, one explicit-write path, the read state as the only truth.
- **The live host attaches no provider yet.** `HostSystem::new()` reads the real
  identity with `updates: None`, which is the honest state until packaging
  supplies a `SystemProvider`; the pane's About rows are live and the update
  controls report absence. `MockSystem` drives the provider states in CI.
- **The Settings pane is `General`, the tile is `Software Update`.** The pane
  (`apps/settings/GeneralPane.qml`) mirrors the capture's grouped disclosure
  rows — `About` and `Software Update` — each opening its dialog, because that
  is the reference anatomy. The Control Center tile shows the live update
  summary plus the one action the state calls for (check, install, or restart)
  and a `General Settings…` link (T-16's launch entry point). No new settingsd
  keys: About is a live read and the update writes are explicit actions, like
  Storage; there are no durable presentation preferences to store.
- **The update provider is a `Send` seam.** `UpdateProvider` gains a `Send`
  bound so `HostSystem` can be served from the bridge host's D-Bus worker
  threads. The change is additive to ADR 0136 (all implementors already are
  `Send`).
- **Absence is layered and documented.** The adapter/view is `unavailable` only
  when neither the identity nor the provider is reachable. A running host with
  no provider is `available` with `updatesAvailable: false`: the About rows stay
  live and only the update controls disable. The Settings pane shows a one-line
  note when the bridge host is absent; the tile hides then too. The
  absent-provider matrix is asserted in `tst_settings_absence.qml`.
- **The Control Center panel stays fixed-height.** The Software Update tile is
  added to the existing fixed 360×1160 surface by tightening the tile padding to
  the `spacing.xs` token across the panel (the inter-tile gap was already `xs`);
  the panel's content fits the surface with margin, asserted by
  `tst_controlcenter.qml`. A scrolling panel or taller nested output is deferred
  (ADR 0135's note).

## Consequences

- The pane and tile are one unit over the T-15.10a adapter; neither reimplements
  the identity read or the update state. `make e2e` covers the Rust host and the
  C++/QML seams (`cargo test -p dragonfruit-system-status`,
  `tst_settings_general`, `tst_controlcenter`, `tst_statusmodel`).
- When T-16.9/T-16.10 lands the concrete distro provider, it is attached to
  `HostSystem` in `system-status`; nothing in this pane, the tile, or the
  adapter changes.
- General's remaining rows (Date & Time, Language & Region, Login Items,
  Sharing, Startup Disk) are other providers/future tasks and are omitted rather
  than shipped as dead controls; Storage is a first-class sidebar pane.