// SPDX-License-Identifier: MIT
#include "SettingsBridge.h"

#include "settingsclient.h"

#include <QColor>
#include <QDateTime>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusObjectPath>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QDBusVariant>
#include <QDir>
#include <cstdio>
#include <QFileInfo>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLinearGradient>
#include <QPainter>
#include <QStandardPaths>
#include <QUrl>

namespace {

// The app's own original wallpaper artwork (T-09.3): a small set of gradients,
// never Apple's. Each is rendered once to a PNG under the user's app-data
// directory so the compositor (another process) can decode it by path; the
// in-process preview uses the same file.
struct WallpaperPreset {
    const char *id;
    const char *name;
    const char *collection;
    const char *top;
    const char *bottom;
};

const WallpaperPreset kPresets[] = {
    { "dusk", "Dusk", "Dragonfruit", "#2b1055", "#7597de" },
    { "ember", "Ember", "Dragonfruit", "#3a1c1c", "#b3402a" },
    { "lagoon", "Lagoon", "Dragonfruit", "#06283d", "#47b5ff" },
    { "meadow", "Meadow", "Landscape", "#134e5e", "#71b280" },
    { "desert", "Desert", "Landscape", "#5a3f11", "#e0a43a" },
    { "twilight", "Twilight", "Landscape", "#0f2027", "#2c5364" },
};

constexpr int kPresetWidth = 960;
constexpr int kPresetHeight = 600;

// The wallpaper content provider (T-18.1b): the same session-bus surface the
// shell reads. Panes never touch it directly (ADR 0036); this bridge is their
// one accessor, and its absence is a normal state, never an error.
const QString kWallpaperService = QStringLiteral("org.dragonfruit.Wallpaper1");
const QString kWallpaperPath = QStringLiteral("/org/dragonfruit/Wallpaper1");
const QString kWallpaperInterface = QStringLiteral("org.dragonfruit.Wallpaper1");

// Render one original gradient preset in-memory (the same pixels the QML
// preview shows).
QImage renderGradient(const QColor &top, const QColor &bottom)
{
    QImage image(kPresetWidth, kPresetHeight, QImage::Format_RGBA8888);
    QPainter painter(&image);
    QLinearGradient gradient(0, 0, kPresetWidth, kPresetHeight);
    gradient.setColorAt(0.0, top);
    gradient.setColorAt(1.0, bottom);
    painter.fillRect(image.rect(), gradient);
    painter.end();
    return image;
}

} // namespace

SettingsBridge::SettingsBridge(QObject *parent)
    : QObject(parent)
{
    // The Settings app is a consumer, not an owner: it links the exact client
    // the shell uses (ADR 0036). `DF_SETTINGS_FIXTURE` swaps in the
    // deterministic in-process mock for headless QML tests and captures, the
    // same way `DF_STATUS_FIXTURE` selects the status fixture.
    if (qEnvironmentVariableIsSet("DF_SETTINGS_FIXTURE"))
        m_client = new MockSettingsClient(this);
    else
        m_client = new DbusSettingsClient(this);

    connect(m_client, &SettingsClient::changed, this,
            [this](const QString &key, const QVariant &value) {
                emit valuesChanged();
                emit changed(key, value);
            });
    connect(m_client, &SettingsClient::availableChanged, this,
            [this](bool available) { emit availableChanged(available); });

    buildWallpaperPresets();
    connectPortalWatcher();
    connectWallpaperProvider();
}

SettingsBridge::~SettingsBridge() = default;

QVariantMap SettingsBridge::values() const
{
    return m_client->values();
}

bool SettingsBridge::available() const
{
    return m_client->isAvailable();
}

QVariantList SettingsBridge::wallpaperPresets() const
{
    return m_presets;
}

QVariantList SettingsBridge::providerItems() const
{
    return m_providerItems;
}

QString SettingsBridge::providerStatus() const
{
    return m_providerStatus;
}

QString SettingsBridge::providerDefault() const
{
    return m_providerDefault;
}

QString SettingsBridge::wallpaperBuiltinDefault() const
{
    // The provider's own resolution wins; absent, the shared resolver still
    // gives the shipped asset so the Built-in row is never empty (ADR 0094).
    return m_providerBuiltin.isEmpty() ? shippedDefaultWallpaperPath() : m_providerBuiltin;
}

bool SettingsBridge::wallpaperChooserAvailable() const
{
    return m_chooserAvailable;
}

QString SettingsBridge::startPane() const
{
    return qEnvironmentVariable("DF_SETTINGS_START_PANE");
}

bool SettingsBridge::trace() const
{
    return qEnvironmentVariableIsSet("DF_SETTINGS_TRACE");
}

QVariant SettingsBridge::value(const QString &key, const QVariant &fallback) const
{
    return m_client->value(key, fallback);
}

