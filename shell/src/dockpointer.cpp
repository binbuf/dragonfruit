// SPDX-License-Identifier: MIT
#include "dockpointer.h"

#include <QCoreApplication>
#include <QElapsedTimer>
#include <QMouseEvent>
#include <QWindow>

namespace DockPointer {

quint64 timestamp()
{
    static QElapsedTimer clock = [] {
        QElapsedTimer timer;
        timer.start();
        return timer;
    }();
    // The monotonic reference is already large; +1 guarantees the first event is
    // never zero (the value that triggers the DragHandler bug).
    return static_cast<quint64>(clock.msecsSinceReference()) + 1;
}

void send(QWindow *window, QEvent::Type type, const QPointF &pos, Qt::MouseButton button,
          Qt::MouseButtons buttons)
{
    if (!window)
        return;
    QMouseEvent event(type, pos, pos, button, buttons, Qt::NoModifier);
    event.setTimestamp(timestamp());
    QCoreApplication::sendEvent(window, &event);
}

} // namespace DockPointer