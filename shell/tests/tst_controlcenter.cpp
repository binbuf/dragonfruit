// SPDX-License-Identifier: MIT
// Control Center panel test runner (T-11.3a): `QUICK_TEST_MAIN` finds the QML
// TestCase in this directory. Run headless with the offscreen platform and the
// software scene graph (see CMakeLists.txt).
//
// T235 adds an optional headless capture seam: when
// `DF_CONTROL_CENTER_CAPTURE_DIR` is set, the QML capture case writes the
// measured grid, expanded sheets, Edit Controls catalogue, pages, resized
// controls and absence stills there. A normal ctest run is side-effect free.
#include <QGuiApplication>
#include <QQmlContext>
#include <QQmlEngine>
#include <QtQuickTest>

class TestEnvironment : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QString captureDir READ captureDir CONSTANT)

public:
    explicit TestEnvironment(QObject *parent = nullptr) : QObject(parent) {}

    QString captureDir() const
    {
        return qEnvironmentVariable("DF_CONTROL_CENTER_CAPTURE_DIR");
    }
};

class ControlCenterTestSetup : public QObject
{
    Q_OBJECT

public:
    ControlCenterTestSetup() = default;

public slots:
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->rootContext()->setContextProperty("TestEnvironment",
                                                  new TestEnvironment());
    }
};

QUICK_TEST_MAIN_WITH_SETUP(tst_controlcenter, ControlCenterTestSetup)

#include "tst_controlcenter.moc"
