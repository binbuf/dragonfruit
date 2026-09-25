// SPDX-License-Identifier: MIT
// Dragonfruit Files entry point. The real UI lives in the `Dragonfruit.Files`
// QML module; this executable only loads the window and translates the one
// meaningful command-line argument (the location the Dock asked to open).
#include "FilesArguments.h"

#include <QDBusConnection>
#include <QGuiApplication>
#include <QQmlApplicationEngine>

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-files"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

    // The app identity's D-Bus activation name (T-10.6c). `org.dragonfruit.Files1`
    // is the target other components use to open/reveal paths in the running
    // instance; the name is what makes D-Bus activation reachable. A second
    // instance losing the name is fine (single-instance ownership is T-18).
    if (!QDBusConnection::sessionBus().registerService(
                QStringLiteral("org.dragonfruit.Files1"))) {
        qWarning() << "dragonfruit-files: org.dragonfruit.Files1 is already owned";
    }

    // The Dock opens the Trash by launching Files with `trash://` (T-10.6a).
    // Bridge that argument onto the existing `DF_FILES_START_URI` seam the QML
    // shell already reads (`FilesBridge::startUri`); an explicit environment
    // value wins so the capture harness is never overridden.
    if (qEnvironmentVariableIsEmpty("DF_FILES_START_URI")) {
        const QString location = filesLocationArgument(app.arguments());
        if (!location.isEmpty())
            qputenv("DF_FILES_START_URI", location.toUtf8());
    }

    QQmlApplicationEngine engine;
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("Dragonfruit.Files", "FilesWindow");

    return app.exec();
}