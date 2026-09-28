# 0134 — The Menu Bar configuration adapter projects the shell menu bar

- **Status:** Accepted (T-15.9a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [04-shell.md](../04-shell.md), [08-settings.md](../08-settings.md),
  ADR [0126](0126-mission-control-hot-corners-adapter.md),
  ADR [0132](0132-lock-screen-policy-adapter.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.9 splits the Menu Bar configuration surface into an adapter (T-15.9a) and
a Settings pane plus Control Center tile (T-15.9b). Menu Bar configuration has
**no external daemon**: the shell's `MenuBar`
(`shell/menubar/MenuBar.qml`) owns the chrome, the status-item model and the
clock, the menu-broker (`services/menu-broker`) resolves the focused app's
global menu, and `settingsd` owns the durable preferences both read.
`System_Preferences.md` routes the pane to "settingsd → shell" — the same split
principle 3 already gives the overview triggers and the lock-screen display
preferences: `settingsd` owns the preference and the shell applies it live.

The task requires an adapter with state, events, and absent-daemon behavior,
unit-tested against a mock. The risk is re-rendering the bar or re-resolving a
menu in a service, which the task forbids ("reuse, never reimplement") and
which would fork the shell's chrome and the menu-broker's resolution.

## Decision

- **A new crate, `dragonfruit-menubar-adapter` (`services/menubar-adapter`).**
  It implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `MenuBarSource` seam, exactly like
  `dragonfruit-overview` and `dragonfruit-lock-adapter`. `AdapterId::MENU_BAR`
  (id `menu-bar`) joins the shared ids.
- **It projects; it does not render or resolve.** The raw read is
  `MenuBarData`: whether the bar is hidden by auto-hide, the effective
  `MenuBarAutoHide` mode (`never`/`always`/`full-screen`, default
  `full-screen`), the background and global-menu flags, the `ClockOptions`
  (`showDate`/`showSeconds`, the two properties the shell clock already
  renders), and the availability of each `MenuBarControl`
  (`clock`/`wifi`/`bluetooth`/`battery`/`volume`/`focus`/`accessibility`).
  `MenuBarSnapshot` types all of it; control ids are the shell status-item ids
  so a projection maps straight onto the status row.
- **Per-control absence is not adapter absence.** One control's daemon going
  away is an absent `MenuBarControlState` slot inside an `Available` snapshot,
  so only that slot hides. The whole adapter is `Unavailable` only when the
  menu-bar bridge itself is missing.
- **One event stream.** `MenuBarSnapshot::changes(previous)` is a pure diff —
  the runtime hidden flag, the auto-hide mode, the background and global-menu
  flags, each clock option, each control slot — queued by the adapter and
  drained through `drain_changes`.
- **Read-only.** The durable preferences are `settingsd`'s (the keys land in
  T-15.9b) and the shell renders the bar. The adapter has no write method.
- **Absence is a normal state.** A missing menu-bar bridge is
  `AdapterState::Unavailable` and hides the item; a present bridge that cannot
  be read is `Error`, visible and inert with the message. Neither blocks
  session startup. `MockMenuBar` drives the states in CI, with
  `kill`/`restart` for the re-subscribe lifecycle and `push` for the
  configuration and control stream.

Rejected: placing the adapter in `apps/settings` or the shell (the T-15 track
is the services layer, and T-15.5b/T-15.8b show the pane stays shell-native
plus settingsd keys); rendering the bar or resolving the global menu in the
service (forbidden by the task and a fork of `MenuBar`/`menu-broker`);
modelling the Apple-only controls (AirDrop, Screen Mirroring) and the deferred
Search/Spotlight entry, or the richer `Clock Options...` rows the shell clock
does not render yet (they would ship as dead controls).

## Consequences

- `dragonfruit-menubar-adapter` is a workspace member whose only dependency is
  the adapter contract. CI drives it with `MockMenuBar`; no shell, compositor,
  Wayland socket, or hardware is involved. 29 unit tests in-crate plus
  `services/menubar-adapter/tests/read_path.rs` (8 integration tests) are run
  by `make e2e` (`cargo test -p dragonfruit-menubar-adapter`).
- The concrete bridge source is the shell's menu-bar client, which is C++:
  T-15.9b wires the pane and tile (shell-native plus settingsd keys, per the
  Mission Control and Lock Screen precedents) and declares the `menu.*` /
  clock keys this adapter names. Until then the live path is the seam the
  bridge fills; the mock is the tested path.
- A later task that wants to *apply* a mode or a toggle must write the settings
  key and let the shell apply it, not add a write to this adapter.