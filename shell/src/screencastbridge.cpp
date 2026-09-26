// SPDX-License-Identifier: MIT
#include "screencastbridge.h"

#include <QVariantMap>

#if defined(QT_DBUS_LIB)
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusMetaType>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#endif

#if defined(QT_DBUS_LIB)
// The `a(su)` wire shape of `CompleteScreenCast`: the presenter's source
// handle and its type bit.
QDBusArgument &operator<<(QDBusArgument &argument, const ScreenCastSelection &selection)
{
    argument.beginStructure();
    argument << selection.id << selection.sourceType;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, ScreenCastSelection &selection)
{
    argument.beginStructure();
    argument >> selection.id >> selection.sourceType;
    argument.endStructure();
    return argument;
}
#endif

namespace {

// The portal backend's diagnostic presenter surface (T-13.4a).
const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");
const QString kInterface = QStringLiteral("org.dragonfruit.Portal1");

// The source type bits, mirrored from the portal model.
constexpr uint kMonitorType = 1;
constexpr uint kWindowType = 2;

// The type bit for one source row's `kind`.
uint typeBitFor(const QString &kind)
{
    return kind == QLatin1String("window") ? kWindowType : kMonitorType;
}

} // namespace

ScreenCastBridge::ScreenCastBridge(QObject *parent)
    : QObject(parent)
{
#if defined(QT_DBUS_LIB)
    qDBusRegisterMetaType<ScreenCastSelection>();
    qDBusRegisterMetaType<QList<ScreenCastSelection>>();
#endif
}

ScreenCastBridge::~ScreenCastBridge() = default;

int ScreenCastBridge::selectedCount() const
{
    int count = 0;
    for (const QVariant &entry : m_sources) {
        if (entry.toMap().value(QStringLiteral("selected")).toBool())
            ++count;
    }
    return count;
}

QString ScreenCastBridge::streamNote() const
{
    if (m_streamMode == QLatin1String("pipewire"))
        return QString();
    return tr("Live streaming is not available in this build — the app will receive still images.");
}

void ScreenCastBridge::connectService()
{
#if defined(QT_DBUS_LIB)
    if (m_connected)
        return;
    m_connected = true;
    QDBusConnection bus = QDBusConnection::sessionBus();
    bus.connect(kService, kPath, kInterface, QStringLiteral("ScreenCastOpened"), this,
                SLOT(onScreenCastOpened(QString, QString, QString, uint, bool, QVariantMap)));
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

void ScreenCastBridge::checkService()
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    m_serviceAvailable = iface && iface->isServiceRegistered(kService);
#else
    m_serviceAvailable = false;
#endif
    refreshStreamMode();
}

void ScreenCastBridge::refreshStreamMode()
{
    // The backend advertises whether it has a live producer. A missing service
    // or method is not an error: the stills fallback stays named. The call is
    // asynchronous so it can never stall the shell's event loop (and it can
    // reach a presenter served by this same process, as the tests do).
    if (!m_serviceAvailable) {
        setStreamMode(QStringLiteral("stills"));
        return;
    }
#if defined(QT_DBUS_LIB)
    QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
    if (!iface.isValid()) {
        setStreamMode(QStringLiteral("stills"));
        return;
    }
    QDBusPendingCall pending = iface.asyncCall(QStringLiteral("ScreenCastStreamMode"));
    auto *watcher = new QDBusPendingCallWatcher(pending, this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            &ScreenCastBridge::onStreamModeReply);
#endif
}

void ScreenCastBridge::onStreamModeReply(QDBusPendingCallWatcher *watcher)
{
#if defined(QT_DBUS_LIB)
    watcher->deleteLater();
    const QDBusPendingReply<QString> reply = *watcher;
    if (reply.isValid() && !reply.value().isEmpty())
        setStreamMode(reply.value());
#else
    Q_UNUSED(watcher);
#endif
}

void ScreenCastBridge::setStreamMode(const QString &mode)
{
    if (mode == m_streamMode)
        return;
    m_streamMode = mode;
    emit changed();
}

