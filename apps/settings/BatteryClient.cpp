// SPDX-License-Identifier: MIT
#include "BatteryClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Battery");

} // namespace

// --- DbusBatteryClient -----------------------------------------------------

DbusBatteryClient::DbusBatteryClient(QObject *parent)
    : BatteryClient(parent)
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

DbusBatteryClient::~DbusBatteryClient() = default;

bool DbusBatteryClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusBatteryClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusBatteryClient::setActiveProfile(const QString &profile)
{
    if (!profile.isEmpty())
        call(QStringLiteral("SetActiveProfile"), {profile});
}

void DbusBatteryClient::call(const QString &method, const QVariantList &arguments)
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

void DbusBatteryClient::applyReply(const QByteArray &json)
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

// --- MockBatteryClient -----------------------------------------------------

MockBatteryClient::MockBatteryClient(QObject *parent)
    : BatteryClient(parent)
{
    rebuild();
}

void MockBatteryClient::refresh()
{
    rebuild();
}

void MockBatteryClient::setActiveProfile(const QString &profile)
{
    if (profile == QStringLiteral("power-saver")
            || profile == QStringLiteral("balanced")
            || profile == QStringLiteral("performance"))
        m_activeProfile = profile;
    rebuild();
}

void MockBatteryClient::rebuild()
{
    const auto profile = [this](const QString &id, const QString &label,
                                const QString &glyph) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("label"), label);
        entry.insert(QStringLiteral("glyph"), glyph);
        entry.insert(QStringLiteral("active"), id == m_activeProfile);
        return entry;
    };

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("battery"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("battery"));
    view.insert(QStringLiteral("label"), QStringLiteral("71%"));
    view.insert(QStringLiteral("percent"), 71);
    view.insert(QStringLiteral("level"), 0.71);
    view.insert(QStringLiteral("charging"), false);
    view.insert(QStringLiteral("plugged"), false);
    view.insert(QStringLiteral("onBattery"), true);
    view.insert(QStringLiteral("chargeState"), QStringLiteral("discharging"));
    view.insert(QStringLiteral("health"), QStringLiteral("normal"));
    view.insert(QStringLiteral("healthLabel"), QStringLiteral("Normal"));
    view.insert(QStringLiteral("capacity"), 96);
    view.insert(QStringLiteral("chargeCycles"), 112);
    view.insert(QStringLiteral("timeToEmpty"), 7200);
    view.insert(QStringLiteral("timeToFull"), QVariant());
    view.insert(QStringLiteral("profilesAvailable"), true);
    view.insert(QStringLiteral("activeProfile"), m_activeProfile);
    const QString activeLabel = m_activeProfile == QStringLiteral("power-saver")
        ? QStringLiteral("Power Saver")
        : m_activeProfile == QStringLiteral("performance")
            ? QStringLiteral("Performance") : QStringLiteral("Balanced");
    view.insert(QStringLiteral("profileLabel"), activeLabel);
    view.insert(QStringLiteral("profiles"), QVariantList{
        profile(QStringLiteral("power-saver"), QStringLiteral("Power Saver"),
                QStringLiteral("power-saver")),
        profile(QStringLiteral("balanced"), QStringLiteral("Balanced"),
                QStringLiteral("power-balanced")),
        profile(QStringLiteral("performance"), QStringLiteral("Performance"),
                QStringLiteral("power-performance")),
    });
    m_view = view;
    emit changed(m_view);
}