void SettingsBridge::set(const QString &key, const QVariant &value)
{
    m_client->set(key, value);
}

void SettingsBridge::refresh()
{
    m_client->refresh();
}

void SettingsBridge::traceLog(const QString &message) const
{
    if (!trace())
        return;
    std::fprintf(stderr, "DFTRACE %s t=%lld\n", qPrintable(message),
                 static_cast<long long>(QDateTime::currentMSecsSinceEpoch()));
}

QStringList SettingsBridge::keys() const
{
    return m_client->values().keys();
}

QString SettingsBridge::localPathFromUri(const QString &uri)
{
    const QUrl url(uri);
    if (url.isLocalFile())
        return url.toLocalFile();
    if (url.scheme().isEmpty())
        return QFileInfo(uri).isAbsolute() ? uri : QString();
    return QString();
}

void SettingsBridge::buildWallpaperPresets()
{
    QString directory =
        QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    if (directory.isEmpty())
        directory = QDir::tempPath() + QStringLiteral("/dragonfruit-wallpapers");
    directory += QStringLiteral("/wallpapers");
    if (!QDir().mkpath(directory))
        directory = QDir::tempPath() + QStringLiteral("/dragonfruit-wallpapers");
    QDir().mkpath(directory);

    m_presets.clear();
    for (const WallpaperPreset &preset : kPresets) {
        const QString path =
            directory + QLatin1Char('/') + QString::fromLatin1(preset.id)
            + QStringLiteral(".png");
        if (!QFileInfo::exists(path)) {
            const QImage image = renderGradient(QColor(QString::fromLatin1(preset.top)),
                                                QColor(QString::fromLatin1(preset.bottom)));
            image.save(path, "PNG");
        }
        const QUrl url = QUrl::fromLocalFile(path);
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), QString::fromLatin1(preset.id));
        entry.insert(QStringLiteral("name"), QString::fromUtf8(preset.name));
        entry.insert(QStringLiteral("collection"), QString::fromUtf8(preset.collection));
        entry.insert(QStringLiteral("source"), path);
        entry.insert(QStringLiteral("url"), url.toString());
        m_presets.append(entry);
    }
}

void SettingsBridge::connectWallpaperProvider()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    // A provider restart re-reads the catalogue; an unregistration keeps the
    // last known values so the pane is not visibly reset (restart/resync).
    m_providerWatcher = new QDBusServiceWatcher(
        kWallpaperService, bus, QDBusServiceWatcher::WatchForRegistration, this);
    connect(m_providerWatcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) { refreshWallpaperProvider(); });
    bus.connect(kWallpaperService, kWallpaperPath,
                QStringLiteral("org.freedesktop.DBus.Properties"),
                QStringLiteral("PropertiesChanged"), this,
                SLOT(onProviderPropertiesChanged(QString, QVariantMap, QStringList)));
    // The contract signal covers a first fill whose PropertiesChanged the
    // bridge may have missed; a refresh re-reads every property cheaply.
    bus.connect(kWallpaperService, kWallpaperPath, kWallpaperInterface,
                QStringLiteral("ItemsChanged"), this,
                SLOT(onProviderItemsChanged()));
    refreshWallpaperProvider();
}

void SettingsBridge::refreshWallpaperProvider()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    const auto readProperty = [&bus](const QString &name) -> QString {
        QDBusMessage call = QDBusMessage::createMethodCall(
            kWallpaperService, kWallpaperPath,
            QStringLiteral("org.freedesktop.DBus.Properties"), QStringLiteral("Get"));
        call << kWallpaperInterface << name;
        const QDBusMessage reply = bus.call(call);
        if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
            return QString();
        QVariant value = reply.arguments().constFirst();
        if (value.userType() == qMetaTypeId<QDBusVariant>())
            value = value.value<QDBusVariant>().variant();
        return value.toString();
    };
    applyProviderItems(readProperty(QStringLiteral("Items")));
    applyProviderStatus(readProperty(QStringLiteral("Status")));
    applyProviderDefault(readProperty(QStringLiteral("DefaultSource")));
    applyProviderBuiltin(readProperty(QStringLiteral("BuiltinDefaultSource")));
}

void SettingsBridge::applyProviderItems(const QString &json)
{
    const QJsonArray array = QJsonDocument::fromJson(json.toUtf8()).array();
    QVariantList items;
    items.reserve(array.size());
    for (const QJsonValue &value : array) {
        QVariantMap entry = value.toObject().toVariantMap();
        const QString localPath = entry.value(QStringLiteral("localPath")).toString();
        entry.insert(QStringLiteral("source"), localPath);
        entry.insert(QStringLiteral("url"),
                     localPath.isEmpty() ? QString()
                                         : QUrl::fromLocalFile(localPath).toString());
        items.append(entry);
    }
    if (items == m_providerItems)
        return;
    m_providerItems = items;
    emit providerChanged();
}

