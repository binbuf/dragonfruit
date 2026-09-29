// SPDX-License-Identifier: MIT
// Dragonfruit Files entry point. The real UI lives in the `Dragonfruit.Files`
// QML module; this executable loads the window and translates the one
// meaningful command-line argument (the location the Dock asked to open).
//
// `dragonfruit-files --desktop` is the other process this binary can be (T-19.3):
// the Files-owned desktop, a trusted private-protocol client rendering
// `~/Desktop` on the compositor's background layer.
#include "FilesArguments.h"

#include <cstdio>

#include <QDBusConnection>
#include <QDesktopServices>
#include <QFile>
#include <QGuiApplication>
#include <QProcess>
#include <QProcessEnvironment>
#include <QQmlApplicationEngine>
#include <QStringList>
#include <QUrl>

#include "systemfont.h"
#include "i18n.h"

#ifdef DF_FILES_DESKTOP_CLIENT
#include "FilesDesktop.h"
#endif

namespace {

#ifdef DF_FILES_DESKTOP_CLIENT
// The one-time `desktop:` launch token, from the environment or the
// `<socket>.desktop-launch-token` hand-off file the compositor writes (the
// same convention the shell uses for `<socket>.launch-token`).
QString desktopLaunchToken(const QString &socketName)
{
    QString token = qEnvironmentVariable("DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN");
    if (!token.isEmpty())
        return token;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (!runtime.isEmpty() && !socketName.isEmpty()) {
        QFile file(runtime + QLatin1Char('/') + socketName
                   + QStringLiteral(".desktop-launch-token"));
        if (file.open(QIODevice::ReadOnly))
            token = QString::fromUtf8(file.readAll()).trimmed();
    }
    return token;
}

// Environment for a Files browser the desktop launches. The desktop process
// forces the offscreen platform for its own `QQuickWindow`; the browser must
// render as a normal Wayland client of the compositor, so QT_QPA_PLATFORM is
// dropped (otherwise a double-click would open an invisible offscreen window).
QProcessEnvironment browserEnvironment()
{
    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    env.remove(QStringLiteral("QT_QPA_PLATFORM"));
    return env;
}

int runDesktop(QGuiApplication &app)
{
    const QString socketName = qEnvironmentVariable("WAYLAND_DISPLAY");
    const QString token = desktopLaunchToken(socketName);
    if (token.isEmpty()) {
        qWarning("dragonfruit-files: no desktop launch token (set "
                 "DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN or provide the runtime hand-off file)");
        return 1;
    }

    QQmlEngine engine;
    FilesDesktop desktop(&engine);
    QObject::connect(&desktop, &FilesDesktop::openRequested,
                     [&app](const QString &uri, bool isDir) {
        if (uri.isEmpty())
            return;
        if (isDir) {
            // A directory opens a Files window at that path: a second process,
            // so the browser and the desktop never share a crash domain. The
            // browser gets a normal (non-offscreen) platform environment.
            QProcess browser;
            browser.setProcessEnvironment(browserEnvironment());
            browser.setProgram(app.applicationFilePath());
            browser.setArguments({uri});
            if (!browser.startDetached())
                qWarning("dragonfruit-files: could not open %s", qPrintable(uri));
        } else {
            QDesktopServices::openUrl(QUrl(uri));
        }
    });
    if (!desktop.start(socketName, token, filesDesktopDirectory())) {
        fprintf(stderr, "dragonfruit-files: desktop start failed: %s\n",
                qPrintable(desktop.lastError()));
        return 1;
    }
    return app.exec();
}
#endif

} // namespace

int main(int argc, char *argv[])
{
    QStringList arguments;
    arguments.reserve(argc);
    for (int i = 0; i < argc; ++i)
        arguments.append(QString::fromLocal8Bit(argv[i]));
    const bool desktop = filesDesktopRequested(arguments);

    // The desktop process speaks the private protocol itself; the Qt Wayland
    // platform plugin would create an unrelated toplevel, so force offscreen
    // before the platform is chosen (mirroring `dragonfruit-shell`). This must
    // happen before `QGuiApplication` constructs the platform plugin.
#ifdef DF_FILES_DESKTOP_CLIENT
    if (desktop && qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM"))
        qputenv("QT_QPA_PLATFORM", "offscreen");
#endif

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-files"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));
    Dragonfruit::installSystemFont();
    Dragonfruit::installTranslations(app);

    if (desktop) {
#ifdef DF_FILES_DESKTOP_CLIENT
        return runDesktop(app);
#else
        fprintf(stderr, "dragonfruit-files: --desktop requested but this build has no "
                        "Wayland client\n");
        return 1;
#endif
    }

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