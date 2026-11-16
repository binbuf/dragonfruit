# 0279 — Personalization apply machine, one-stage lock, and stored notification policy

- **Status:** Accepted
- **Date:** 2026-11-15
- **Task:** T278
- **Supersedes:** none
- **Related:** ADR 0055, 0058, 0094, 0114–0116, 0130–0133, 0239, 0240

## Context

The Wallpaper, Screen Saver, Notifications, Focus, and Lock Screen panes were
the last personalization/attention surfaces below the reference information
architecture. The review found three classes of dishonesty: a wallpaper pick
could look unselected or be ignored while an apply was pending; a lock-screen
"require password after" row wrote raw inactivity seconds; and notification
preferences were presented as live although the notification service cannot
read display-sleep, locked, or mirroring state.

The Linux session idle engine also has one display-off stage, not the macOS
battery/AC pair, and the notification service owns per-app allow/style/sound/
badge but not the rest of the macOS per-app sheet.

## Decision

1. **A wallpaper pick is an optimistic, superseding, bounded state machine.**
   The first press marks the tile and starts both spinners before any fetch or
   write. The old preview stays until the new image is ready. A second pick
   replaces the first. `wallpaper.source` is written once per committed pick.
   A provider error, decode failure, or bounded timeout reverts the selection
   and shows an inline reason. The minimum dwell, apply timeout, and write
   timeout are exposed properties, with `forceMinElapsed()`/`forceTimeout()`
   seams, so tests never sleep for the production interval.

2. **The wallpaper hero is proportional to the content column.** The fixed
   160×100 thumbnail is replaced by a token-driven ratio/aspect. File URLs go
   through one `SettingsBridge::fileUrl` (QUrl) seam. The provider has four
   distinct empty reasons (absent, offline, error, empty-but-ready).

3. **The screen saver is the first-party lock scene.** The gallery previews
   the live clock/date/seconds keys. `None` disables the saver, so Start After
   and the clock options are disabled with a reason. Start After remains the
   session idle engine's one display-off stage (`idle.blank`), now with
   2 h/5 h/never and a custom duration; the pane says the display turns off
   rather than implying an animated saver engine.

4. **Notifications: store what the service cannot consume.** The four global
   presentation preferences stay settingsd keys but are marked stored-only in
   the map and the pane, because the service has no display-state input. The
   four extra per-app options (lock screen, Notification Center count,
   previews, grouping) are one JSON blob (`notifications.appOptions`) until
   the service grows those fields. The service still owns allow/style/sound/
   badge and enforces them in admission.

5. **Focus is one list and one detail.** The profile selector, mode selector,
   and inert mode rows are replaced by one radio list (Off + service profiles
   + user-added names), Add/Delete over `focus.customProfiles`/
   `focus.customProfile`, and All/Allowed/Not-allowed filters on the allowed
   apps and people. The schedule and validated shortcut stay live when the
   service is absent.

6. **Lock screen: one stage, after-display-off semantics.** Linux has one
   display-off stage, documented in the pane. `lock.requirePasswordAfter`
   (-1 never, 0 immediately) is the delay after display-off/lock; the pane
   derives `idle.lock = idle.blank + after`. Custom idle values show their
   real duration. The pane renders a live lock-scene preview and resets editor
   drafts on Cancel.

## Consequences

- One new schema revision (35): `notifications.appOptions`,
  `focus.customProfiles`, `focus.customProfile`, `lock.requirePasswordAfter`.
- The Wallpaper pane's visible state is the product: a test asserts the
  transition, not a still image's timing.
- The notification service's missing display-state input is explicit rather
  than hidden behind "live" claims. A later service change can consume the
  stored keys without changing the pane.
- The session engine still reads `idle.*` through its `from_keys` seam; the
  pane derives the lock stage from the durable after-display-off preference.
- The `make t278-personalization-capture` target writes every pane plus
  absence stills in light/dark.
