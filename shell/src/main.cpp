// SPDX-License-Identifier: GPL-3.0-or-later
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
#include <QLibraryInfo>
#include <QProcessEnvironment>
#include <QThread>
#include <QDebug>

#include <cstdio>
#include <csignal>

#include "shellcontroller.h"
#include "systemfont.h"
#include "i18n.h"
#include "gpuchromeframeproducer.h"
#include "qpaplatform.h"

namespace {

bool qtWaylandPlatformPluginAvailable()
{
    return waylandQpaPluginPresent(qpaPluginDirectories(
        QLibraryInfo::path(QLibraryInfo::PluginsPath),
        QProcessEnvironment::systemEnvironment()));
}

void useOffscreenQpa(const QString &reason)
{
    qWarning().noquote()
        << QStringLiteral(
               "%1; falling back to the offscreen QPA. "
               "Install the Qt 6 Wayland platform plugin (for example "
               "qt6-qtwayland on Fedora or qt6-wayland on Debian/Arch) to "
               "use the GPU chrome rail.")
               .arg(reason);
    qputenv("QT_QPA_PLATFORM", "offscreen");
}

} // namespace

int main(int argc, char *argv[])
{
    // A clipboard client can close its read end before the shell finishes
    // serving it (T310); never let SIGPIPE terminate the session.
    std::signal(SIGPIPE, SIG_IGN);

    // Keep runtime QML warnings (e.g. Canvas paint errors) visible on stderr.
    qInstallMessageHandler([](QtMsgType, const QMessageLogContext &, const QString &message) {
        fprintf(stderr, "dragonfruit-shell: %s\n", message.toLocal8Bit().constData());
    });

    // The private protocol connection is ours, so the shell does not need a
    // QPA of its own unless the GPU rail can run. GPU-first: when the
    // transport is requested (the default, unless `DRAGONFRUIT_CHROME_GPU`
    // is 0/off/false) and a host Wayland display is present, use the Wayland
    // QPA for the Qt-side GL context only. The chrome surfaces stay on the
    // shell's own libwayland connection (ADR 0195); no QQuickWindow is shown,
    // so the QPA creates no toplevel. A missing/no-GPU host falls back to the
    // offscreen software rail in the producer. The Wayland QPA is a separate
    // distro package on some hosts (T342); check for it before
    // QGuiApplication aborts on the missing plugin.
    const QString requestedPlatform = qEnvironmentVariable("QT_QPA_PLATFORM");
    if (requestedPlatform.isEmpty()) {
        if (GpuChromeFrameProducer::environmentRequested()
            && !qEnvironmentVariableIsEmpty("WAYLAND_DISPLAY")) {
            if (qtWaylandPlatformPluginAvailable()) {
                qputenv("QT_QPA_PLATFORM", "wayland");
            } else {
                useOffscreenQpa(QStringLiteral(
                    "the Qt Wayland platform plugin is missing"));
            }
        } else {
            qputenv("QT_QPA_PLATFORM", "offscreen");
        }
    } else if (qpaPlatformIsWayland(requestedPlatform)
               && !qtWaylandPlatformPluginAvailable()) {
        useOffscreenQpa(
            QStringLiteral("QT_QPA_PLATFORM=%1 was requested but the Qt "
                           "Wayland platform plugin is missing")
                .arg(requestedPlatform));
    }

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-shell"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

    // The desktop's type, before any QML surface is created.
    Dragonfruit::installSystemFont();

    // The desktop's language, before any QML surface is created (T-16.7):
    // loads the catalog for the effective locale and sets the default
    // `QLocale` for date/number formatting.
    Dragonfruit::installTranslations(app);

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Dragonfruit shell"));
    parser.addHelpOption();
    QCommandLineOption socketOption(QStringLiteral("socket-name"),
                                    QStringLiteral("Compositor Wayland socket name"),
                                    QStringLiteral("name"));
    QCommandLineOption heightOption(QStringLiteral("menubar-height"),
                                    QStringLiteral("Menu bar height in pixels"),
                                    QStringLiteral("px"), QStringLiteral("28"));
    parser.addOption(socketOption);
    parser.addOption(heightOption);
    parser.process(app);

    QString socketName = parser.value(socketOption);
    if (socketName.isEmpty())
        socketName = qEnvironmentVariable("WAYLAND_DISPLAY");
    const int barHeight = parser.value(heightOption).toInt();

    // Prefer the compositor's runtime hand-off file over the environment:
    // a crashed/restarted shell gets a freshly minted token in that file when
    // its previous connection disconnects, while the copy imported into the
    // systemd user environment is the original (now consumed) token. The
    // environment remains the fallback for a session that has not written
    // the file yet.
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    auto readToken = [&]() {
        QString token;
        if (!runtime.isEmpty() && !socketName.isEmpty()) {
            QFile file(runtime + QLatin1Char('/') + socketName
                       + QStringLiteral(".launch-token"));
            if (file.open(QIODevice::ReadOnly))
                token = QString::fromUtf8(file.readAll()).trimmed();
        }
        if (token.isEmpty())
            token = qEnvironmentVariable("DRAGONFRUIT_LAUNCH_TOKEN");
        return token;
    };

    // A systemd restart can race the compositor's disconnect handler: the new
    // shell may read the previous (consumed) token just before the compositor
    // rewrites the hand-off file. Retry a few times with the freshly read
    // token instead of crash-looping the whole shell service.
    QString token = readToken();
    if (token.isEmpty()) {
        qWarning("dragonfruit-shell: no launch token (set DRAGONFRUIT_LAUNCH_TOKEN or "
                 "provide the runtime hand-off file)");
        return 1;
    }

    constexpr int kStartAttempts = 4;
    for (int attempt = 0; attempt < kStartAttempts; ++attempt) {
        ShellController controller;
        if (controller.start(socketName, token, barHeight)) {
            // T314: the unit is Type=notify with WatchdogSec=30. READY and the
            // GUI-thread heartbeat are armed from the event loop, so a shell
            // that hangs before its first iteration is still caught.
            controller.startShellWatchdog();
            return app.exec();
        }
        fprintf(stderr, "dragonfruit-shell: start failed (attempt %d/%d): %s\n",
                attempt + 1, kStartAttempts, qPrintable(controller.lastError()));
        if (attempt + 1 == kStartAttempts)
            break;
        // Let the compositor observe the closed connection and reissue the
        // one-time token before re-reading the hand-off file.
        QThread::msleep(250);
        token = readToken();
    }
    return 1;
}
