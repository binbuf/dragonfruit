// SPDX-License-Identifier: MIT
// Dragonfruit Files — entry point stub (T-18 builds the real app).
#include <QGuiApplication>
#include <QQmlApplicationEngine>

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-files"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

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
