// SPDX-License-Identifier: MIT
// Dragonfruit Settings — entry point stub (T-16 builds the real app).
#include <QDateTime>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <cstdio>

#include "MenuPublisher.h"
#include "SettingsDeepLink.h"
#include "systemfont.h"
#include "i18n.h"

namespace {

// T232: `--deep-link <settings://pane/...>` or `--deep-link=<url>`.
QString deepLinkArgument(const QStringList &arguments)
{
    for (int i = 0; i < arguments.size(); ++i) {
        const QString &argument = arguments.at(i);
        if (argument == QStringLiteral("--deep-link") && i + 1 < arguments.size())
            return arguments.at(i + 1);
        if (argument.startsWith(QStringLiteral("--deep-link=")))
            return argument.mid(int(qstrlen("--deep-link=")));
    }
    return {};
}

} // namespace

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-settings"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));
    Dragonfruit::installSystemFont();
    Dragonfruit::installTranslations(app);

    const QString deepLink = deepLinkArgument(app.arguments());
    const QString service = QStringLiteral("org.dragonfruit.SettingsApp1");

    // A second launch is a deep-link delivery, not a second Settings window.
    if (!deepLink.isEmpty()) {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (bus.isConnected() && bus.interface()
                && bus.interface()->isServiceRegistered(service)) {
            QDBusMessage message = QDBusMessage::createMethodCall(
                service, QStringLiteral("/org/dragonfruit/SettingsApp1"),
                service, QStringLiteral("Open"));
            message << deepLink;
            bus.send(message);
            return 0;
        }
    }

    SettingsDeepLink deepLinks;
    if (!deepLinks.registerService())
        qWarning() << "dragonfruit-settings: deep-link service is already owned";
    if (!deepLink.isEmpty())
        qputenv("DF_SETTINGS_START_DEEP_LINK", deepLink.toUtf8());

    QQmlApplicationEngine engine;
    engine.setInitialProperties({{QStringLiteral("startAboutPanel"),
                                  app.arguments().contains(QStringLiteral("--about-system"))}});
    engine.rootContext()->setContextProperty(QStringLiteral("SettingsDeepLinks"),
                                             &deepLinks);
    // T230: the app-side end of the native global-menu publication. The model
    // stays in SettingsMenu.qml; this object only carries it to the broker.
    MenuPublisher menuPublisher;
    engine.rootContext()->setContextProperty(QStringLiteral("MenuPublisher"),
                                             &menuPublisher);
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
