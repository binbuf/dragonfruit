// SPDX-License-Identifier: MIT
// Dragonfruit Settings — entry point stub (T-16 builds the real app).
#include <QDateTime>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <cstdio>

#include "systemfont.h"

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-settings"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));
    Dragonfruit::installSystemFont();

    QQmlApplicationEngine engine;
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("Dragonfruit.Settings", "SettingsWindow");

    // Temporary pane-switch timing trace: with `DF_SETTINGS_TRACE=1`, log each
    // presented app frame so its cadence can be correlated with the
    // compositor's `DRAGONFRUIT_FRAME_TRACE` lines.
    if (qEnvironmentVariableIsSet("DF_SETTINGS_TRACE")) {
        for (QObject *root : engine.rootObjects()) {
            if (auto *window = qobject_cast<QQuickWindow *>(root)) {
                QObject::connect(
                    window, &QQuickWindow::afterRendering, window, []() {
                        std::fprintf(stderr, "DFTRACE appframe t=%lld\n",
                                     QDateTime::currentMSecsSinceEpoch());
                    });
            }
        }
    }

    return app.exec();
}
