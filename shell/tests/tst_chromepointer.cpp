// SPDX-License-Identifier: MIT
// Chrome pointer injection unit tests (T-16.12): the shared helper owns the
// injected event timestamp, so a bare `QMouseEvent` can never reach a chrome
// window with `timestamp() == 0` (the value that made `QQuickDragHandler` grab
// a stationary press). No QML, Wayland, or compositor.
#include <QMouseEvent>
#include <QVector>
#include <QWindow>

#include <QtTest>

#include "chromepointer.h"

namespace {

// Captures the timestamps of the mouse events the helper delivers to a window.
class MouseStampSpy : public QObject
{
public:
    QVector<quint64> stamps;

protected:
    bool eventFilter(QObject *, QEvent *event) override
    {
        switch (event->type()) {
        case QEvent::MouseMove:
        case QEvent::MouseButtonPress:
        case QEvent::MouseButtonRelease:
            stamps.append(static_cast<QMouseEvent *>(event)->timestamp());
            break;
        default:
            break;
        }
        return false;
    }
};

} // namespace

class TestChromePointer : public QObject
{
    Q_OBJECT

private slots:
    // The first timestamp is never zero and every later one is strictly larger,
    // even when the sequence lands inside one clock millisecond.
    void timestampsAreNonzeroAndStrictlyIncreasing()
    {
        const quint64 first = ChromePointer::timestamp();
        QVERIFY2(first > 0, "the first injected timestamp must not be zero");
        quint64 previous = first;
        for (int i = 0; i < 64; ++i) {
            const quint64 next = ChromePointer::timestamp();
            QVERIFY2(next > previous, "injected timestamps must strictly increase");
            previous = next;
        }
    }

    // Every event `send` delivers carries a fresh nonzero stamp from that same
    // source, so a call site cannot forget one.
    void sendStampsEveryDeliveredEvent()
    {
        QWindow window;
        MouseStampSpy spy;
        window.installEventFilter(&spy);

        ChromePointer::send(&window, QEvent::MouseMove, QPointF(5, 5), Qt::NoButton,
                            Qt::NoButton);
        ChromePointer::send(&window, QEvent::MouseButtonPress, QPointF(5, 5), Qt::LeftButton,
                            Qt::LeftButton);
        ChromePointer::send(&window, QEvent::MouseButtonRelease, QPointF(5, 5), Qt::LeftButton,
                            Qt::NoButton);

        QCOMPARE(spy.stamps.size(), 3);
        for (int i = 0; i < spy.stamps.size(); ++i) {
            QVERIFY2(spy.stamps.at(i) > 0, "a delivered event carried the zero timestamp");
            if (i > 0)
                QVERIFY2(spy.stamps.at(i) > spy.stamps.at(i - 1),
                         "delivered events must carry increasing timestamps");
        }
    }

    // A null window is a no-op, not a crash: call sites guard nothing themselves.
    void sendToNullWindowIsIgnored()
    {
        ChromePointer::send(nullptr, QEvent::MouseMove, QPointF(0, 0), Qt::NoButton, Qt::NoButton);
    }
};

QTEST_MAIN(TestChromePointer)
#include "tst_chromepointer.moc"