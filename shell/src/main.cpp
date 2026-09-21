// SPDX-License-Identifier: MIT
// Dragonfruit shell process entry point (T-09).
//
// The shell is a Wayland client of our compositor through the private
// protocol. QML rendering is decoupled from the toolkit's Wayland platform
// plugin: the shell connects with libwayland-client itself and paints the
// menu bar into a shared-memory chrome surface, so the bar can reserve its
// zone and be placed on a chrome layer.

#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QFile>
#include <QGuiApplication>
#include <QDebug>

#include <cstdio>

#include "shellcontroller.h"

int main(int argc, char *argv[])
{
    // Keep runtime QML warnings (e.g. Canvas paint errors) visible on stderr.
    qInstallMessageHandler([](QtMsgType, const QMessageLogContext &, const QString &message) {
        fprintf(stderr, "dragonfruit-shell: %s\n", message.toLocal8Bit().constData());
    });

    // Force the offscreen QPA: the private protocol connection is ours, and
    // the Qt Wayland platform plugin would create an unrelated toplevel.
    if (qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM"))
        qputenv("QT_QPA_PLATFORM", "offscreen");

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-shell"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Dragonfruit shell"));
    parser.addHelpOption();
    QCommandLineOption socketOption(QStringLiteral("socket-name"),
                                    QStringLiteral("Compositor Wayland socket name"),
                                    QStringLiteral("name"));
    QCommandLineOption heightOption(QStringLiteral("menubar-height"),
                                    QStringLiteral("Menu bar height in pixels"),
                                    QStringLiteral("px"), QStringLiteral("28"));
    QCommandLineOption placeholdersOption(
        QStringLiteral("placeholders"),
        QStringLiteral("Show placeholder status items until the T-20 adapters land"));
    parser.addOption(socketOption);
    parser.addOption(heightOption);
    parser.addOption(placeholdersOption);
    parser.process(app);

    QString socketName = parser.value(socketOption);
    if (socketName.isEmpty())
        socketName = qEnvironmentVariable("WAYLAND_DISPLAY");
    const int barHeight = parser.value(heightOption).toInt();

    QString token = qEnvironmentVariable("DRAGONFRUIT_LAUNCH_TOKEN");
    if (token.isEmpty()) {
        const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
        if (!runtime.isEmpty() && !socketName.isEmpty()) {
            QFile file(runtime + QLatin1Char('/') + socketName
                       + QStringLiteral(".launch-token"));
            if (file.open(QIODevice::ReadOnly))
                token = QString::fromUtf8(file.readAll()).trimmed();
        }
    }
    if (token.isEmpty()) {
        qWarning("dragonfruit-shell: no launch token (set DRAGONFRUIT_LAUNCH_TOKEN or "
                 "provide the runtime hand-off file)");
        return 1;
    }

    ShellController controller;
    if (!controller.start(socketName, token, barHeight, parser.isSet(placeholdersOption))) {
        fprintf(stderr, "dragonfruit-shell: start failed: %s\n",
                qPrintable(controller.lastError()));
        return 1;
    }
    return app.exec();
}
