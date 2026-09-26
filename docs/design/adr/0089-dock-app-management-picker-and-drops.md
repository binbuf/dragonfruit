# 0089 — Dock app management: the Add Application picker and drop identity

## Status

accepted

## Context

The Dock could only gain a pinned app through the "Keep in Dock" context menu
of an already-running app, or by dragging a `.desktop` file from somewhere
else, and removal required the context menu or a drag-out. There is no surface
that lists installed applications, so a user with an empty Dock — or anyone on
a distro whose apps are not already pinned — has no in-product way to add
anything. `app-index` now owns the `.desktop` corpus, themed icons, and
identity (ADR [0086](0086-app-index-identity-ownership.md)); T-14.1a/T-14.1b
load it into the shell and T-14.1c adds coalesced install/uninstall/update
subscriptions (ADR
[0087](0087-app-index-events-launch-registry-recency.md)). Launchpad and the
Spotlight-equivalent search are explicitly post-gate
([../ROADMAP.md](../ROADMAP.md), "Explicitly deferred").

## Decision

- **The Dock owns an Add Application picker.** A divider-menu entry opens an
  overlay popover listing installed applications from the shell's app-index
  corpus (`m_index`): themed `iconPath`, name, and pinned state, with a
  design-system `SearchField` filter. The list/filter/sort/dedupe logic is pure
  and headless-testable; the Dock never scans `.desktop` directories (the
  interim scanner is deleted by T-14.7).
- **One surface toggles membership.** Selecting a row pins an unpinned app or
  removes a pinned one. `dock.pinned` stays the only persisted state and
  settingsd the only writer; the existing Keep/Remove context-menu items
  remain. The picker manages Dock membership only — it is not a launcher.
- **Filtering is corpus-safe.** `noDisplay` entries are excluded (app-index
  already folds `Hidden` into it). `OnlyShowIn`/`NotShowIn`/`TryExec` are not
  parsed by app-index today and are recorded as a follow-up rather than
  half-implemented here.
- **Freshness rides app-index.** The shell subscribes to the T-14.1c coalesced
  change signal and reloads the corpus; if the service is absent the picker
  shows an explicit "Application index unavailable" state rather than an empty
  panel.
- **Drops show identity.** An external drag reads its payload at drag *enter*
  (a data offer is readable once), so the open gap and the hovered targets
  display the dragged app's real name and icon. Drop resolution and actions
  stay with `dockdrops` and the existing shell paths (open with app, Trash,
  Downloads); app-alias drops pin through `dock.pinned` exactly like the
  picker. Failed operations raise the Dock's existing notice instead of a log
  line.

## Consequences

- Any installed app can be added or removed in two clicks, on any distro whose
  apps app-index enumerates (including Flatpak exports); running-state is no
  longer a precondition for a Dock entry.
- The Dock's contents and the picker cannot disagree: both read `dock.pinned`
  and the app-index corpus.
- A session without app-index degrades to a clear absence state; there is no
  second identity source to drift from ADR 0086.
- The identity ghost adds one payload read per drag, cached and consumed at
  drop instead of re-requested.
- Spotlight-equivalent search and launching remain post-gate; if that lands
  later, the picker's corpus/filter helper is the natural reuse point.