# 0086 — Application identity is owned by `app-index`; the compositor publishes raw identity

## Status

accepted

## Context

Application identity (`app_id` → `.desktop` entry, icon, name) is owned by
`app-index` ([01-architecture.md](../01-architecture.md)), but until T-14.1a
the real resolution lived in two interim places: the compositor carried a
pure-std `.desktop` resolver (`compositor/src/identity.rs`, T-06's "interim
application-identity resolver") that mapped Xwayland `WM_CLASS` to a desktop
id, and the shell carried its own `.desktop` scan/launch resolver
(`shell/src/desktopentry.{h,cpp}`) for the Dock. Neither owned themed icons.
ADR [0034](0034-compositor-policy-via-shell-bridge.md) already establishes
that the compositor opens no D-Bus connection and the shell is the sole
policy forwarder.

## Decision

- **The compositor resolves nothing.** It publishes the raw window identity
  over the private protocol: the Wayland `app_id`, or the X11 `WM_CLASS`
  class (falling back to the instance). `compositor/src/identity.rs` is
  deleted; `DfState` has no app resolver and emits no identity stats.
- **`app-index` owns the resolution pipeline and themed icons.** The pure
  engine in `dragonfruit-app-index` scans the XDG/Flatpak application
  directories and resolves in order: exact desktop id, `StartupWMClass`,
  desktop id/stem/reverse-DNS tail, `Exec` executable basename, then a
  tolerant normalized heuristic. Every miss is recorded; the miss set is the
  heuristic input and is served by `Misses`.
- **Icons resolve to files.** `icons` implements the freedesktop icon-theme
  lookup (`$DF_ICON_THEME` or GTK `gtk-icon-theme-name` or `hicolor`, with
  `Inherits`, then `pixmaps`) and returns a path; results are memoized and
  directory listings cached, because `Enumerate` resolves every entry.
- **The bus surface is `org.dragonfruit.AppIndex1` at
  `/org/dragonfruit/AppIndex1`**, with flat JSON replies: `Resolve`,
  `ResolveWindow`, `Lookup`, `Enumerate`, `Misses`, `IconPath`, `Stats`.
  Install/update events, the launch registry, recency, and subscription are
  T-14.1b/T-14.1c; methods are additive-only.
- **The shell consumes it.** `AppIndexClient` loads the Dock's identity
  corpus via `Enumerate` (falling back to the legacy local scan only when the
  service is absent; that fallback is deleted in T-14.7). The Dock renders
  the returned `iconPath`; SVG icons go through `QtQuick.VectorImage`, since
  the toolchain ships no svg imageformat plugin.

## Consequences

- One owner for identity and icons: the compositor, Dock, and (later)
  menu-broker/switcher all read `app-index`, and the miss set has a single
  home. A raw X11 `WM_CLASS` reaches the shell unresolved and is resolved
  there; grouping still works because the raw class is a stable key.
- `app-index` is restartable and the shell re-queries on startup; the
  subscription API that removes the startup re-query is T-14.1c.
- The pure engine is unit-tested against a fixture `.desktop` corpus and the
  D-Bus surface against a private `dbus-daemon` (`services/app-index/tests/
  session_bus.rs`), covering the Wayland and X11 fixtures and themed icons.
- The shell's local `.desktop` scan and launch-command code still exists for
  launch semantics and as the fallback; T-14.7 deletes it once the launch
  API lands.