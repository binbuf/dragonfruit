// SPDX-License-Identifier: GPL-3.0-only
// Files entry point. The real UI lives in the `Dragonfruit.Files`
// QML module; this executable loads the window and translates the one
// meaningful command-line argument (the location the Dock asked to open).
//
// `dragonfruit-files --desktop` is the other process this binary can be (T-19.3):
// the Files-owned desktop, a trusted private-protocol client rendering
// `~/Desktop` on the compositor's background layer.
//
// Single instance (T333): the first browser process owns
// `org.dragonfruit.Files1` and answers `Activate(location)` by opening a new
// tab; a `--activation-handoff` launch asks it before opening its own window.
#include "FilesArguments.h"
#include "FilesThumbnailProvider.h"
#include "MenuPublisher.h"
#include "OpenHandler.h"

#include <cstdio>

#include <QDBusConnection>
#include <QDBusContext>
#include <QDBusInterface>
#include <QDBusReply>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QMetaObject>
#include <QProcess>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QStringList>
#include <QUrl>

#include "systemfont.h"
#include "i18n.h"

#ifdef DF_FILES_DESKTOP_CLIENT
#include "FilesDesktop.h"
#endif

namespace {

// Ask an already-running Files to open `location` (empty: show its windows)
// as a new tab, over the app's one D-Bus name (T333). Returns true when it
// answered, so this process can exit. A request with no location is sent as
// `activate` with an empty location.
bool handOffToRunningInstance(const QString &location)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    QDBusInterface files(QStringLiteral("org.dragonfruit.Files1"),
                         QStringLiteral("/org/dragonfruit/Files1"),
                         QStringLiteral("org.dragonfruit.Files1"), bus);
    if (!files.isValid())
        return false;
    QDBusReply<void> reply = files.call(QStringLiteral("Activate"), location);
    if (!reply.isValid()) {
        qWarning() << "dragonfruit-files: hand-off to the running instance failed:"
                   << reply.error().message();
        return false;
    }
    return true;
}

