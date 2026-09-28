# 0120 — The Storage pane and tile ride the status bridge host

## Status

accepted

## Context

T-15.2b renders the storage adapter ([0119](0119-storage-adapter-absence-and-mount-outcomes.md))
as one functional unit: a Settings pane and a Control Center tile. It follows
the Bluetooth pane/tile ([0118](0118-bluetooth-pane-and-tile.md)). The adapter
is a Rust crate and the two consumers are C++/QML; the project's rule is that
D-Bus is the services seam and QML never links an adapter ([0029](0029-system-status-bridge-host.md)).
The Settings app is a consumer, not an owner ([0036](0036-settings-app-consumer-client.md)).

## Decision

- **A fourth interface on the existing bridge host, not a second service.**
  `StorageHost<DbusUDisks>` sits beside `StatusHost` and `BluetoothHost` under
  one `org.dragonfruit.SystemStatus1`, served at
  `org.dragonfruit.SystemStatus1.Storage` with `State()`, `Refresh()`,
  `Mount(path)`, `Unmount(path)`, and `Eject(path)`. Its `view()` is a flat JSON
  projection of `StorageSnapshot` (present flag, summary counts, drive/volume
  lists); reports use the shared `accepted`/`denied`/`absent`/`failed` shape so
  one shell decode path reads every action.
- **The shell and the app use their existing seams.** The shell's
  `SystemStatusClient`/`SystemStatusModel` gain `storage`/`storageVisible` and
  the three writes; the Settings app's `StorageClient` (live `DbusStorageClient`
  + `MockStorageClient` under `DF_STORAGE_FIXTURE`) is the pane's one accessor,
  exposed through the `Settings` singleton.
- **Storage ships as a top-level Settings pane.** The reference puts Storage
  under `General > Storage`, but `General` has not shipped (T-15.10). A
  top-level `Storage` row keeps the no-half-panes rule without gating the pane
  behind a later task; when General lands the row can move into it.
- **The tile is not a toggle.** Unlike Bluetooth there is no radio: the tile
  shows the mounted-count subtitle and a Mount/Unmount row per volume, with an
  Eject action on removable volumes and a `Storage Settings…` entry point.
- **The status link is an entry point only.** Launching Settings on a named
  pane is T-16; the shell logs until then, matching Wi-Fi/Focus/Appearance.

## Consequences

- The absent-daemon matrix (T-15.16) drives Storage three ways: mask the bridge
  host (view empty), mask `org.freedesktop.UDisks2` (`unavailable`), or drop
  the mountable volume (`present: false`).
- The Settings link from tile to pane is T-16, as for the other tiles.
- The pane applies live through the adapter; no pane control requires a restart
  and none writes settingsd.