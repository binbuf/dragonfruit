# 0036 — One shared settingsd client, exposed to the Settings app as a QML singleton

## Status

accepted

## Context

T-08 gave the desktop one settings owner (`org.dragonfruit.Settings1`,
`services/settingsd`) and one C++ client seam (`SettingsClient` with a live
`DbusSettingsClient` and an in-process `MockSettingsClient`) that the shell's
Dock, design-system `Theme`, and compositor-policy forwarder all share. T-09.1b
makes the Settings app a real consumer: every Wave-1 control must write keys
through settingsd and react to its `Changed` signal, never to a local file and
never by polling.

The client had lived inside `shell/src` and was compiled into the shell's
Wayland-free `dragonfruit-shell-dockcore` library. Exposing it from the app by
compiling the same file into a second target would fork the D-Bus dialect and
the mirrored schema-default table; linking the whole shell core into a
first-party app would drag in unrelated shell machinery. The Settings app also
needs a QML-facing surface, because panes are QML and must not touch D-Bus.

## Decision

- **Extract the client into `libs/settings-client`** (static library
  `dragonfruit-settings-client`). It stays Wayland-free and QML-free. Both
  `dragonfruit-shell-dockcore` and the Settings app link it, so the shell and
  the app share one read/write/subscribe implementation and one defaults table.
- **The Settings app registers a C++ QML singleton, `Settings`**
  (`apps/settings/SettingsBridge.{h,cpp}`), over that client. It exposes the
  reactive `values` map (bindings re-evaluate on every `Changed`), the typed
  `value(key, fallback)` read, `set(key, value)`, `refresh()`, and `keys()`.
  Panes bind to `values` and write through `set`.
- **`DF_SETTINGS_FIXTURE` selects `MockSettingsClient`** for headless QML tests
  and captures (the same fixture convention as `DF_STATUS_FIXTURE`); otherwise
  the live client is used and degrades to schema defaults + in-memory writes
  when no daemon is on the bus.
- **A bound control writes on interaction and re-binds to `Settings.values`**,
  so a user edit and an external edit both converge (the pattern the T-09.1b
  test drives with a stock design-system `Toggle`).

## Consequences

- New consumers (settings panes, Files, and later first-party apps) link
  `dragonfruit-settings-client` rather than re-implementing the protocol.
- The `Settings` singleton is the Settings app's one settings surface; a pane
  never opens its own bus connection. `appearance.accent` still has no
  design-system consumer (T-09.2).
- The QML singleton name `Settings` is scoped to `Dragonfruit.Settings`; a file
  that also imports a module exporting `Settings` (e.g. `QtCore`) must qualify
  it.
- The T-09.1b acceptance test (`apps/settings/tests/tst_settings_live.cpp`)
  drives a control through a fake service and, when built, the real
  `dragonfruit-settingsd` over a private session bus.