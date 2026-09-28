// SPDX-License-Identifier: MIT
#include "SoundClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Audio");

} // namespace

// --- DbusSoundClient -------------------------------------------------------

DbusSoundClient::DbusSoundClient(QObject *parent)
    : SoundClient(parent)
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

DbusSoundClient::~DbusSoundClient() = default;

bool DbusSoundClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusSoundClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusSoundClient::setVolume(double volume)
{
    call(QStringLiteral("SetVolume"), {volume});
}

void DbusSoundClient::setMute(bool muted)
{
    call(QStringLiteral("SetMute"), {muted});
}

void DbusSoundClient::setDefaultSink(int id)
{
    call(QStringLiteral("SetDefaultSink"), {id});
}

void DbusSoundClient::setDefaultSource(int id)
{
    call(QStringLiteral("SetDefaultSource"), {id});
}

void DbusSoundClient::call(const QString &method, const QVariantList &arguments)
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

void DbusSoundClient::applyReply(const QByteArray &json)
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

// --- MockSoundClient -------------------------------------------------------

MockSoundClient::MockSoundClient(QObject *parent)
    : SoundClient(parent)
{
    rebuild();
}

void MockSoundClient::refresh()
{
    rebuild();
}

void MockSoundClient::setVolume(double volume)
{
    m_volume = qBound(0.0, volume, 1.0);
    if (m_volume > 0.0)
        m_muted = false;
    rebuild();
}

void MockSoundClient::setMute(bool muted)
{
    m_muted = muted;
    rebuild();
}

void MockSoundClient::setDefaultSink(int id)
{
    if (id == 7 || id == 9)
        m_defaultSinkId = id;
    rebuild();
}

void MockSoundClient::setDefaultSource(int id)
{
    if (id == 20 || id == 21)
        m_defaultSourceId = id;
    rebuild();
}

void MockSoundClient::rebuild()
{
    const int percent = m_muted ? 0 : qRound(m_volume * 100.0);
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("audio"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"),
                (m_muted || m_volume <= 0.0) ? QStringLiteral("volume-muted")
                                             : QStringLiteral("volume"));
    view.insert(QStringLiteral("label"),
                m_muted ? QStringLiteral("Muted")
                        : QStringLiteral("%1%").arg(percent));
    view.insert(QStringLiteral("volume"), m_volume);
    view.insert(QStringLiteral("percent"), percent);
    view.insert(QStringLiteral("muted"), m_muted);
    view.insert(QStringLiteral("defaultSink"),
                m_defaultSinkId == 9 ? QStringLiteral("headphones")
                                     : QStringLiteral("speakers"));
    view.insert(QStringLiteral("defaultSource"),
                m_defaultSourceId == 21 ? QStringLiteral("usb-mic")
                                        : QStringLiteral("microphone"));

    const auto device = [](int id, const QString &name, const QString &description,
                           double volume, bool muted, bool isDefault) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("description"), description);
        entry.insert(QStringLiteral("volume"), volume);
        entry.insert(QStringLiteral("percent"), qRound(volume * 100.0));
        entry.insert(QStringLiteral("muted"), muted);
        entry.insert(QStringLiteral("default"), isDefault);
        return entry;
    };

    QVariantList sinks;
    sinks.append(device(7, QStringLiteral("speakers"), QStringLiteral("Built-in Speakers"),
                        m_volume, m_muted, m_defaultSinkId == 7));
    sinks.append(device(9, QStringLiteral("headphones"), QStringLiteral("Headphones"),
                        m_volume, m_muted, m_defaultSinkId == 9));
    view.insert(QStringLiteral("sinks"), sinks);

    QVariantList sources;
    sources.append(device(20, QStringLiteral("microphone"),
                          QStringLiteral("Built-in Microphone"), 0.5, false,
                          m_defaultSourceId == 20));
    sources.append(device(21, QStringLiteral("usb-mic"), QStringLiteral("USB Microphone"),
                          0.5, false, m_defaultSourceId == 21));
    view.insert(QStringLiteral("sources"), sources);
    view.insert(QStringLiteral("sinkCount"), sinks.size());
    view.insert(QStringLiteral("sourceCount"), sources.size());
    m_view = view;
    emit changed(m_view);
}