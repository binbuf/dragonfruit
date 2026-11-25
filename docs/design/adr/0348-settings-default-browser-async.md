# ADR 0348 — Settings default-browser reads are asynchronous and bounded

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T337
- **Supersedes:** none

## Context

The Desktop & Dock pane's default-browser row read app-index `Enumerate` with a
blocking `QDBusReply` and ran `xdg-settings` with `waitForFinished`. The
Settings app is a normal Wayland client, so a slow or hung app-index or
default-apps delegate froze the whole window. The row already had a durable
`defaults.browser` key and a truthful absence state, but no loading state and
no bound on either source.

## Decision

`SettingsBridge::refreshDefaultBrowsers()` is one asynchronous, coalesced read
of two independent sources:

1. app-index `Enumerate` goes through `QDBusPendingCallWatcher` on
   `asyncCall` with a 5 s bound. Only visible `WebBrowser` records become
   choices.
2. the freedesktop default-apps delegate runs through `QProcess` signals with
   a 1.5 s read timeout and a 2 s write timeout. A timeout kills the child; it
   is deleted only after `finished` so no process outlives the bridge. No
   `waitForFinished` runs on the GUI thread.

The row exposes `defaultBrowserLoading`. While a read is in flight it keeps
the last known choices and value and shows a loading description; a second
refresh coalesces into one follow-up read. A failed or timed-out read keeps the
last known host default. A rejected write sets `defaultBrowserError` and keeps
the persisted key. An absent app-index is a definitive absence (empty choices,
no spinner); a missing delegate is an absence of the host default, not an
error.

## Consequences

- Opening Desktop & Dock can never freeze the Settings window on app-index or
  the host delegate. The cost is that the row may briefly show its previous
  choices; that is the intended "keep the last known value" behavior.
- `DF_SETTINGS_FIXTURE` selects a deterministic `ready` mode and adds a
  `setDefaultBrowserFixture()` seam (`slow-index`, `slow-subprocess`,
  `timeout-subprocess`, `absent`) so the QML tests can hold the row in each
  state without a bus or a real default-apps tool.
- The app-index worker and the `defaults.browser` write path are unchanged.
  A future default-apps pane reuses the same async delegate rather than adding
  a second blocking reader.
