// SPDX-License-Identifier: MIT
// Dragonfruit Settings — entry point stub (T-16 builds the real app).
#include <QGuiApplication>
#include <QQmlApplicationEngine>

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-settings"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

    QQmlApplicationEngine engine;
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("Dragonfruit.Settings", "SettingsWindow");

    return app.exec();
}
