// SPDX-License-Identifier: MIT
#include "UpdatesClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Updates");

} // namespace

// --- DbusUpdatesClient -----------------------------------------------------

DbusUpdatesClient::DbusUpdatesClient(QObject *parent)
    : UpdatesClient(parent)
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

DbusUpdatesClient::~DbusUpdatesClient() = default;

bool DbusUpdatesClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusUpdatesClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusUpdatesClient::check()
{
    call(QStringLiteral("Check"), {});
}

void DbusUpdatesClient::install()
{
    call(QStringLiteral("Install"), {});
}

void DbusUpdatesClient::reboot()
{
    call(QStringLiteral("Reboot"), {});
}

void DbusUpdatesClient::call(const QString &method, const QVariantList &arguments)
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
    const bool isState = method == QStringLiteral("State");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    refresh();
            });
}

void DbusUpdatesClient::applyReply(const QByteArray &json)
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

// --- MockUpdatesClient -----------------------------------------------------

MockUpdatesClient::MockUpdatesClient(QObject *parent)
    : UpdatesClient(parent)
{
    rebuild();
}

void MockUpdatesClient::refresh()
{
    rebuild();
}

void MockUpdatesClient::check()
{
    m_phase = m_updateCount > 0 ? QStringLiteral("available")
                                : QStringLiteral("up-to-date");
    rebuild();
}

void MockUpdatesClient::install()
{
    if (m_updateCount > 0) {
        m_updateCount = 0;
        m_phase = QStringLiteral("reboot-required");
    } else {
        m_phase = QStringLiteral("up-to-date");
    }
    rebuild();
}

void MockUpdatesClient::reboot()
{
    m_phase = QStringLiteral("up-to-date");
    rebuild();
}

void MockUpdatesClient::resetForTest()
{
    m_phase = QStringLiteral("available");
    m_updateCount = 1;
    rebuild();
}

void MockUpdatesClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("updates"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("hostName"), QStringLiteral("dragon"));
    view.insert(QStringLiteral("deviceName"), QStringLiteral("Dragonfruit Book"));
    view.insert(QStringLiteral("osLabel"), QStringLiteral("Dragonfruit Linux 44"));
    view.insert(QStringLiteral("kernel"), QStringLiteral("6.12.0"));
    view.insert(QStringLiteral("architecture"), QStringLiteral("x86_64"));
    view.insert(QStringLiteral("processor"), QStringLiteral("Example CPU"));
    view.insert(QStringLiteral("memoryLabel"), QStringLiteral("16 GB"));
    view.insert(QStringLiteral("serial"), QStringLiteral("SERIAL-1"));
    view.insert(QStringLiteral("hasSerial"), true);

    view.insert(QStringLiteral("updatesAvailable"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("software-update"));
    view.insert(QStringLiteral("phase"), m_phase);
    view.insert(QStringLiteral("busy"), false);
    view.insert(QStringLiteral("rebootRequired"),
                m_phase == QStringLiteral("reboot-required"));
    view.insert(QStringLiteral("updateCount"), m_updateCount);
    view.insert(QStringLiteral("securityCount"),
                m_phase == QStringLiteral("available") ? 1 : 0);
    view.insert(QStringLiteral("lastCheckedMs"), 1000);
    view.insert(QStringLiteral("label"),
                m_phase == QStringLiteral("reboot-required")
                    ? QStringLiteral("Restart Required")
                : m_updateCount > 0 ? QStringLiteral("1 Update Available")
                                    : QStringLiteral("Up to Date"));

    QVariantList updates;
    if (m_updateCount > 0) {
        QVariantMap update;
        update.insert(QStringLiteral("id"), QStringLiteral("glibc"));
        update.insert(QStringLiteral("name"), QStringLiteral("glibc"));
        update.insert(QStringLiteral("summary"), QStringLiteral("C library"));
        update.insert(QStringLiteral("currentVersion"), QStringLiteral("2.40"));
        update.insert(QStringLiteral("availableVersion"), QStringLiteral("2.41"));
        update.insert(QStringLiteral("severity"), QStringLiteral("security"));
        update.insert(QStringLiteral("severityLabel"),
                      QStringLiteral("Security Update"));
        updates.append(update);
    }
    view.insert(QStringLiteral("updates"), updates);
    m_view = view;
    emit changed(m_view);
}