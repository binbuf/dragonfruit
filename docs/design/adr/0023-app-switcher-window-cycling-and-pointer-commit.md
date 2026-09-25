# 0023 — Within-app Cmd+` cycling extends the switcher machine; the compositor commits on a preview press

## Status

accepted

## Context

ADR 0021 put the Cmd-Tab switcher in one compositor-owned `AppSwitcher`
machine and explicitly deferred "direction-aware within-app Cmd+` window
cycling (T-06.2b)" to this task, with the rule that it "must not add a parallel
binding path outside the shortcut engine". ADR 0022 put the live previews in
the compositor and the card chrome in a shell `app-switcher` overlay surface,
and flagged that the entry event carries no window handle. T-06.2b must commit
the selection (the task text names `activate_app`, cross-Space and
restore-if-minimized), cycle windows with Cmd+`, and stay interruptible — by
keyboard and pointer.

The shell overlay is an offscreen QML scene read back as an image, so it never
receives pointer events; pointer interaction must be resolved by the
compositor that owns the scene and the machine.

## Decision

- **One binding path.** Cmd+` / Cmd+Shift+` are system shortcuts in the one
  `ShortcutEngine` resolving to `InputAction::AppSwitcherWindow`; the key filter
  calls `DfState::app_switcher_window_key`, which drives the existing
  `AppSwitcher`. No new machine and no client grab.
- **The machine owns the window cursor.** `SwitcherApp` carries all of the
  app's windows, most-recent first; `AppSwitcher` adds a `window_cursor` and
  `step_window`, wrapping and reset on an app step. `commit` returns the
  cursor-selected window. With the switcher closed, Cmd+` seeds the snapshot on
  the focused app (`open_focused`) and takes one step, so a bare Cmd+` switches
  the frontmost app's windows. The app selection (the overlay highlight) never
  moves.
- **Previews follow the cursor.** `switcher_candidates` previews the
  cursor-selected window of the selected entry, so the live surface swaps with
  Cmd+` without a protocol change (the shell still draws only cards).
- **Commit reuses app activation.** The app-level default commits through
  `DfState::activate_app`, the same most-recent-window resolver the Dock's
  `activate_app` request uses; a cycled window is activated by id through the
  same `activate_window_id`. No second activation path.
- **Pointer commit is compositor-side.** While the switcher is active a left
  press is intercepted before chrome routing; `switcher_window_at` hit-tests
  the live preview rects, a hit selects and commits that entry
  (`app_switcher_commit_window`), and a miss cancels. The shell needs no
  pointer forwarding.

Rejected: a standalone `Cmd+`` window list on the shell (would need a second
recency source and window handles on the wire); adding the window handle to
`app_switcher_entry` (not needed — the compositor resolves the previews); a
shell `MouseArea` path (the overlay is an offscreen image, not a live input
surface).

## Consequences

- Later tasks (T-07 chrome, T-14 identity) consume the same `app_switcher` +
  `app_switcher_entry` projection; `query switcher` gains `window` and
  `window-direction` fields and the entry line gains a window count, all
  additive.
- If a future shell overlay needs its own pointer handling, it must route
  through the machine rather than activate directly.
- Multi-window previews still show one live surface per app (the cursor
  selection); per-window cards remain a later polish item.