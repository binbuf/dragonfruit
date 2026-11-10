# 0253 — Focus profiles and allowed people are one policy presentation

- **Status:** Accepted
- **Date:** 2026-11-10
- **Task:** T234
- **Supersedes:** none
- **Related:** ADR 0058, 0059, 0061, 0130, 0131, 0239, 0252

## Context

ADR 0058 froze the one Focus/DND admission policy: three modes (`off`,
`focus`, `dnd`), an app allow list, and "suppress the banner, keep the
history". ADR 0252 left named profiles (Personal/Work/Sleep/custom) and a
person allow list as explicit `OPEN` rows because they looked like a second
policy. T234's scope requires them, and the menu bar, Control Center, and
Settings must still agree.

The freedesktop `Notify` contract has no sender field. A per-person allow list
therefore needs a hint, not a new daemon or a parallel identity store.

## Decision

1. **Profiles are a presentation over the one policy.** `FocusPolicy` keeps
   the ADR 0058 mode and admission rule. It gains a built-in catalog
   (`dnd`, `personal`, `work`, `sleep`, `custom`), each with a mode and its own
   app/person lists. `SetFocusProfile(id)` installs the profile's mode and
   lists into the same policy; an unknown id is rejected. A manual mode change
   selects the synthetic `custom` profile, and `off` clears the active profile.
   Editing a list while a named profile is active edits that profile.
2. **`custom` is a marker, not a user-picked name.** The presentation label is
   the named profile when one is active, otherwise the mode label, so Control
   Center and the menu bar never show a bare "Custom" after a manual toggle.
3. **Allowed people ride an optional sender hint.** `Notify` reads `sender`
   or `x-dragonfruit-sender` from its hints (unknown hints stay ignored).
   `allowedPeople` is matched case-insensitively, and an allow-listed person is
   admitted in `focus` and `dnd` exactly like an allow-listed app. No person
   field is invented on the freedesktop signature; apps that cannot name a
   sender simply never match.
4. **One shared view.** `FocusPolicy()` gains `profiles`, `activeProfile`,
   `activeProfileLabel`, and `allowedPeople`; the notify adapter and status
   host project them into the Settings view (`profile`, `profiles`, `people`).
   The menu bar, Control Center, and Settings read the same view; no surface
   stores Focus state.

## Consequences

- The admission rule is unchanged and still testable without a bus; profiles
  and people are additive fields.
- A service that serves no profile catalog still reads as a valid three-mode
  policy; the pane shows the mode selector without named chips.
- A person can only be allow-listed after an app has sent a notification with
  the hint, which is the same discovery rule as the observed app list.
- Scheduling, per-app presentation preferences, and the profile catalog's
  future persistence remain additive; a settingsd-backed catalog is not
  required for this task.
