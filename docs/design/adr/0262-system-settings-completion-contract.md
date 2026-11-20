# 0262 — System Settings completion is one measured, consumer-backed contract

| | |
|---|---|
| **Status** | accepted (target) |
| **Date** | 2026-11-14 |
| **Supersedes** | none; extends ADR 0035/0164/0225/0231/0232/0233/0236/0239 |
| **Companions** | [24-system-settings.md](../24-system-settings.md), [08-settings.md](../08-settings.md), [settings-business-logic-map.md](../settings-business-logic-map.md) |

## Context

The first System Settings pass (T222–T229a) landed breadth: 28 shipped
panes, real host adapters, an executable row map, live apply/validation, and
an absence matrix. The 2026-11-14 pane-by-pane review found that breadth did
not become a top-tier experience:

- absent adapters collapse whole panes to one note;
- many shipped controls write keys no process consumes;
- adapter write failures are invisible and controls stay optimistic;
- the shell is 800×640 with a flat 192 pt sidebar and no groups;
- the the reference design row inventory is only partly present, especially in
  Accessibility, Appearance, General, Desktop & Dock, and Displays;
- shared components have a row binding loop, clipped selects/dialogs,
  missing focus rings, and incomplete accessibility names.

The existing `shipped` flag means “a body exists”, not “the pane is
top-tier”. The release needs one contract that distinguishes the two.

## Decision

1. **System Settings is the first program in the `system-settings` task set.**
   T263–T284 run before the remaining menu-bar, Dock, window-management,
   Search, HUD, and distro work in `0.2.0`. T222–T229a remain the landed
   foundation; they are not re-done.
2. **The measured window and IA are fixed.** 740×820 pt target, 52 pt
   unified chrome, 232 pt grouped sidebar, 508 pt detail content, 30 pt
   content inset, General as the default pane, last-pane/geometry
   persistence, and the split unified toolbar.
3. **Storage is General → Storage.** The `storage` id and
   `settings://pane/storage` deep link remain as an alias. The top-level
   Storage row is removed. VPN is an optional Connectivity entry when a
   profile exists. Online Accounts stays visible with an absent-service
   state.
4. **A row is real only with an owner, a consumer, and an absence path.**
   A durable preference with no consumer is `stored-only` in the schema and
   map and is not described as live. A shipped control may not be a fake
   toggle.
5. **No blank panes.** An absent provider keeps the pane header, section
   titles, settingsd-backed rows, and disabled host rows with reasons. A
   present-but-unreadable provider is visible, inert, and named.
6. **One component contract.** `SettingsRow`/`SettingsGroup` own the row and
   card anatomy; `Select`, `ContextMenu`, and `Dialog` live in the window
   overlay layer; every control has a focus ring, accessible name, current
   value, and range/allowed values; long strings, RTL, and 1×/1.5×/2× are
   tested.
7. **Search is truthful.** Pane ids and colloquial names are indexed; results
   are grouped with a per-pane cap; a result that cannot be landed is not
   indexed; technical owner/key terms do not dominate the user's words.
8. **Evidence is a matrix, not one still.** Every pane has light/dark,
   absence, reduced-motion, increase-contrast, keyboard-focus, and
   1×/1.5×/2× evidence plus a golden diff. Real-host rows are separate.

## Consequences

- `SettingsPanes.qml`/`SettingsMap.qml` gain group metadata and the
  `stored-only`/consumer contract; `tst_settings_map` fails on a row that
  names a missing key or a consumer-less live row.
- `SettingsWindow.qml`/`SettingsShell.qml` move to the measured window,
  grouped sidebar, General default, and persistence.
- Pane tasks T269–T281 must either consume their keys or mark them
  `stored-only`; T282 hardens the daemon and adapter error path.
- T283/T284 produce the evidence and real-host matrix; T260–T262 remain the
  release-wide close-out.
- The existing “done” task notes describe the first pass. A pane is not
  release-ready until its T263–T284 owner says so.

## Alternatives considered

- **Keep the 800×640 window and flat sidebar.** Rejected: it is not the
  measured reference and the sidebar cannot reach 28 panes.
- **Ship every macOS row as a disabled stub.** Rejected: a disabled row
  without an owner is still a dead affordance; the review requires an owner
  or an explicit omission.
- **Hide panes when an adapter is absent.** Rejected: it hides the
  settingsd-backed half and breaks macOS familiarity.
- **Keep the current single-theme captures.** Rejected: they cannot prove
  the absence, contrast, scale, or focus states.

## References

- [24-system-settings.md](../24-system-settings.md)
- [08-settings.md](../08-settings.md)
- [settings-business-logic-map.md](../settings-business-logic-map.md)
- [23-macos-structural-surface-review.md](../23-macos-structural-surface-review.md)
- [v0.2.0/ROADMAP.md](../../v0.2.0/ROADMAP.md)
- [system-settings roadmap](../../tasks/system-settings/plan/ROADMAP.md)
- ADR 0035, 0164, 0225, 0231, 0232, 0233, 0236, 0239
