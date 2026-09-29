// SPDX-License-Identifier: MIT
#include "chromepointer.h"

#include <QCoreApplication>
#include <QElapsedTimer>
#include <QMouseEvent>
#include <QWindow>

namespace ChromePointer {

quint64 timestamp()
{
    static QElapsedTimer clock = [] {
        QElapsedTimer timer;
        timer.start();
        return timer;
    }();
    static quint64 last = 0;
    // The monotonic reference is already large; +1 guarantees the first event is
    // never zero (the value that triggers the DragHandler bug). A strict bump
    // past the previous value keeps a same-millisecond sequence monotonic too.
    quint64 now = static_cast<quint64>(clock.msecsSinceReference()) + 1;
    if (now <= last)
        now = last + 1;
    last = now;
    return now;
}

void send(QWindow *window, QEvent::Type type, const QPointF &pos, Qt::MouseButton button,
          Qt::MouseButtons buttons, Qt::KeyboardModifiers modifiers)
{
    if (!window)
        return;
    QMouseEvent event(type, pos, pos, button, buttons, modifiers);
    event.setTimestamp(timestamp());
    QCoreApplication::sendEvent(window, &event);
}

} // namespace ChromePointer