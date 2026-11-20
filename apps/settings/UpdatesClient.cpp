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
#include <QTimer>

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
    if (!bus.isConnected()) {
        // T291e: absence is a normal state, not a silent empty map. A bus-less
        // session (headless CI, a broken user session) publishes the absent
        // view once so the shell's status row says "Unavailable" instead of
        // keeping its neutral catalog label behind a read that never happens.
        // The bridge connects `changed` after this constructor returns, so the
        // read is deferred; `call()` reports the absent view from there. A
        // later bus or host appears through RuntimeAdded and the first write.
        QTimer::singleShot(0, this, [this]() { refresh(); });
        return;
    }
    m_available = available();
    if (!m_available) {
        // T291e: a connected bus without the host is the same absence as a
        // bus-less session. Publish the empty view now and once more after the
        // bridge has subscribed, so the shell's status row says "Unavailable";
        // the service watcher publishes the provider view when the host
        // appears.
        emit changed(m_view);
        QTimer::singleShot(0, this, [this]() {
            if (!m_available)
                emit changed(m_view);
        });
    }
    m_busyTimer = new QTimer(this);
    m_busyTimer->setInterval(1000);
    connect(m_busyTimer, &QTimer::timeout, this, &DbusUpdatesClient::refresh);
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
                if (m_busyTimer)
                    m_busyTimer->stop();
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
    call(QStringLiteral("Refresh"), {});
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

void DbusUpdatesClient::setAutoPolicy(bool autoCheck, bool autoInstall)
{
    call(QStringLiteral("SetAutomaticPolicy"), {autoCheck, autoInstall});
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
        // T291e: the absent host publishes one empty view so the shell's
        // status row resolves to "Unavailable" instead of waiting on a read
        // that never arrives.
        if (method == QStringLiteral("State") || method == QStringLiteral("Refresh"))
            emit changed(m_view);
        return;
    }
    const bool isState = method == QStringLiteral("State")
        || method == QStringLiteral("Refresh");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, method, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    if (!isState)
                        emit writeFinished(method, QStringLiteral("failed"),
                                           reply.error().message());
                    return;
                }
                if (isState) {
                    applyReply(reply.value().toUtf8());
                    return;
                }
                // T282: the write reply is the outcome report; the new state
                // is the host's re-read, never invented here.
                QString outcome = QStringLiteral("applied");
                QString message;
                const QJsonDocument document =
                    QJsonDocument::fromJson(reply.value().toUtf8());
                if (document.isObject()) {
                    const QJsonObject object = document.object();
                    outcome = object.value(QStringLiteral("outcome"))
                                  .toString(outcome);
                    message = object.value(QStringLiteral("error")).toString();
                    if (message.isEmpty())
                        message = object.value(QStringLiteral("note")).toString();
                }
                emit writeFinished(method, outcome, message);
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
    if (m_busyTimer) {
        if (m_view.value(QStringLiteral("busy")).toBool()) {
            if (!m_busyTimer->isActive())
                m_busyTimer->start();
        } else {
            m_busyTimer->stop();
        }
    }
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

void MockUpdatesClient::setAutoPolicy(bool autoCheck, bool autoInstall)
{
    m_autoCheck = autoCheck;
    m_autoInstall = autoInstall;
}

void MockUpdatesClient::resetForTest()
{
    m_phase = QStringLiteral("available");
    m_updateCount = 1;
    m_absent = false;
    m_providerAvailable = true;
    rebuild();
}

void MockUpdatesClient::setAbsentFixture(bool absent)
{
    if (m_absent == absent)
        return;
    m_absent = absent;
    emit availableChanged(!m_absent);
    emit changed(view());
}

void MockUpdatesClient::setProviderAvailableFixture(bool available)
{
    if (m_providerAvailable == available)
        return;
    m_providerAvailable = available;
    rebuild();
}

void MockUpdatesClient::rebuild()
{
    if (m_absent) {
        m_view.clear();
        emit changed(m_view);
        return;
    }
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
    view.insert(QStringLiteral("gpu"), QStringLiteral("Example GPU"));
    view.insert(QStringLiteral("gpuLabel"), QStringLiteral("Example GPU"));
    view.insert(QStringLiteral("diskBytes"), qlonglong(512) * 1024 * 1024 * 1024);
    view.insert(QStringLiteral("diskLabel"), QStringLiteral("512 GiB"));

    view.insert(QStringLiteral("updatesAvailable"), m_providerAvailable);
    view.insert(QStringLiteral("glyph"), QStringLiteral("software-update"));
    view.insert(QStringLiteral("phase"), m_phase);
    view.insert(QStringLiteral("busy"), false);
    view.insert(QStringLiteral("rebootRequired"),
                m_phase == QStringLiteral("reboot-required"));
    view.insert(QStringLiteral("updateCount"), m_updateCount);
    view.insert(QStringLiteral("securityCount"),
                m_phase == QStringLiteral("available") ? 1 : 0);
    view.insert(QStringLiteral("lastCheckedMs"), qlonglong(1763000000000));
    view.insert(QStringLiteral("label"),
                !m_providerAvailable
                    ? QStringLiteral("No update provider")
                : m_phase == QStringLiteral("reboot-required")
                    ? QStringLiteral("Restart Required")
                : m_updateCount > 0 ? QStringLiteral("1 Update Available")
                                    : QStringLiteral("Up to Date"));

    QVariantList updates;
    if (m_updateCount > 0 && m_providerAvailable) {
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