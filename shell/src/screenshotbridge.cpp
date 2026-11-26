// SPDX-License-Identifier: GPL-3.0-only
#include "screenshotbridge.h"

#include "portalpresenter.h"

#if defined(QT_DBUS_LIB)
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#endif

namespace {

// The portal backend's diagnostic presenter surface (T-13.3a).
const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");
const QString kInterface = QStringLiteral("org.dragonfruit.Portal1");

// The mode labels the portal's `ScreenshotOpened` signal and the shortcut path
// use. An unknown value falls back to the interactive region selection.
QString normalizeMode(const QString &mode)
{
    if (mode == QLatin1String("window"))
        return QStringLiteral("window");
    if (mode == QLatin1String("fullscreen"))
        return QStringLiteral("fullscreen");
    return QStringLiteral("region");
}

} // namespace

ScreenshotBridge::ScreenshotBridge(QObject *parent)
    : QObject(parent)
{
}

ScreenshotBridge::~ScreenshotBridge() = default;

void ScreenshotBridge::connectService()
{
#if defined(QT_DBUS_LIB)
    if (m_connected)
        return;
    m_connected = true;
    QDBusConnection bus = QDBusConnection::sessionBus();
    bus.connect(kService, kPath, kInterface, QStringLiteral("ScreenshotOpened"), this,
                SLOT(onScreenshotOpened(QString, QString, QString, QString, QVariantMap)));
    if (QDBusConnectionInterface *iface = bus.interface()) {
        const auto update = [this](const QString &name) {
            if (name == kService)
                checkService();
        };
        connect(iface, &QDBusConnectionInterface::serviceRegistered, this, update);
        connect(iface, &QDBusConnectionInterface::serviceUnregistered, this, update);
    }
    checkService();
#endif
}

void ScreenshotBridge::checkService()
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    const bool wasAvailable = m_serviceAvailable;
    m_serviceAvailable = iface && iface->isServiceRegistered(kService);
    qInfo() << "shell: screenshot portal available" << m_serviceAvailable
            << bus.baseService();
    // A restarted backend can hold requests this shell never saw. Ask it what
    // is waiting instead of assuming a signal will arrive for them.
    if (m_serviceAvailable && !wasAvailable)
        queryPending();
#else
    m_serviceAvailable = false;
#endif
}

void ScreenshotBridge::onScreenshotOpened(const QString &handle, const QString &mode,
                                          const QString &appId, const QString &parentWindow,
                                          const QVariantMap &options)
{
    Q_UNUSED(options);
    qInfo() << "shell: screenshot portal request" << handle << mode << appId;
    begin(handle, mode, appId, parentWindow);
}

void ScreenshotBridge::begin(const QString &handle, const QString &mode, const QString &appId,
                            const QString &parentWindow)
{
    Q_UNUSED(parentWindow);
    // A second request replaces the first. Answer the old handle before
    // dropping it, or the first caller waits for the portal's deadline.
    cancelCurrent();
    reset();
    m_active = true;
    m_handle = handle;
    m_mode = normalizeMode(mode);
    m_appId = appId;
    emit started();
    emit changed();
}

void ScreenshotBridge::beginLocal(const QString &mode)
{
    // The desktop's own shortcut replaces any portal request that is waiting.
    cancelCurrent();
    reset();
    m_active = true;
    m_mode = normalizeMode(mode);
    emit started();
    emit changed();
}

void ScreenshotBridge::accept(int x, int y, int width, int height)
{
    if (!m_active)
        return;
    if (width <= 0 || height <= 0) {
        m_error = tr("Select an area to capture.");
        emit changed();
        return;
    }
    emit captureRequested(m_mode, x, y, width, height);
    // The desktop's own shortcut has no portal request to wait on: the
    // capture seam (T-13.3b) owns the result, so close the overlay now. A
    // portal request stays active until `complete()` returns the URI.
    if (m_handle.isEmpty()) {
        reset();
        emit finished(true);
    }
}

void ScreenshotBridge::complete(const QString &uri)
{
    if (!m_active || uri.isEmpty())
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable && !handle.isEmpty()) {
        QDBusMessage message = QDBusMessage::createMethodCall(
            kService, kPath, kInterface, QStringLiteral("CompleteScreenshot"));
        message << handle << uri;
        QDBusConnection::sessionBus().asyncCall(message);
    }
#else
    Q_UNUSED(handle);
#endif
    reset();
    emit finished(true);
}

void ScreenshotBridge::cancelCurrent()
{
    if (!m_active || m_handle.isEmpty())
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    // Only a request that came from the running portal has a handle to
    // answer. The fixture and the desktop shortcut use `begin()`/`beginLocal`
    // with no service; calling the name there would make the bus try to
    // activate a backend that is not part of the test session.
    if (m_serviceAvailable) {
        // A message call needs no introspection, so it cannot block on a
        // presenter that is served by this same process (as the tests do).
        QDBusMessage message = QDBusMessage::createMethodCall(
            kService, kPath, kInterface, QStringLiteral("CancelScreenshot"));
        message << handle;
        QDBusConnection::sessionBus().asyncCall(message);
    }
#else
    Q_UNUSED(handle);
#endif
}

void ScreenshotBridge::cancel()
{
    if (!m_active)
        return;
    cancelCurrent();
    reset();
    emit finished(false);
}

void ScreenshotBridge::queryPending()
{
#if defined(QT_DBUS_LIB)
    if (!m_serviceAvailable)
        return;
    QDBusMessage message = QDBusMessage::createMethodCall(
        kService, kPath, kInterface, QStringLiteral("PendingScreenshots"));
    auto *watcher = new QDBusPendingCallWatcher(
        QDBusConnection::sessionBus().asyncCall(message), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            &ScreenshotBridge::onPendingReply);
#endif
}

void ScreenshotBridge::onPendingReply(QDBusPendingCallWatcher *watcher)
{
#if defined(QT_DBUS_LIB)
    watcher->deleteLater();
    const QDBusMessage reply = watcher->reply();
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty())
        return;
    const QStringList handles =
        Dragonfruit::PortalPresenter::pendingHandles(reply.arguments().first());
    for (const QString &handle : handles) {
        if (handle.isEmpty() || handle == m_handle)
            continue;
        QDBusMessage cancel = QDBusMessage::createMethodCall(
            kService, kPath, kInterface, QStringLiteral("CancelScreenshot"));
        cancel << handle;
        QDBusConnection::sessionBus().asyncCall(cancel);
    }
#else
    Q_UNUSED(watcher);
#endif
}

void ScreenshotBridge::reset()
{
    if (!m_active && m_handle.isEmpty() && m_mode.isEmpty())
        return;
    m_active = false;
    m_handle.clear();
    m_mode.clear();
    m_appId.clear();
    m_error.clear();
    emit changed();
}