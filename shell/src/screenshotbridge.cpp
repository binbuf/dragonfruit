// SPDX-License-Identifier: MIT
#include "screenshotbridge.h"

#if defined(QT_DBUS_LIB)
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
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
    m_serviceAvailable = iface && iface->isServiceRegistered(kService);
#else
    m_serviceAvailable = false;
#endif
}

void ScreenshotBridge::onScreenshotOpened(const QString &handle, const QString &mode,
                                          const QString &appId, const QString &parentWindow,
                                          const QVariantMap &options)
{
    Q_UNUSED(options);
    begin(handle, mode, appId, parentWindow);
}

void ScreenshotBridge::begin(const QString &handle, const QString &mode, const QString &appId,
                            const QString &parentWindow)
{
    Q_UNUSED(parentWindow);
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
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CompleteScreenshot"), handle, uri);
    }
#else
    Q_UNUSED(handle);
#endif
    reset();
    emit finished(true);
}

void ScreenshotBridge::cancel()
{
    if (!m_active)
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable && !handle.isEmpty()) {
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CancelScreenshot"), handle);
    }
#else
    Q_UNUSED(handle);
#endif
    reset();
    emit finished(false);
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