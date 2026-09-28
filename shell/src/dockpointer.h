// SPDX-License-Identifier: MIT
#pragma once

#include <QEvent>
#include <QPointF>
#include <QtGlobal>

class QWindow;

// Dock pointer injection (T-14.7x).
//
// The compositor delivers Dock-surface pointer events over the private shell
// protocol; the shell runs its chrome on the offscreen QPA, so `ShellController`
// re-synthesizes them as `QMouseEvent`s into the offscreen Dock window. A bare
// `QMouseEvent` has `timestamp() == 0`, which makes `QQuickDragHandler` measure
// a bogus initial movement and take the exclusive grab on the press, so a
// sibling `TapHandler` never receives the tap: a stationary click on an app or
// temporary entry (the kinds that enable a `DragHandler`) silently no-ops.
// Stamping every injected event with a monotonic timestamp keeps the handler
// arbitration correct, so a stationary tap taps and only a real move lifts.
//
// This is injection plumbing, shared by `ShellController` and the
// `tst_dock` regression so the test drives the production sequence.
namespace DockPointer {

// Starts the shared monotonic clock on first use; the returned value is always
// positive, so a first event never collides with the zero timestamp.
quint64 timestamp();

// Sends one `QMouseEvent` to `window` at `pos` (local and scene coordinates)
// with a fresh nonzero timestamp. `button` is the button that changed (or
// `Qt::NoButton` for a move) and `buttons` is the full button state, exactly
// like `ShellController`'s Dock injection.
void send(QWindow *window, QEvent::Type type, const QPointF &pos, Qt::MouseButton button,
          Qt::MouseButtons buttons);

} // namespace DockPointer