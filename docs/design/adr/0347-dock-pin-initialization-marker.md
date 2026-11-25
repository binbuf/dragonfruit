# ADR 0347 — The Dock's pin seed is an explicit first-run marker

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T336
- **Supersedes:** none

## Context

The shell seeded the installed default Dock pins whenever `dock.pinned` was
empty. The schema default is an empty list, so "never seeded" and "the user
unpinned every app" were the same value. Clearing the Dock was therefore undone
at the next login, and no other persisted fact distinguished the two states.

The pin list must stay a plain ordered `as` list: the Dock, the app picker, the
external-drop path, and the drag reorder all write the complete set through one
settingsd key (ADR 0090), and adding a second structural value to that list
would leak into every consumer.

## Decision

Revision 39 adds `dock.pinsInitialized` (bool, default `false`, owner and
consumer `shell/Dock`). It is the only first-run signal:

1. The shell resolves the default pins only while the marker is false and the
   pinned list is empty. An empty list with the marker set is a durable user
   choice and is never re-seeded.
2. Every shell write of the complete `dock.pinned` set also sets the marker, so
   clearing the Dock during the session cannot be undone by a later app-index
   resync or the next login.
3. The settings migration sets the marker when an older persisted file already
   names `dock.pinned` — even as `[]` — because that file has been through a
   seed or a user edit. A file that never named the key is a first run and
   leaves the marker false for the shell to seed once.

The decision is a pure function in `shell/src/dockmodel.*`
(`dockPinSeedAction`), unit-tested by `tst_dockcore`; the migration is covered
by the settingsd persistence tests. `scripts/settings-consumers.json` records
the marker's consumer because it is a live (`stored_only: false`) key.

## Consequences

- Unpinning every app survives a shell restart and a re-login.
- A fresh install seeds exactly once; a legacy install with pins gains only the
  marker, and one whose Dock was already empty keeps it empty.
- Any future "reset the Dock" action must clear `dock.pinned` **and** leave the
  marker set; only an explicit reset that wants the defaults again may clear
  the marker.