void SettingsBridge::applyProviderStatus(const QString &status)
{
    if (status == m_providerStatus)
        return;
    m_providerStatus = status;
    emit providerChanged();
}

void SettingsBridge::applyProviderDefault(const QString &path)
{
    if (path == m_providerDefault)
        return;
    m_providerDefault = path;
    emit providerChanged();
}

void SettingsBridge::applyProviderBuiltin(const QString &path)
{
    if (path == m_providerBuiltin)
        return;
    m_providerBuiltin = path;
    emit providerChanged();
}

void SettingsBridge::onProviderItemsChanged()
{
    refreshWallpaperProvider();
}

void SettingsBridge::onProviderPropertiesChanged(const QString &interface,
                                                 const QVariantMap &changed,
                                                 const QStringList &invalidated)
{
    Q_UNUSED(interface);
    Q_UNUSED(invalidated);
    if (changed.contains(QStringLiteral("Items")))
        applyProviderItems(changed.value(QStringLiteral("Items")).toString());
    if (changed.contains(QStringLiteral("Status")))
        applyProviderStatus(changed.value(QStringLiteral("Status")).toString());
    if (changed.contains(QStringLiteral("DefaultSource")))
        applyProviderDefault(changed.value(QStringLiteral("DefaultSource")).toString());
    if (changed.contains(QStringLiteral("BuiltinDefaultSource")))
        applyProviderBuiltin(changed.value(QStringLiteral("BuiltinDefaultSource")).toString());
}

void SettingsBridge::preloadWallpapers()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    const QDBusMessage call = QDBusMessage::createMethodCall(
        kWallpaperService, kWallpaperPath, kWallpaperInterface, QStringLiteral("Preload"));
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *call) {
                call->deleteLater();
                // Absent provider: the reply is an error, which is ignored; the
                // properties stay empty and no error reaches the pane.
                refreshWallpaperProvider();
            });
}

void SettingsBridge::connectPortalWatcher()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    QDBusConnectionInterface *interface = bus.interface();
    if (!interface)
        return;
    const QString portal = QStringLiteral("org.freedesktop.portal.Desktop");
    setChooserAvailable(interface->isServiceRegistered(portal));
    connect(interface, &QDBusConnectionInterface::serviceRegistered, this,
            [this, portal](const QString &name) {
                if (name == portal)
                    setChooserAvailable(true);
            });
    connect(interface, &QDBusConnectionInterface::serviceUnregistered, this,
            [this, portal](const QString &name) {
                if (name == portal)
                    setChooserAvailable(false);
            });
}

void SettingsBridge::setChooserAvailable(bool available)
{
    if (m_chooserAvailable == available)
        return;
    m_chooserAvailable = available;
    emit wallpaperChooserAvailableChanged();
}

void SettingsBridge::chooseWallpaperPhoto()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected() || !m_chooserAvailable)
        return;

    QDBusMessage message = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.portal.Desktop"),
        QStringLiteral("/org/freedesktop/portal/desktop"),
        QStringLiteral("org.freedesktop.portal.FileChooser"),
        QStringLiteral("OpenFile"));
    QVariantMap options;
    options.insert(QStringLiteral("multiple"), false);
    options.insert(QStringLiteral("handle_token"), QStringLiteral("dragonfruit_wallpaper"));
    message << QString() << options;

    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(message), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *call) {
                call->deleteLater();
                if (call->isError())
                    return;
                const QDBusMessage reply = call->reply();
                if (reply.arguments().isEmpty())
                    return;
                m_requestPath =
                    reply.arguments().constFirst().value<QDBusObjectPath>().path();
                QDBusConnection::sessionBus().connect(
                    QStringLiteral("org.freedesktop.portal.Desktop"), m_requestPath,
                    QStringLiteral("org.freedesktop.portal.Request"),
                    QStringLiteral("Response"), this,
                    SLOT(onPortalResponse(uint, QVariantMap)));
            });
}

void SettingsBridge::onPortalResponse(uint response, const QVariantMap &results)
{
    // Response 0 is success; 1 is user cancellation and 2 an error.
    if (response != 0)
        return;
    QVariant uris = results.value(QStringLiteral("uris"));
    if (uris.userType() == qMetaTypeId<QDBusVariant>())
        uris = uris.value<QDBusVariant>().variant();
    const QStringList list = uris.toStringList();
    if (list.isEmpty())
        return;
    const QString path = localPathFromUri(list.constFirst());
    if (!path.isEmpty())
        emit wallpaperPhotoChosen(path);
}