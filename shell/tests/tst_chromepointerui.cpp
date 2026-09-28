// SPDX-License-Identifier: MIT
// Chrome pointer injection view tests (T-16.12): `QUICK_TEST_MAIN_WITH_SETUP`
// finds the QML TestCase in this directory. The setup adds `ChromeInject`, a
// QML-callable wrapper over the production `ChromePointer` injection, so the
// QML test can drive the exact sequence the compositor path uses. QtTest's own
// mouse injection stamps its events, which hides the zero-timestamp
// DragHandler grab; the wrapper reproduces the production sequence exactly.
//
// Run headless with the offscreen platform and the software scene graph (see
// CMakeLists.txt).
#include <QEvent>
#include <QObject>
#include <QPointF>
#include <QQmlContext>
#include <QQmlEngine>
#include <QWindow>

#include <QtQuickTest>

#include "chromepointer.h"

class ChromeInject : public QObject
{
    Q_OBJECT

public:
    using QObject::QObject;

    Q_INVOKABLE void move(QObject *window, qreal x, qreal y)
    {
        ChromePointer::send(windowFor(window), QEvent::MouseMove, QPointF(x, y), Qt::NoButton,
                            m_buttons);
    }

    Q_INVOKABLE void button(QObject *window, qreal x, qreal y, int button, bool pressed)
    {
        const Qt::MouseButton qtButton = static_cast<Qt::MouseButton>(button);
        if (pressed)
            m_buttons |= qtButton;
        else
            m_buttons &= ~qtButton;
        ChromePointer::send(windowFor(window),
                            pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                            QPointF(x, y), qtButton, m_buttons);
    }

    Q_INVOKABLE void left(QObject *window)
    {
        ChromePointer::send(windowFor(window), QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                            m_buttons);
    }

    Q_INVOKABLE void reset() { m_buttons = Qt::NoButton; }

private:
    static QWindow *windowFor(QObject *object) { return qobject_cast<QWindow *>(object); }

    Qt::MouseButtons m_buttons = Qt::NoButton;
};

class ChromeTestSetup : public QObject
{
    Q_OBJECT

public:
    ChromeInject injector;

public slots:
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->rootContext()->setContextProperty(QStringLiteral("ChromeInject"), &injector);
    }
};

QUICK_TEST_MAIN_WITH_SETUP(tst_chromepointerui, ChromeTestSetup)

#include "tst_chromepointerui.moc"