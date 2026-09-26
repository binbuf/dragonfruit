# 0075 — Settings and GlobalShortcuts portal projection

## Status

accepted

## Context

T-13.1b adds the first concrete portal interfaces to the backend T-13.1a
registered. Both are read-through projections of services that already exist
and whose ownership is fixed: `settingsd` is the single settings writer
(ADR [0030](0030-settingsd-schema-and-dbus-surface.md),
[0031](0031-settingsd-persistence-format-and-atomic-writes.md)), and the
compositor's `ShortcutEngine` is the single shortcut arbiter — sandboxed
applications register through the portal but never grab keys
([02-compositor.md](../02-compositor.md)). The decisions that constrain the
later T-13 interfaces (FileChooser, Screenshot, ScreenCast) are: how a portal
exposes another service's state, which namespaces the Settings portal answers,
and where a GlobalShortcuts session lives.

## Decision

- **The Settings portal projects, it never owns.** It serves
  `org.freedesktop.impl.portal.Settings` (version 2) at the standard object
  path and reads `settingsd` over `org.dragonfruit.Settings1` only: one
  `GetAll` resync at startup, then a resync on every `Changed` and on every
  `org.dragonfruit.Settings1` name (re)appearance (restart recovery). It never
  writes and never keeps a private copy of a setting; the shared
  `SettingsSnapshot` is the only state.
- **Two namespaces.** `org.freedesktop.appearance` carries the standard
  `color-scheme` (`auto`=0 / `dark`=1 / `light`=2, derived from
  `appearance.colorScheme`), `contrast` (always 0 today), and `accent-color`
  (`a(ddd)`, present only when `appearance.accent` is a concrete `#rrggbb`).
  `org.dragonfruit.desktop` exposes every settingsd key under its own name,
  read-only. An unknown namespace/key is a typed D-Bus error, not an empty
  value.
- **Absent settingsd is normal.** The snapshot seeds the appearance defaults,
  so a session with no daemon still answers; the desktop namespace starts
  empty and fills in when settingsd appears.
- **GlobalShortcuts is bookkeeping plus the standard contract.** The pure
  `ShortcutRegistry` owns sessions: `CreateSession` creates one session
  object per caller path (a duplicate path is refused), `BindShortcuts`
  replaces and returns a session's shortcuts and emits `ShortcutsChanged`,
  `ListShortcuts` returns them, and the session's `Close()` drops it and
  emits `Closed`. Response codes and signal shapes are the portal's.
- **The compositor stays the arbiter.** The portal does not install grabs and
  does not implement focus rules. A Dragonfruit diagnostic method,
  `ActivateShortcut`/`DeactivateShortcut` on `org.dragonfruit.Portal1`, raises
  the standard `Activated`/`Deactivated` for a bound session/shortcut; a later
  task drives it from the compositor's engine (which holds the same registry
  in-process via the shell bridge). It is not part of the portal contract.

## Consequences

- `model::BACKEND_INTERFACES` and `dragonfruit.portal` now list
  `org.freedesktop.impl.portal.Settings` and
  `org.freedesktop.impl.portal.GlobalShortcuts`; later T-13 tasks append
  theirs. The backend name, path, and data-file identity are unchanged.
- The D-Bus layer is thin and the value/namespace/session logic is pure and
  unit-tested; the integration test (`portal/tests/portals.rs`) drives both
  interfaces over a private bus and runs a real settingsd object beside them.
- A real `xdg-desktop-portal` routing check and feeding bound shortcuts into
  the compositor are later work (T-13.7, the shell bridge); until then the
  diagnostic activation bridge is the seam.