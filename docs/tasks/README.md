# Task sets

This directory holds the executable task sequence. The main sequence lives
directly here (`001-…` through `182-…`); release-specific sets live in
subdirectories so their roadmaps and progress notes can run independently.

| Set | Tasks | Roadmap | Status |
|---|---|---|---|
| Main sequence | T01–T182 (files `001-…`–`182-…`, plus suffixed units) | [docs/ROADMAP.md](../ROADMAP.md) | mostly agent-done; hardware rail open |
| v0.1.1 presentation foundation | T183–T193 | [docs/v0.1.1/ROADMAP.md](../v0.1.1/ROADMAP.md) | in progress |
| Liquid glass | T194–T208 | [docs/tasks/glass/plan/ROADMAP.md](glass/plan/ROADMAP.md) | in progress |
| v0.2.0 total experience | T209–T262 (+ T229a, T245a) | [docs/v0.2.0/ROADMAP.md](../v0.2.0/ROADMAP.md) | proposed |

## Which set do I use?

- **New total-desktop behaviour** (Settings, menu bar/extras, Dock, Overview,
  switcher, Search, drawer, motion/HUDs, cross-distro) → the v0.2.0 set.
- **Material/glass** changes → the liquid-glass set; check its status first.
- **Presentation transport/HiDPI/refresh** → v0.1.1.
- **A defect in an existing main-sequence area** → add a task to the newest
  relevant set; do not reopen a completed legacy ticket.

## Rules

- One task = one session. A task ends with `make gates` green (or an explicit
  capture/`OPEN` artifact for capture tasks), docs updated, and the `## Hand-off`
  filled in.
- Capture tasks T209–T218 run on the macOS capture host; implementation tasks
  run on Linux.
- Any task touching Dock/menu/menus/Quick Settings/Notifications/Settings/Files
  chrome/OSD checks the liquid-glass set first and implements only missing
  glass work.
- No task may add a Settings row without an owner and an absence path
  ([settings map](../design/settings-business-logic-map.md)).
- No task may add an animation without a measured token and a reduced-motion
  path ([motion/feedback](../design/motion-and-feedback.md)).
