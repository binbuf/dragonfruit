// SPDX-License-Identifier: GPL-3.0-only
// Chrome stability fixture runner (T349).
//
// `make verify-chrome-stability` invokes this binary through
// `tests/chrome-stability/run_fixtures.py`, which compares the per-function
// results with the checked-in expected-failure manifest. The binary is not a
// CTest target because the pre-fix reproducers are expected to fail until
// their owning tasks land; the manifest is the gate.
//
// Runs headless on the offscreen platform and the software scene graph, like
// the Dock and Menu Bar suites.
#include <QQmlContext>
#include <QQmlEngine>
#include <QtQuickTest>

class ChromeStabilityEnvironment : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QString captureDir READ captureDir CONSTANT)

public:
    explicit ChromeStabilityEnvironment(QObject *parent = nullptr)
        : QObject(parent)
    {}
    QString captureDir() const
    {
        return qEnvironmentVariable("DF_CHROME_STABILITY_CAPTURE_DIR");
    }
};

class ChromeStabilitySetup : public QObject
{
    Q_OBJECT

public slots:
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->rootContext()->setContextProperty(
            "ChromeStabilityEnvironment", new ChromeStabilityEnvironment());
    }
};

QUICK_TEST_MAIN_WITH_SETUP(tst_chromestability, ChromeStabilitySetup)

#include "tst_chromestability.moc"
