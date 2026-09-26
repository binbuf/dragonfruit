# 0096 — Focus-scoped accelerators and the global-menu toggle

## Status

accepted — implements the shortcut-dispatch and toggle halves of
[06-global-menu.md](../06-global-menu.md); extends ADR
[0095](0095-menu-broker-resolution-and-fixed-menu.md)

## Context

[06-global-menu.md](../06-global-menu.md) makes shortcuts **dispatched, not
merely displayed**: system shortcuts win, the focused window's menu wins among
application accelerators, and the feature is a **toggle** ("when switched off,
our first-party applications restore their local menu presentation
immediately"). T-14.2a built the broker's resolution and fixed application
menu but left accelerators and the toggle open. The compositor already owned a
focus-scoped `ShortcutEngine` and an `app_accelerator` event, but nothing
registered accelerators and the event was never emitted; the shell had no
global-menu setting.

## Decision

- **The menu-broker owns the focus-scoped accelerator table.**
  `services/menu-broker/src/accelerators.rs` parses the `shortcut` strings a
  published model carries into normalized chords and keeps one table per app.
  Only the focused app's table resolves, and a reserved system-chord set wins
  over every app. `Resolve` now carries an `accelerators` array; the new
  `Accelerators`/`FocusedAccelerators`/`Dispatch`/`SetSystemAccelerators`
  methods expose the table and a chord→action dispatch over D-Bus. Parsing is
  pure data (no `xkb`), so the whole rule is unit-tested.
- **The shell mirrors the table to the compositor; the compositor matches.**
  `df_toplevel_manager.set_app_accelerators` (protocol v7, additive) carries
  one `action<TAB>chord` per line. The compositor feeds its existing
  `ShortcutEngine`, which already resolves system → focused-app and folds the
  lowercase base keysym. A match is delivered back as the existing
  `app_accelerator` event, and the shell routes it through the **same seam** a
  menu row click uses (`ShellController::dispatchAppAction`), so a shortcut and
  a click can never diverge. The broker never installs a grab; the compositor
  stays the single key authority.
- **`menu.global` is the toggle.** A new settingsd key (schema v6, group
  `menu`, default on, owner `apps/settings`, consumer `shell/MenuBar`). When
  off the shell sets `MenuBar.globalMenuEnabled` false, which suppresses the
  focused app's exported menus; the fixed system and application menus stay, so
  the empty desktop still reads and first-party apps present their own local
  menus (the never-break-apps rule).

## Consequences

- The DBusMenu bridge (T-14.4) plugs its parsed accelerator strings into the
  same table and wire form without a second path.
- The broker's D-Bus surface is complete and tested on a private bus; the
  shell still computes its table from the menu model it already renders rather
  than reading `Resolve` over D-Bus. Wiring the shell to publish Settings'
  model and consume the broker's resolved menu (and therefore its accelerator
  table) remains the same transport follow-up T-14.2a recorded.
- An accelerator action that is not a shell-owned verb (e.g. `edit.undo`) is
  routed to the shell's dispatch seam but has no first-party app channel yet;
  the shell logs it. The compositor→shell half of dispatch is real and tested.