# 0021 — The app-switcher state lives in one compositor-owned machine

## Status

accepted

## Context

T-06 makes Cmd-Tab real: the compositor must open, cycle, reverse, commit, and
cancel an app switcher whose recency order and activation semantics already
exist in `WindowModel` (`recency`, `touch_recency`) and
`DfState::activate_window_id` (cross-Space, restore-if-minimized). Before this
task the private protocol carried `cycle_app_switcher` and the `app_switcher`
event, but the state was a shell-shaped `AppSwitcherState` on
`ShellProtocolState` and the shortcut engine's `AppSwitcher` action fell through
to a no-op: the chord was declared but nothing resolved it into state. T-06.2a
(overlay) and T-06.2b (commit/cycling) both need one authority for the
selection so the overlay can never diverge from the activation.

## Decision

- The state is one pure machine, `app_switcher::AppSwitcher`, on `DfState`
  (not on the shell protocol state). It holds an app-recency snapshot with each
  app's most recent window, the selected index, and the last direction; the
  rules are unit-testable without a `Window` or a GPU.
- The **input path owns the chord**, via the shortcut engine: Cmd+Tab and
  Cmd+Shift+Tab resolve to `InputAction::AppSwitcher`, and the filter derives
  the direction from the held Shift role. The switcher consumes its own keys
  (Tab, arrows, Escape) and their releases while open; a Command release
  commits exactly once because `commit` clears the active state before
  returning.
- `broadcast_app_switcher` projects the machine onto the existing
  `df_toplevel_manager.app_switcher` event; the shell's `cycle_app_switcher`
  request drives the *same* machine with the shell trigger (cycling never
  focuses; a commit does).
- The first selection steps one app away from the focused app so a release
  switches (macOS behavior); with no focused window or a single app it selects
  the most recent.

Rejected: keeping the selection on the shell, which would let a shell restart
or a stale overlay activate the wrong app; and a second discrete "instant"
commit path outside the input engine.

## Consequences

- T-06.2a/b consume the `app_switcher` projection and do not add a second
  selection or recency derivation; the overlay must render the machine's
  `selected_app` and never recompute it.
- The activation on commit reuses `activate_window_id`, so cross-Space and
  minimized-restore behavior stays in one place.
- `query switcher` (headless introspection) reports active/selected/entries/
  focus, so protocol tests assert commit and cancel without a GPU.
- Direction-aware within-app Cmd+` window cycling (T-06.2b) extends this
  machine; it must not add a parallel binding path outside the shortcut engine.