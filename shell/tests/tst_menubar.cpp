// SPDX-License-Identifier: MIT
// Menu-bar test runner (T-09): `QUICK_TEST_MAIN_WITH_SETUP` finds the QML
// TestCase in this directory and exposes the optional capture directory
// (`DF_MENUBAR_CAPTURE_DIR`) as `TestEnvironment.captureDir` for T231's
// evidence case. Run headless with the offscreen platform and the software
// scene graph (see CMakeLists.txt).
#include <QQmlContext>
#include <QQmlEngine>
#include <QtQuickTest>

class TestEnvironment : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QString captureDir READ captureDir CONSTANT)

public:
    explicit TestEnvironment(QObject *parent = nullptr) : QObject(parent) {}
    QString captureDir() const { return qEnvironmentVariable("DF_MENUBAR_CAPTURE_DIR"); }
};

class TestSetup : public QObject
{
    Q_OBJECT

public slots:
    void applicationAvailable() {}
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->rootContext()->setContextProperty("TestEnvironment",
                                                  new TestEnvironment());
    }
};

QUICK_TEST_MAIN_WITH_SETUP(tst_menubar, TestSetup)

#include "tst_menubar.moc"
