# 0058 — Focus/DND policy semantics

## Status

accepted

## Context

T-11.1a shipped one `do_not_disturb` bit in the notification service: on
suppresses every banner while still recording the history
([0056](0056-notification-service-surface-and-shell-banner.md)). The T-11
design wants three-way consistency between the menu bar, the Control Center,
and Settings later ([11-control-center-notifications.md](../tracks/11-control-center-notifications.md)),
and the legacy design names modes (off / Focus / DND) plus per-app overrides
and "suppress the banner, keep history"
([legacy/25](../../tasks/legacy/25-notifications-and-osd.md) FR-4). The
service must freeze those rules before any surface binds to them.

## Decision

- **Three modes, one owner.** `dragonfruit-notifications` owns a
  `FocusPolicy` with `FocusMode` = `off` | `focus` | `dnd` and a per-app
  allow list. The mode and the rule live in `services/notifications/src/policy.rs`;
  no other component stores Focus/DND state.
- **The admission rule** (whether a `Notify` becomes a banner):
  - `off` — every notification banners.
  - `focus` — allow-listed apps and `critical` urgency banner; everything
    else is suppressed.
  - `dnd` — only allow-listed apps banner; everything else, including
    `critical`, is suppressed.
  Matching is case-insensitive on the trimmed `app_name`; the allow list is
  the explicit per-app override and is honored in both suppressing modes.
- **Suppression never drops.** A suppressed notification is still recorded
  in the history (now flagged `suppressed: true`) and counted in the policy's
  current batch. The batch accumulates while the mode suppresses and is
  cleared when the mode returns to `off`; the service never emits a synthetic
  summary banner. The shell can show "N notifications while Focus was on"
  from `FocusPolicy().batchedCount` (T-11.2b/T-11.3b).
- **The shell-facing surface grows additively.**
  `org.dragonfruit.Notifications1` gains `FocusPolicy()` (JSON: `mode`,
  `allowList`, `batchedCount`), `SetFocusMode(mode) -> bool` (unknown names
  rejected, previous mode kept), and `SetFocusAllowList(apps)`. The T-11.1a
  `DoNotDisturb()`/`SetDoNotDisturb(bool)` pair is kept as a compat mapping
  (`true` = `dnd`, `false` = `off`), so no existing shell call changes shape.
  Each setter emits `Changed`.

## Consequences

- Settings, the menu bar, and the Control Center all bind to the same three
  names and the same rule; `docs/settings-keys.md` can add a Focus section
  without re-deciding semantics.
- Critical alerts are guaranteed to break through Focus but are silenced by
  DND; an app that needs to override DND must be on the allow list. This is
  deliberate and testable.
- `History()` entries gain a `suppressed` field; additive for the shell
  decoder.
- Scheduling Focus windows and richer per-app policies remain out of scope;
  the policy is a plain value the service can extend additively.