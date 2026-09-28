# 0133 — The Lock Screen pane and tile are shell-native plus settingsd keys

- **Status:** Accepted (T-15.8b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settingsd-live-settings.md](../tracks/08-settingsd-live-settings.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md),
  ADR [0127](0127-mission-control-pane-and-tile.md),
  ADR [0132](0132-lock-screen-policy-adapter.md)

## Context

T-15.8a shipped `dragonfruit-lock-adapter`, a read-only projection over the
session idle/lock engine and the compositor lock. Like Mission Control
(T-15.5b), this subsystem has **no external daemon**: the session owns the idle
timing (`idle.*` keys, ADR [0070](0070-idle-timer-engine-and-policy.md)), the
compositor owns the one fail-secure lock (ADR [0067](0067-session-lock-protocol-and-ui.md)),
and the shell is the only process that speaks to the compositor over the
private `df_toplevel_manager` bridge. A `dragonfruit-system-status` process
cannot read a Wayland client's private socket, so the T-15.8a consequences note
anticipated "shell-native plus settingsd keys".

The durable home for the lock-screen display options is principle 3's existing
split: `settingsd` owns the preference, the session/compositor applies it live.
The timing half already exists as the revision-5 `idle.blank`/`idle.lock` keys,
so the pane reuses them rather than inventing a second spelling.

## Decision

The pane and the Control Center tile are one functional unit, and both are
shell-native where the runtime is concerned.

1. **Settingsd keys, revision 15.** A new `lock` group declares
   `lock.showUserNameAndPhoto`, `lock.showPasswordHints`,
   `lock.showMessageWhenLocked`, `lock.message`, and `lock.showPowerButtons`;
   the four toggle suffixes are the T-15.8a adapter's `LockDisplayOption::id()`
   spellings, and the defaults mirror `LockDisplay::default()`. The pane binds
   the timing rows to the existing `idle.blank`/`idle.lock` keys, which the
   session idle engine already applies live (`IdlePolicy::from_keys`).
   `docs/settings-keys.md` documents every row and the schema-doc test keeps the
   table honest.
2. **The pane writes keys, never the adapter.** `apps/settings/LockScreenPane.qml`
   has a display-off popup, a require-password popup, the four display toggles,
   and a `Set...` dialog for the custom message. Every row applies live and
   persists. The compositor's lock hot path is never touched.
3. **The tile is projected by the shell.** No services-layer host is added.
   `shell/src/controlcenterpolicy.cpp::lockPolicyView` maps the `idle.lock`
   value onto the tile view (`Password after <duration>` / `No password
   required`) and the `lock` glyph. It is pure and unit-tested by
   `tst_controlcenterpolicy`; the QML tile renders it and raises
   `lockScreenSettingsRequested`.
4. **Absence is a missing settings daemon.** With no daemon the pane's rows
   stay live on the schema defaults and it shows a one-line note; there is no
   second `present` hide rule because the shell path has no adapter view.
5. **New design-system glyph.** `lock` joins `Icon.qml`, used by the Lock
   Screen sidebar row, the tile, and (later) the lock screen itself.

Rejected: a `dragonfruit-system-status` lock host (no honest live source); a
shell→D-Bus push bridge (inverts ownership and duplicates the compositor's one
lock mirror); driving the lock hot path from the pane (the compositor owns it).

## Consequences

- The Control Center panel grows from 360×1040 to 360×1120 to fit the new tile;
  the fit test and the T-15.8b capture assert it.
- The four `lock.*` keys persist and apply live in settingsd, and the timing
  keys already drive the session engine, but the lock-screen renderer does not
  yet read the display options to hide the name/photo, hints, message, or power
  buttons. A later task wires `lock.*` into `shell/lock/LockScreen.qml` (and the
  shell subscription) and extends `df_toplevel_manager` only if the compositor
  needs it — the preference is the single source of truth until then.
- The pane drops the Apple-only rows the capture shows: `Login window shows`
  (no login-window provider) and `Accessibility Options...` (the Accessibility
  pane is not shipped, so the link would be a dead control), plus the
  project-wide Apple Account/sidebar row and `?` help
  (ADR [0122](0122-tahoe-interface-language-across-chrome.md)). The battery/AC
  display-off pair is the single `Turn display off when inactive` row, the
  Linux adaptation ADR [0132](0132-lock-screen-policy-adapter.md) fixed.