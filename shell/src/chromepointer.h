// SPDX-License-Identifier: MIT
#pragma once

#include <QEvent>
#include <QPointF>
#include <QtGlobal>

class QWindow;

// Chrome pointer injection (T-14.7x, generalized in T-16.12).
//
// The compositor delivers chrome-surface pointer events over the private shell
// protocol; the shell runs its chrome on the offscreen QPA, so `ShellController`
// re-synthesizes them as `QMouseEvent`s into each offscreen chrome window (menu
// bar, Control Center, Dock, chooser, screenshot, screencast, polkit, overview,
// notification banner). A bare `QMouseEvent` has `timestamp() == 0`, which makes
// `QQuickDragHandler` measure a bogus initial movement and take the exclusive
// grab on the press, so a sibling `TapHandler` never receives the tap.
// Stamping every injected event with a monotonic timestamp keeps the handler
// arbitration correct, so a stationary tap taps and only a real move lifts.
//
// This is the single constructor for every injected chrome pointer event: the
// timestamp is owned here, never by the call site. A handler that builds its own
// `QMouseEvent` is the bug.
namespace ChromePointer {

// Starts the shared monotonic clock on first use; the returned value is always
// positive and strictly greater than the previous one, so a first event never
// collides with the zero timestamp and a rapid sequence never repeats.
quint64 timestamp();

// Sends one `QMouseEvent` to `window` at `pos` (local and scene coordinates)
// with a fresh nonzero timestamp. `button` is the button that changed (or
// `Qt::NoButton` for a move) and `buttons` is the full button state, exactly
// like the per-surface button bookkeeping `ShellController` keeps.
void send(QWindow *window, QEvent::Type type, const QPointF &pos, Qt::MouseButton button,
          Qt::MouseButtons buttons);

} // namespace ChromePointer

// Compatibility alias for the T-14.7x Dock helper this grew out of. The Dock
// and `tst_dock`'s `DockInject` still name it; new chrome injections use
// `ChromePointer`.
namespace DockPointer = ChromePointer;