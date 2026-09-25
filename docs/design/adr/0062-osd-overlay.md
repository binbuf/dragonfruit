# 0062 — OSD overlay: centered shell surface and a pure presentation model

## Status

accepted

## Context

T-11.4a adds the brief volume/brightness overlay the T-11 track promises
([11-control-center-notifications.md](../tracks/11-control-center-notifications.md)).
The Control Center (T-11.3a/b) already provides the user gestures that change
volume (`SystemStatusClient`) and brightness (settingsd `display.brightness`);
the notification service owns Focus/DND. Nothing owned the transient OSD, and
the design's legacy OSD policy is specific: appear on the active output, respect
reduced motion, and be suppressed while a fullscreen surface owns the output
([legacy/25-notifications-and-osd.md](../../tasks/legacy/25-notifications-and-osd.md)).

## Decision

- **The OSD is shell-rendered into a centered `overlay` layer surface.** The
  surface namespace is `"osd"`, size 220×220, with **no anchors**, so the
  compositor centers it; it reserves nothing (`exclusive_zone = -1`), never
  takes keyboard, and has an empty input region. This reuses the compositor's
  per-output overlay rule: an unanchored `overlay` renders on the output the
  chrome interaction happened on (the active output in practice), and falls
  back to every output before any chrome focus exists.
- **A pure model owns the presentation state.** `shell/src/osdmodel.{h,cpp}`
  keeps `kind`, `value`, `muted`, the visibility window, the deadline, and the
  fullscreen suppression policy. `present` (re)arms the display and the last
  change wins, so concurrent volume+brightness triggers coalesce; `tick`
  auto-dismisses; `fade` is the animation curve. It is deliberately not a
  `QObject` and holds no timer, so the contract is unit-tested headlessly.
- **The shell drives the fade on a 16 ms timer only while visible.**
  `ShellController` owns one timer, starts it on `present`, and stops it on
  hide/dismiss, so the idle shell adds no wakeups. (A brief OSD is expected to
  render frames; FR-6 is about the idle desktop.)
- **Reduced motion collapses the fade to 1.0 immediately.** The model takes the
  reduced-motion flag (the shell's `CompositorPolicy`, the same settingsd
  value) and returns a constant 1.0, so the state change stays legible without
  interpolation; there is no QML animation to disable.
- **A fullscreen surface suppresses the OSD.** `OsdModel::setFullscreen`
  hides a visible card at once and refuses a later `present`;
  `ShellProtocol::fullscreenOverlayActive()` derives this from the active Space
  that a fullscreen window owns, with the focused toplevel's fullscreen state
  bit as a fallback. Volume/brightness are not critical warnings, so nothing is
  exempted.
- **The card reuses the design system.** New `component.osd` and `motion.osd`
  tokens live in `tokens.json` (generated into `Theme.qml` /
  `design_tokens.rs`); the view is `shell/osd/Osd.qml`, a pure projection of the
  model's `kind`/`value`/`muted`/`fade`.
- **A capture-only seam presents the OSD deterministically.**
  `DF_OSD_FIXTURE=volume|brightness` triggers one presentation shortly after
  startup, mirroring `DF_DOCK_FAIL_FIXTURE`; it is never set in a real session.

Rejected: a compositor-drawn OSD (the compositor has no text renderer and the
shell already owns every chrome surface); a per-output pinned surface (the
compositor's active output is not currently exposed to the shell, and the
unanchored rule already targets the interaction output); a QML-driven fade with
a `Behavior` (it would duplicate the model's timing and split the reduced-motion
path).

## Consequences

- The OSD shows on the interaction output; before any chrome focus it shows on
  every output. A single-output session is exact. Pinning it to the
  compositor's active output is a later enhancement.
- Hardware volume/brightness keys are not wired to the OSD in this slice
  (there is no compositor input action yet); the Control Center and the menu-bar
  gestures are the triggers. A later task can route media keys into the same
  `showOsd` seam.
- T-11.4b adds the keyboard/AT-SPI pass and commits the OSD/DND capture set;
  the view already carries an `Accessible.Alert` name for it to build on.