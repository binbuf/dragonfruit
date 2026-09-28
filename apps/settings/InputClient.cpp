// SPDX-License-Identifier: MIT
#include "InputClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Input");

} // namespace

// --- DbusInputClient -------------------------------------------------------

DbusInputClient::DbusInputClient(QObject *parent)
    : InputClient(parent)
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

DbusInputClient::~DbusInputClient() = default;

bool DbusInputClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusInputClient::refresh()
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
        iface->asyncCall(QStringLiteral("State")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (reply.isValid())
                    applyReply(reply.value().toUtf8());
            });
}

void DbusInputClient::applyReply(const QByteArray &json)
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

// --- MockInputClient -------------------------------------------------------

MockInputClient::MockInputClient(QObject *parent)
    : InputClient(parent)
{
    rebuild();
}

void MockInputClient::refresh()
{
    rebuild();
}

void MockInputClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("input"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("keyboard"));
    view.insert(QStringLiteral("label"),
                QStringLiteral("1 keyboards, 2 pointing devices"));
    view.insert(QStringLiteral("deviceCount"), 3);
    view.insert(QStringLiteral("keyboardCount"), 1);
    view.insert(QStringLiteral("pointerCount"), 2);
    view.insert(QStringLiteral("mouseCount"), 1);
    view.insert(QStringLiteral("touchpadCount"), 1);

    const auto device = [](const QString &name, const QString &kernel,
                           const QString &kind, const QString &kindLabel,
                           const QString &glyph) {
        QVariantMap entry;
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("kernel"), kernel);
        entry.insert(QStringLiteral("kind"), kind);
        entry.insert(QStringLiteral("kindLabel"), kindLabel);
        entry.insert(QStringLiteral("glyph"), glyph);
        entry.insert(QStringLiteral("seat"), QStringLiteral("seat0, default"));
        return entry;
    };

    QVariantList devices;
    devices.append(device(QStringLiteral("AT Translated Set 2 keyboard"),
                          QStringLiteral("/dev/input/event3"),
                          QStringLiteral("keyboard"), QStringLiteral("Keyboard"),
                          QStringLiteral("keyboard")));
    devices.append(device(QStringLiteral("Logitech USB Mouse"),
                          QStringLiteral("/dev/input/event9"),
                          QStringLiteral("mouse"), QStringLiteral("Mouse"),
                          QStringLiteral("mouse")));
    devices.append(device(QStringLiteral("Synaptics TouchPad"),
                          QStringLiteral("/dev/input/event7"),
                          QStringLiteral("touchpad"), QStringLiteral("Trackpad"),
                          QStringLiteral("trackpad")));
    view.insert(QStringLiteral("devices"), devices);
    m_view = view;
    emit changed(m_view);
}