# ADR 0239 — Wallpaper links, per-app notification preferences, Focus schedule, and lock-screen observation

- **Status:** Accepted
- **Date:** 2026-11-08
- **Task:** T229a
- **Supersedes:** none
- **Related:** ADR 0055, 0058, 0067, 0070, 0094, 0116, 0130–0133, 0233, 0238

## Context

The Wallpaper, Notifications, Focus, and Lock Screen panes were the remaining
first-party Settings surfaces below the reference information architecture.
The map declared rows the panes did not render (the wallpaper Screen Saver /
Clock Appearance links, per-app notification presentation, the Focus people /
schedule / shortcut rows, the login-window and accessibility rows), and the
`lock.*` keys were stored but never observed by the lock scene.

The notification service owned the Focus mode and allow list but no per-app
presentation preference. The Focus schedule had no owner at all, and the
Lock Screen pane's power buttons had no consumer.

## Decision

1. **Wallpaper links navigate; they do not duplicate Screen Saver.** The
   Wallpaper pane's Screen Saver… and Clock Appearance… rows emit
   `navigateRequested("screen-saver")`; the Screen Saver pane remains the one
   owner of `screenSaver.*` and the lock clock options.
2. **Per-app notification preferences live in the notification service.**
   `FocusPolicy` gains an app-preference map (`allowed`, `style`, `sound`,
   `badge`), served in `FocusPolicy()` as `appPreferences` and written through
   `SetAppPreference(app, style, sound, badge, allowed)`. The service enforces
   the mute and `style = none` in its admission rule; the shell reads the same
   view. The Notifications pane's per-app sheet writes one preference per
   interaction and re-seeds from the pushed view. The Focus allow list stays a
   separate concept: it only governs admission while Focus/DND is active.
3. **The Focus schedule is a settingsd policy with one shell consumer.**
   `focus.schedule.enabled/start/end/days` are schema revision 23 keys.
   `shell/src/focusschedule.cpp` evaluates the window (including an overnight
   start and the weekday set) on a 30-second timer, on settings changes, and on
   service availability. It turns Focus on at the opening boundary and Off at
   the closing boundary only when it turned it on; a manual mode change wins
   until the next window. The shortcut key (`focus.shortcut`) is validated and
   stored; global registration is deferred with the shell shortcut engine, the
   same contract as `search.shortcut`.
4. **The lock scene observes `lock.*`; the pane never enforces.** The shell
   pushes the display policy onto `Dragonfruit.Lock/LockScreen` (identity,
   password hint, lock message, power buttons). The power row calls logind
   through one shell path shared with the system menu. The pane's Login window
   row is disabled with the display-manager reason; Accessibility Options
   navigates to the Accessibility pane.

## Consequences

- One new schema revision (23) and two new families of keys; additive only.
- The notification service, notify adapter, status host, Settings bridge, and
  pane all carry the per-app preference in one shape. A future shell banner
  sound/badge consumer reads the same fields.
- The lock screen gains a real power path (logind) and stops ignoring the
  `lock.*` preferences; the compositor remains the fail-secure owner.
- Focus schedule enforcement is testable without a bus (`windowActive`), and
  the pane's absence path keeps the schedule/shortcut rows live on schema
  defaults.