#ifdef DF_FILES_DESKTOP_CLIENT
// The one-time `desktop:` launch token, from the `<socket>.desktop-launch-token`
// hand-off file the compositor writes (the same convention the shell uses for
// `<socket>.launch-token`). Read the file first: the compositor reissues it
// when a trusted client disconnects, whereas the environment copy is the
// original token and is already consumed after a restart.
QString desktopLaunchToken(const QString &socketName)
{
    QString token;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (!runtime.isEmpty() && !socketName.isEmpty()) {
        QFile file(runtime + QLatin1Char('/') + socketName
                   + QStringLiteral(".desktop-launch-token"));
        if (file.open(QIODevice::ReadOnly))
            token = QString::fromUtf8(file.readAll()).trimmed();
    }
    if (token.isEmpty())
        token = qEnvironmentVariable("DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN");
    return token;
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
    engine.addImageProvider(QStringLiteral("files-thumbnail"),
                            new FilesThumbnailProvider);
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
            browser.setProcessEnvironment(filesDesktopChildEnvironment());
            browser.setProgram(app.applicationFilePath());
            browser.setArguments({uri});
            if (!browser.startDetached())
                qWarning("dragonfruit-files: could not open %s", qPrintable(uri));
        } else {
            // A file opens in its default handler. Never call QDesktopServices
            // here: the desktop is an offscreen-platform process, whose plugin
            // has no QPlatformServices (it logs "This plugin does not support
            // QPlatformServices::openDocument()"). `filesOpen` resolves the
            // MIME default and launches the handler out of process with the
            // offscreen platform stripped instead.
            if (!filesOpen(uri))
                qWarning("dragonfruit-files: no default handler (xdg-open/gio) "
                         "for %s", qPrintable(uri));
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

// The running instance's `org.dragonfruit.Files1` object (T333): the
// single-instance target. `Activate` opens `location` as a new tab in the
// current window (empty raises the window); a non-browsable location is
// refused with an error. The object is registered only by the process that
// won the name; a second window process publishes nothing.
class FilesInstance : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Files1")

public:
    explicit FilesInstance(QQmlApplicationEngine *engine, QObject *parent = nullptr)
        : QObject(parent), m_engine(engine)
    {
    }

public slots:
    void Activate(const QString &location)
    {
        if (!calledFromDBus()) {
            apply(location);
            return;
        }
        const QUrl url(location);
        if (!location.isEmpty() && url.isValid() && url.scheme().isEmpty()) {
            apply(QUrl::fromLocalFile(QFileInfo(location).absoluteFilePath()).toString());
            return;
        }
        if (!location.isEmpty() && url.scheme() != QStringLiteral("file")
            && url.scheme() != QStringLiteral("trash")) {
            sendErrorReply(QDBusError::InvalidArgs,
                           QStringLiteral("Files browses file:// and trash:// locations"));
            return;
        }
        apply(location);
    }

private:
    void apply(const QString &location)
    {
        const QList<QObject *> roots = m_engine->rootObjects();
        for (QObject *root : roots) {
            if (QMetaObject::invokeMethod(root, "activateLocation",
                                          Q_ARG(QVariant, location)))
                return;
        }
    }

    QQmlApplicationEngine *m_engine = nullptr;
};

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

    // T333: single instance. A component's "open this location" request
    // (`--activation-handoff=<token>`) is handed to an already-running Files,
    // which opens it as a new tab; only when nobody answers does this process
    // open a window. A bare invocation, the Dock's positional location and
    // `--new-window` (the window menu's New Window) always open a window, so
    // Dock clicks and New Window dual-boot the browser.
    const bool activation = filesActivationRequested(arguments)
            && !filesNewWindowRequested(arguments);
    if (activation) {
        const QString location = filesHandoffLocation(arguments);
        if (handOffToRunningInstance(location)) {
            // Leave the service name alone: the running instance owns it, and
            // an unregister here would race that owner.
            return 0;
        }
    }

    // The app identity's D-Bus activation name (T-10.6c). `org.dragonfruit.Files1`
    // is the target other components use to open a tab in the running
    // instance; a second window process losing the name is fine (ADR 0340).
    const bool ownsName = QDBusConnection::sessionBus().registerService(
                QStringLiteral("org.dragonfruit.Files1"));
    if (!ownsName)
        qWarning() << "dragonfruit-files: org.dragonfruit.Files1 is already owned";

    // The Dock opens the Trash by launching Files with `trash://` (T-10.6a).
    // Bridge that argument onto the existing `DF_FILES_START_URI` seam the QML
    // shell already reads (`FilesBridge::startUri`); an explicit environment
    // value wins so the capture harness is never overridden. A hand-off
    // request's (`--location=`) or `--new-window`'s location wins over a
    // positional one.
    if (qEnvironmentVariableIsEmpty("DF_FILES_START_URI")) {
        const QString location = filesLaunchLocation(arguments);
        if (!location.isEmpty())
            qputenv("DF_FILES_START_URI", location.toUtf8());
    }

    // T334: the app-side end of the native global-menu publication. The model
    // stays in FilesMenu.qml; this object only carries it to the broker.
    // Declared before the engine so it outlives QML's `onDestruction` calls.
    MenuPublisher menuPublisher;

    QQmlApplicationEngine engine;
    engine.addImageProvider(QStringLiteral("files-thumbnail"),
                            new FilesThumbnailProvider);
    engine.rootContext()->setContextProperty(QStringLiteral("MenuPublisher"),
                                             &menuPublisher);
    if (ownsName) {
        QDBusConnection::sessionBus().registerObject(
                QStringLiteral("/org/dragonfruit/Files1"),
                QStringLiteral("org.dragonfruit.Files1"),
                new FilesInstance(&engine, &engine),
                QDBusConnection::ExportAllSlots);
    }
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("Dragonfruit.Files", "FilesWindow");

    return app.exec();
}

#include "main.moc"
