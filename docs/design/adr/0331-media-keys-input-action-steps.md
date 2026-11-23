# 0331 — Media keys route through the input-action stream with 5% steps

- Status: accepted
- Date: 2026-11-23
- Task: T321

## Context

The XF86 volume/mute/brightness keys did nothing. The compositor already owns
one shortcut engine that intercepts every key before the focused client, the
shell already owns the audio client and settingsd brightness writes, and the
transient OSD already presents volume/brightness changes
([0062](0062-osd-overlay.md), [0063](0063-osd-keyboard-atspi.md)). The only
missing piece was a binding and a step policy, not a new surface or a second
state owner.

## Decision

1. **Five compositor actions, empty modifiers.** `InputAction::VolumeUp`,
   `VolumeDown`, `Mute`, `BrightnessUp`, and `BrightnessDown` are bound to
   `XF86AudioRaiseVolume`, `XF86AudioLowerVolume`, `XF86AudioMute`,
   `XF86MonBrightnessUp`, and `XF86MonBrightnessDown` with `RoleMods::EMPTY`.
   They resolve in the system table before any focused application
   accelerator and travel over the existing `input_action` event as
   `volume-up`, `volume-down`, `mute`, `brightness-up`, and `brightness-down`.
2. **One 5% step per press; no repeat.** Volume and brightness each move by
   0.05 (snapped to hundredths so repeated presses do not accumulate float
   noise) and clamp to `[0, 1]`. The input filter already ignores auto-repeat
   of an intercepted shortcut (design 02), so holding a key does not ramp;
   every press is one step. Changing that would be a global shortcut-engine
   change, not a media-key exception.
3. **One write through the existing owner.** Volume uses
   `SystemStatusClient::setVolume`, mute uses `setMute`, and brightness writes
   settingsd's `display.brightness`; the existing `changed` connection
   forwards it through `applyDisplayPolicy` to the compositor and the T319
   panel writer ([0329](0329-panel-brightness-through-logind.md)). The shell
   then calls the existing `showOsd` with the resulting level. There is no
   second audio or brightness state.
4. **Volume keys preserve mute.** A volume step changes only the level; mute
   is the explicit toggle. This keeps one write per key and makes the OSD
   truthful about the resulting state.
5. **Absence is a silent no-op.** The audio path requires the status host and
   a visible audio view; the brightness path requires the T319 panel writer.
   When either is absent the shell neither writes nor presents the OSD, so a
   key can never open a 220×220 window over nothing.

## Consequences

- Supersedes the “hardware media keys are not wired” consequences of ADRs
  0062 and 0063; the OSD view and its model are unchanged.
- The step policy is pure and unit-tested in `shell/tests/tst_mediakeys.cpp`;
  the binding and precedence are in the compositor `input::shortcuts` tests;
  the OSD presentation is covered by `tst_osd`.
- Media playback keys (play/pause/next/previous) are not bound; T346 owns
  them if they are still wanted.
- The real keyboard check on the reference laptop remains a hardware row in
  the T347 report; the hand-off records the exact `wev` command.
