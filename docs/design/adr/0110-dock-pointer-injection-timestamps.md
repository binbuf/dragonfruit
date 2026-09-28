# 0110 — Dock pointer injection stamps a monotonic timestamp

## Status

accepted

## Context

The shell runs its chrome (including the Dock) on the offscreen QPA, so the
compositor's Dock-surface pointer events are re-injected as `QMouseEvent`s into
the offscreen Dock window by `ShellController::onDockPointerMoved` /
`onDockPointerButton` (T-14.7g). Those hand-built events carried no timestamp
(`timestamp() == 0`). With a zero timestamp, `QQuickDragHandler` measures a
bogus initial movement on the press and takes an exclusive grab immediately, so
the sibling left `TapHandler` — which shares the 8 px slop — never receives the
tap. A stationary click on an app or temporary entry (the kinds that enable a
`DragHandler`) silently no-oped; Trash, minimized, and stack entries tapped
because their delegates do not enable a `DragHandler`. The effect only appeared
through this injection path: QtTest's own mouse injection stamps events, so the
existing `tst_dock.qml` cases passed while the live session was dead to clicks.
T-14.7g worked around it with the `DF_DOCK_ACTIVATION_FIXTURE` capture seam.

## Decision

- **The injection stamps every event with a monotonic timestamp.** A small
  `DockPointer` helper in the Wayland-free dock core builds the `QMouseEvent`
  and sets `QInputEvent::setTimestamp` from a shared `QElapsedTimer` before
  `QCoreApplication::sendEvent`. `ShellController` uses this helper for the Dock
  move/press/release/left sequence; no other chrome input path is changed here.
- **The seam is retired.** `DF_DOCK_ACTIVATION_FIXTURE` is removed from the
  shell; the T-14.7g activation and T-14.7l launch-origin captures now perform a
  real synthetic click and discover the pinned entry's position by scanning for
  the launched window (`query identity`).
- **The regression is at the injection boundary.** `tst_dock` exposes a
  QML-callable wrapper over the production `DockPointer`, and new cases drive a
  stationary tap / slop-drag / right-click through it. The app/temporary tap
  cases fail on the pre-fix (untimestamped) code and pass after.

## Consequences

- Handler arbitration stays in QML (ADR 0101's shared slop); production never
  fakes a tap — it fixes the pointer synthesis instead.
- Any future chrome surface re-injected into an offscreen window must use
  `DockPointer` (or set a timestamp) or it can hit the same DragHandler grab.
- The timestamp is wall-independent (elapsed monotonic), so it needs no clock
  injection and cannot regress to zero on the first event.

## Amendment — T-16.12: the rule is chrome-wide, and the helper is renamed

The scope note below was originally Dock-only because only the Dock paired a
`TapHandler` with a `DragHandler`. T-16.12 generalizes the rule to **every**
chrome pointer injection, so no surface can re-introduce the bug when it later
grows a `DragHandler` sibling.

- **One constructor for every injected chrome pointer event.** The helper is
  renamed `ChromePointer` (`shell/src/chromepointer.{h,cpp}`); the timestamp is
  owned by the helper, never by the call site. `DockPointer` remains a thin
  compatibility alias, so the Dock path and `tst_dock`'s `DockInject` name it
  unchanged. A handler that builds its own `QMouseEvent` is the bug.
- **Every injection site routes through it.** Menu bar / main,
  Control Center, the FileChooser overlay, screenshot, screencast, polkit,
  overview, and the notification banner now call `ChromePointer::send` for
  their move / button / leave events, keeping each surface's existing button
  bookkeeping. Ad-hoc `QMouseEvent` construction in those handlers is gone
  (`grep QMouseEvent shell/src/shellcontroller.cpp` is empty), so a reviewer can
  find a violation mechanically.
- **The regression is generalized.** Besides the Dock's `DockInject` cases, a
  sibling `tst_chromepointer` unit test drives the helper directly and
  `tst_chromepointerui` proves a stationary injected tap beside a `DragHandler`
  taps on a non-Dock surface. Zeroing the timestamp in the helper fails that
  case (the T-14.7x probe, repeated), so the invariant is enforced outside the
  Dock.
- **No behavior change is intended.** Only the timestamp differs; positions,
  buttons, modifiers, and per-surface state are identical. The offscreen-window
  injection seam (compositor event → `ShellController::on*Pointer*` →
  `ChromePointer::send` into the offscreen `QWindow`) is unchanged.