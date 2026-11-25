# ADR 0352 — App index desktop visibility filters

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T341
- **Supersedes:** none

## Context

The app index scanned every `.desktop` entry and published all of them through
`Enumerate`, which feeds the Applications drawer, the Add Application picker,
and the Dock's default pins. A GNOME-only entry therefore appeared in
Dragonfruit's launcher, and an entry whose `TryExec` program was not installed
still looked launchable. The desktop-entry spec defines `OnlyShowIn`,
`NotShowIn`, and `TryExec` for exactly this purpose.

The index is also the single identity owner (ADR 0086): a running window must
keep resolving even when its launcher entry should not be offered.

## Decision

1. `AppRecord` parses `OnlyShowIn` and `NotShowIn` as semicolon-separated
   desktop-name lists, and `TryExec` as the raw program string. An empty
   `TryExec` is not a filter.
2. `AppIndex` carries the current desktop names from `XDG_CURRENT_DESKTOP`
   (colon-separated, lowercased) and the `PATH` used for a relative `TryExec`.
   `from_dirs` reads the process environment; `from_dirs_with` takes both
   explicitly for tests and nested runs.
3. Visibility is evaluated once at scan time. `visible_records()` keeps an
   entry when a non-empty `OnlyShowIn` names one of the current desktops, its
   `NotShowIn` names none of them, and a non-empty `TryExec` resolves as an
   executable (directly when absolute, otherwise on `PATH`). With no
   `XDG_CURRENT_DESKTOP`, `OnlyShowIn` entries hide and `NotShowIn` entries
   stay visible.
4. `Enumerate` (and the `--enumerate` debug command) publish
   `visible_records()`. `Resolve`, `ResolveWindow`, and `Lookup` still search
   the full identity corpus, so a running hidden app resolves for window
   identity, menus, and icons.
5. `NoDisplay`/`Hidden` are unchanged: they stay in the catalog with their
   flag and the shell's launcher filter still removes them.

## Consequences

- The drawer, picker, and Dock default pins only offer entries intended for
  Dragonfruit and actually installed; no shell-side duplicate rule is needed.
- Identity resolution is not coupled to launcher visibility, so hiding an
  entry never turns a running window into an unknown app.
- `refresh` re-evaluates `TryExec` against the same `PATH`, so installing or
  removing the program emits the normal install/update events and updates the
  drawer.
- The rules are unit-tested against fixture `.desktop` files: GNOME-only,
  `NotShowIn`, missing/empty/absolute/path `TryExec`, the no-desktop case, and
  the preserved `NoDisplay`/`Hidden` behavior.
