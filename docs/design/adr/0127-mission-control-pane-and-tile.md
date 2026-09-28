# 0127 — The Mission Control pane and tile are shell-native plus settingsd keys

- **Status:** Accepted (T-15.5b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settingsd-live-settings.md](../tracks/08-settingsd-live-settings.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md),
  ADR [0126](0126-mission-control-hot-corners-adapter.md)

## Context

T-15.5a shipped `dragonfruit-overview`, the adapter over the compositor's
hot-corner detector and its one overview machine. Unlike Bluetooth, storage,
audio, and input, this subsystem has **no external daemon**: the compositor
owns the runtime and the shell is the only process that speaks to it, over the
private `df_toplevel_manager` bridge. The T-15.5a consequences note anticipated
that T-15.5b would "wire the shell bridge through the status bridge host"; that
plan does not survive contact with the architecture, because a
`dragonfruit-system-status` process cannot read a Wayland client's private
socket. Forcing it would mean inventing a shell→host push bridge that publishes
one process's compositor mirror back to a service the same process reads.

The pane also needs a durable home for the four hot-corner assignments. That is
principle 3's existing split: `settingsd` owns the preference, the compositor
applies it live. The gesture trio that also drives Mission Control already
exists as the revision-1 `gestures.*` keys and is already forwarded over
`set_input_policy`.

## Decision

The pane and the Control Center tile are one functional unit, and both are
shell-native where the runtime is concerned.

1. **Settingsd keys, revision 13.** `overview.hotCornerTopLeft`,
   `overview.hotCornerTopRight`, `overview.hotCornerBottomLeft`, and
   `overview.hotCornerBottomRight` are enumerated text keys whose values are
   the stable `HotCornerAction::id()` spellings (`none`, `mission-control`,
   `notification-center`, `desktop-reveal`, `lock-screen`); defaults mirror
   `HotCornerConfig::default()`. `docs/settings-keys.md` documents them and the
   schema-doc test keeps the table honest.
2. **The pane writes keys, never the adapter.** `apps/settings/MissionControlPane.qml`
   has four `Select` rows bound to the corner keys and two toggles for
   `gestures.missionControl`/`gestures.spaceSwitch`. Every row applies live and
   persists. Applying an assignment in the compositor needs the append-only
   policy request ADR [0126](0126-mission-control-hot-corners-adapter.md) names
   and is deferred, exactly as the T-15.4b pointer keys were; the preference is
   the single source of truth until then.
3. **The tile is projected by the shell.** No services-layer host is added.
   `shell/src/controlcenterpolicy.cpp::missionControlView` maps the settingsd
   values onto the tile view, mirroring the Rust `MissionControlSnapshot::label()`
   (`Gesture`, `Gesture, n corner(s)`, `n corner(s)`, `No trigger`) and the
   `overview` glyph. It is pure and unit-tested by `tst_controlcenterpolicy`;
   the QML tile renders it and raises `missionControlSettingsRequested`.
4. **Absence is a missing settings daemon.** With no daemon the pane's rows
   stay live on the schema defaults and it shows a one-line note. There is no
   second `present` hide rule because the shell path has no adapter view.

Rejected: a `dragonfruit-system-status` overview host (no honest live source);
a shell→D-Bus push bridge (inverts ownership and duplicates the one overview
mirror); a per-corner widget grid in the Control Center (the tile is a
quick-settings entry, not a second pane).

## Consequences

- The Control Center panel grows from 360×880 to 360×980 to fit the new tile;
  the fit test and the T-15.5b capture assert it.
- `overview.*` keys persist and apply live in settingsd but the compositor does
  not yet read the corner assignments. A later task extends
  `df_toplevel_manager` with an append-only `set_hot_corners` request and
  forwards it from `applyCompositorPolicy`, the same shape as the pointer-key
  follow-up in ADR [0125](0125-keyboard-mouse-trackpad-pane-and-tile.md).
- `dragonfruit-overview` remains the contract/model for the subsystem and is
  exercised by its own crate tests; it is not linked into the shell or the
  services host.
- The pane and tile drop the Apple-only `Stage Manager` toggle and the
  `In Stage Manager` segment (they map to our overview/workspaces, not to
  Mission Control's trigger configuration) and the project-wide Apple Account
  sidebar row (ADR [0122](0122-tahoe-interface-language-across-chrome.md)).