void ScreenCastBridge::onScreenCastOpened(const QString &handle, const QString &sessionHandle,
                                          const QString &appId, uint types, bool multiple,
                                          const QVariantMap &options)
{
    // `cursor_mode` travels in the raw options (the signal keeps its argument
    // count down); hidden (1) is the spec default.
    const uint cursorMode = options.value(QStringLiteral("cursor_mode")).toUInt();
    begin(handle, sessionHandle, appId, types, multiple, cursorMode == 0 ? 1 : cursorMode);
}

void ScreenCastBridge::begin(const QString &handle, const QString &sessionHandle,
                             const QString &appId, uint types, bool multiple, uint cursorMode)
{
    reset();
    m_active = true;
    m_handle = handle;
    m_sessionHandle = sessionHandle;
    m_appId = appId;
    m_types = types == 0 ? kMonitorType : types;
    m_multiple = multiple;
    m_cursorMode = cursorMode;
    emit started();
    emit changed();
}

void ScreenCastBridge::setSources(const QVariantList &sources)
{
    m_sources.clear();
    m_sources.reserve(sources.size());
    for (const QVariant &entry : sources) {
        QVariantMap source = entry.toMap();
        source.insert(QStringLiteral("selected"), false);
        m_sources.append(source);
    }
    m_error.clear();
    emit changed();
}

void ScreenCastBridge::select(const QString &id)
{
    if (!m_active || id.isEmpty())
        return;
    for (int index = 0; index < m_sources.size(); ++index) {
        QVariantMap source = m_sources.at(index).toMap();
        if (source.value(QStringLiteral("id")).toString() != id)
            continue;
        const bool selected = source.value(QStringLiteral("selected")).toBool();
        if (m_multiple) {
            source.insert(QStringLiteral("selected"), !selected);
        } else {
            // Single selection: clear every other row and select this one.
            for (int other = 0; other < m_sources.size(); ++other) {
                QVariantMap clear = m_sources.at(other).toMap();
                clear.insert(QStringLiteral("selected"), other == index);
                m_sources[other] = clear;
            }
            source.insert(QStringLiteral("selected"), true);
        }
        m_sources[index] = source;
        break;
    }
    m_error.clear();
    emit changed();
}

QList<ScreenCastSelection> ScreenCastBridge::selections() const
{
    QList<ScreenCastSelection> chosen;
    for (const QVariant &entry : m_sources) {
        const QVariantMap source = entry.toMap();
        if (!source.value(QStringLiteral("selected")).toBool())
            continue;
        ScreenCastSelection selection;
        selection.id = source.value(QStringLiteral("id")).toString();
        selection.sourceType = typeBitFor(source.value(QStringLiteral("kind")).toString());
        chosen.append(selection);
    }
    return chosen;
}

void ScreenCastBridge::accept()
{
    if (!m_active)
        return;
    const QList<ScreenCastSelection> chosen = selections();
    if (chosen.isEmpty()) {
        m_error = tr("Choose a screen or window to share.");
        emit changed();
        return;
    }
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable && !handle.isEmpty()) {
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CompleteScreenCast"), handle,
                            QVariant::fromValue(chosen));
    }
#else
    Q_UNUSED(handle);
#endif
    reset();
    emit finished(true);
}

void ScreenCastBridge::cancel()
{
    if (!m_active)
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable && !handle.isEmpty()) {
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CancelScreenCast"), handle);
    }
#else
    Q_UNUSED(handle);
#endif
    reset();
    emit finished(false);
}

void ScreenCastBridge::reset()
{
    if (!m_active && m_handle.isEmpty() && m_sources.isEmpty())
        return;
    m_active = false;
    m_handle.clear();
    m_sessionHandle.clear();
    m_appId.clear();
    m_types = kMonitorType;
    m_multiple = false;
    m_cursorMode = 1;
    m_sources.clear();
    m_error.clear();
    emit changed();
}