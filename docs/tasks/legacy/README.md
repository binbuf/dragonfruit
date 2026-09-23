# Legacy task archive (superseded)

These are the original phase-based tickets (T-01…T-35) and their index. They
are **historical reference and detailed specs**, not the current plan. The
current plan is the vertical-slice sequence in [`../../ROADMAP.md`](../../ROADMAP.md).

Why the plan was re-cut: the phase plan produced a great deal of verified
machinery (compositor core, input, windows, Spaces, Xwayland, private
protocols, design system, menu bar, Dock) while deferring the user-facing
affordance layer (SSD titlebars, lifecycle animations, materials) and the live
chrome (adapters, settings, apps). That made "done" tickets impossible to
experience end to end and pushed the first honest product review to the far
end of a long dependency chain. The new plan sequences the *same underlying
design* as vertical slices, each ending in a human-verifiable demo.

## How to use this folder

- **Detailed scope still lives here.** The new slice tasks link to the legacy
  ticket(s) that specify the implementation detail. Do not duplicate the
  detail in the new task files; link to it.
- **Ticket numbers collide by design.** A reference to "T-13" in a legacy
  ticket, in the design docs, in `docs/PROGRESS.md`, or in source comments
  means *legacy* T-13. The new plan restarts at T-01; the mapping table in
  [`../../ROADMAP.md`](../../ROADMAP.md) is the translation.
- **Comments in source code** that cite `docs/tasks/NN-*.md` refer to the
  files in this folder (for example `docs/tasks/legacy/02-compositor-core.md`).

## Status of the legacy work

The status recorded when the plan was re-cut (2026-09-23):

| Legacy ticket | Status | New home |
|---|---|---|
| T-01 scaffolding/CI/nested workflow | done | inherited foundation |
| T-02 compositor core | partial — nested verified; DRM unrun; budgets unmeasured | T-03 (real session), T-16 (perf) |
| T-03 input stack | partial — engine + synthetic harness done | inherited foundation; hardware validation in T-03/T-16 |
| T-04 window model | partial — model + headless conformance done | inherited foundation |
| T-05 Spaces | partial — model done; image wallpaper + slide open | T-04/T-05 |
| T-06 Xwayland | partial — eager start done; XDnD missing; app matrix unrun | T-14 (XDnD/zoo), T-03 (DRM) |
| T-07 private protocols | partial — served; chrome rendering is T-09/T-10 | inherited foundation |
| T-08 design system | done | inherited foundation |
| T-09 menu bar | partial — render/interaction done; content stubbed | T-01 (app name/Quit), T-07 (status), T-14 (broker) |
| T-10 Dock | done at the model level; user-facing loop unreachable without decorations | inherited foundation; loop affordances in T-01/T-02 |
| T-11 Mission Control | partial — one state machine + chrome; live transform blocked on T-33 | T-05 |
| T-12 app switcher | not started | T-06 |
| T-13 SSD decorations | not started | T-01 (functional), T-04 (materials) |
| T-14 hot corners/desktop | partial — hot corners done; Desktop Reveal shares T-11 | T-05 |
| T-15 settingsd | not started | T-08 |
| T-16 Settings app | not started | T-09, T-15 |
| T-17/T-18 files-core/Files | not started | T-10 |
| T-19 desktop icons | deferred | post-gate backlog |
| T-20 system adapters | not started | T-07 (MVP slice), T-15 (rest) |
| T-21 Control Center | not started | T-11 |
| T-22 menu broker | not started | T-14 |
| T-23 app-index | not started | T-14 |
| T-24 session lifecycle | not started | T-12 |
| T-25 notifications/OSD | not started | T-11 |
| T-26 lock/idle | not started | T-12 |
| T-27 portals | not started | T-13 |
| T-28 screenshot/recording | not started | T-13 |
| T-29 clipboard/auth | not started | T-13 |
| T-30 compatibility bridges | not started | T-14 |
| T-31 polish/hardening | not started | T-16 |
| T-32 packaging | not started | T-16 |
| T-33 effects/materials | not started | T-04 |
| T-34 MVP gate | not started | T-17 |
| T-35 lifecycle animations | not started | T-02 |
