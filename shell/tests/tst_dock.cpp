// SPDX-License-Identifier: MIT
// Dock test runner (T-10): `QUICK_TEST_MAIN_WITH_SETUP` finds the QML TestCase
// in this directory. Run headless with the offscreen platform and the software
// scene graph (see CMakeLists.txt).
//
// The setup adds `DockInject`, a thin QML-callable wrapper over the production
// `DockPointer` injection, so the QML tests can drive the exact pointer
// sequence the compositor path uses instead of QtTest's own mouse injection
// (T-14.7x). That sequence is what exposes the zero-timestamp DragHandler bug.
#include <QEvent>
#include <QObject>
#include <QPointF>
#include <QQmlContext>
#include <QQmlEngine>
#include <QWindow>

#include <QtQuickTest>

#include "dockpointer.h"

class DockInject : public QObject
{
    Q_OBJECT

public:
    using QObject::QObject;

    // Mirrors ShellController::onDockPointerMoved / Button / Left.
    Q_INVOKABLE void move(QObject *window, qreal x, qreal y)
    {
        DockPointer::send(windowFor(window), QEvent::MouseMove, QPointF(x, y), Qt::NoButton,
                          m_buttons);
    }

    Q_INVOKABLE void button(QObject *window, qreal x, qreal y, int button, bool pressed)
    {
        const Qt::MouseButton qtButton = static_cast<Qt::MouseButton>(button);
        if (pressed)
            m_buttons |= qtButton;
        else
            m_buttons &= ~qtButton;
        DockPointer::send(windowFor(window),
                          pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                          QPointF(x, y), qtButton, m_buttons);
    }

    Q_INVOKABLE void left(QObject *window)
    {
        DockPointer::send(windowFor(window), QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                          m_buttons);
    }

    Q_INVOKABLE void reset() { m_buttons = Qt::NoButton; }

private:
    static QWindow *windowFor(QObject *object) { return qobject_cast<QWindow *>(object); }

    Qt::MouseButtons m_buttons = Qt::NoButton;
};

class DockTestSetup : public QObject
{
    Q_OBJECT

public:
    DockInject injector;

public slots:
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->rootContext()->setContextProperty(QStringLiteral("DockInject"), &injector);
    }
};

QUICK_TEST_MAIN_WITH_SETUP(tst_dock, DockTestSetup)

#include "tst_dock.moc"