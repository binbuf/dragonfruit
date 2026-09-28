// SPDX-License-Identifier: MIT
#include "AccessibilityClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Accessibility");

} // namespace

// --- DbusAccessibilityClient -----------------------------------------------

DbusAccessibilityClient::DbusAccessibilityClient(QObject *parent)
    : AccessibilityClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_available = available();
    m_watcher = new QDBusServiceWatcher(
        m_service, bus, QDBusServiceWatcher::WatchForRegistration
                          | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) {
                m_available = true;
                emit availableChanged(true);
                refresh();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
            });
    refresh();
}

DbusAccessibilityClient::~DbusAccessibilityClient() = default;

bool DbusAccessibilityClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusAccessibilityClient::refresh()
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(QStringLiteral("State"), {}), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                applyReply(reply.value().toUtf8());
            });
}

void DbusAccessibilityClient::applyReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_view)
        return;
    m_view = view;
    emit changed(m_view);
}

// --- MockAccessibilityClient -----------------------------------------------

MockAccessibilityClient::MockAccessibilityClient(QObject *parent)
    : AccessibilityClient(parent)
{
    resetForTest();
}

void MockAccessibilityClient::resetForTest()
{
    m_enabled = true;
    m_screenReader = true;
    rebuild();
}

void MockAccessibilityClient::refresh()
{
    rebuild();
}

void MockAccessibilityClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("present"), m_enabled || m_screenReader);
    view.insert(QStringLiteral("enabled"), m_enabled);
    view.insert(QStringLiteral("enabledLabel"),
                m_enabled ? QStringLiteral("On") : QStringLiteral("Off"));
    view.insert(QStringLiteral("screenReader"), m_screenReader);
    view.insert(QStringLiteral("screenReaderLabel"),
                m_screenReader ? QStringLiteral("On") : QStringLiteral("Off"));
    view.insert(QStringLiteral("label"),
                m_screenReader ? QStringLiteral("Screen Reader On")
                               : (m_enabled ? QStringLiteral("On")
                                            : QStringLiteral("Off")));
    m_view = view;
    emit changed(m_view);
}