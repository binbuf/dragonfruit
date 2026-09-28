# 0142 — The Privacy and Security adapter projects the portal PermissionStore

- **Status:** Accepted (T-15.13a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  [13-portals-capture-clipboard.md](../tracks/13-portals-capture-clipboard.md),
  ADR [0074](0074-portal-backend-registration-and-frontend-degradation.md),
  ADR [0140](0140-printers-and-scanners-adapter.md)

## Context

T-15.13 splits the macOS Privacy & Security surface into an adapter (T-15.13a)
and a Settings pane plus Control Center tile (T-15.13b). [07-system-integration.md]
routes the surface to host services and [08-settings.md] to
"`settingsd` + host services (Secret Service, polkit) and portals". On Linux the
desktop's actual privacy record is the `xdg-desktop-portal`
**PermissionStore**: the database portals already use to record which
applications may reach the resources they mediate (camera, location,
notifications, screen casting, remote desktop, USB, …). T-13 already ships our
portal backend and its frontend degradation rules; T-15's privacy pane is meant
to reference the portal settings ([13-portals-capture-clipboard.md]). The task
requires state, events, and absent-daemon behavior, unit-tested against a mock,
and forbids reimplementing the host stack.

## Decision

- **A new crate, `dragonfruit-privacy-adapter` (`services/privacy-adapter`).**
  It implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `PrivacySource` seam, exactly like the other
  T-15 adapters. `AdapterId::PRIVACY` (id `privacy`) joins the shared ids.
- **The host stack is the standard PermissionStore.** The live source,
  `HostPrivacy`, talks to `org.freedesktop.impl.portal.PermissionStore` on the
  **session bus** at `/org/freedesktop/impl/portal/PermissionStore` — the
  interface the portal spec documents and `xdg-desktop-portal`'s
  `xdg-permission-store` daemon serves. One read is `List(table)` for every
  curated table followed by `Lookup(table, id)` for each returned resource id;
  a write is one call each: `SetPermission(table, create, id, app,
  permissions)` and `DeletePermission(table, id, app)`. This is the same reuse
  the audio adapter makes of `pw-dump`/`wpctl` and the printer adapter of the
  CUPS tools; the project reimplements no portal and makes no permission
  decision.
- **The table vocabulary is the portal's, pinned in `KNOWN_TABLES`.** The
  fourteen tables `xdg-desktop-portal` uses (`devices`/camera, `location`,
  `notifications`, `screencast`, `remote-desktop`, `screenshot`, `background`,
  `usb`, `input-capture`, `gamemode`, `inhibit`, `realtime`, `wallpaper`,
  `desktop-used-apps`) become the pane's categories in a fixed order. A table
  the store has no entry for is an honest empty category, never missing and
  never an invented permission.
- **The permission value is a tristate, raw strings preserved.** The portal
  stores `yes`/`no`/`ask` for a yes/no/ask permission; `PermissionState`
  derives `Allowed`/`Denied`/`Ask`, and anything else (an empty list, a
  location accuracy record) stays `Unset` with the raw strings kept on
  `AppPermission`. Nothing is guessed.
- **Absence is normal and single-layered.** The adapter is `Unavailable` only
  when there is no session bus or no PermissionStore owner. A store that
  answers with no entry is `Available` with an empty snapshot; the pane's hide
  rule is `PrivacySnapshot::present()`. A store that owns its name but cannot
  be read is `Error`, visible and inert with the message.
- **Explicit writes, no invented snapshots.** `set_permission` and
  `delete_permission` answer `Applied`/`Denied`/`Absent`/`Failed`;
  `xdg-desktop-portal` publishes the result and the host re-reads. A policy
  refusal is surfaced per action without degrading the read state.
- **One event stream.** `PrivacySnapshot::changes(previous)` is a pure diff —
  resources added/removed and applications added/removed/changed — queued by
  the adapter and drained through `drain_changes`.

Rejected: reimplementing the portal permission store or reading its on-disk
database directly (the store's D-Bus API is the front door and its schema is
private); inventing macOS categories (Calendars, Photos, …) with no Linux host
owner behind them (the pane must be a projection of the host stack, not a
mock-up); hiding the whole item when one table is empty (an empty category is a
normal row with `None`); treating a missing store as an error (it is a normal
absence).

## Consequences

- `dragonfruit-privacy-adapter` is a workspace member whose dependencies are
  the adapter contract, `zbus`, and `serde`. CI drives it with `MockPrivacy`;
  no portal, bus, shell, or hardware is involved. `make e2e` runs
  `cargo test -p dragonfruit-privacy-adapter`.
- `HostPrivacy` is the live `PrivacySource`. Its read is best-effort over the
  store's free-form tables; the mock is the contract's CI path, and the table
  vocabulary and tristate are pinned by tests.
- T-15.13b adds the pane and tile. It must project this adapter's snapshot
  through the bridge host like the other T-15 adapters and must not add a
  second permission implementation. The durable presentation preferences
  `settingsd` owns are T-15.13b's to declare.
- The Secret Service (passkeys/passwords) and polkit host services the
  reference also names are **not** part of this adapter: it is the portal
  permission projection. A later task may add a read-only Secret Service
  projection if a row needs it.