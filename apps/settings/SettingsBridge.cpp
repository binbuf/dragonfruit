// SPDX-License-Identifier: MIT
#include "SettingsBridge.h"

#include "AccessibilityClient.h"
#include "AccountsClient.h"
#include "BatteryClient.h"
#include "BluetoothClient.h"
#include "InputClient.h"
#include "NotificationsClient.h"
#include "PrintersClient.h"
#include "PrivacyClient.h"
#include "SoundClient.h"
#include "StorageClient.h"
#include "UpdatesClient.h"
#include "VpnClient.h"
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
#include <QRegularExpression>
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

// The deterministic provider catalogue used by `DF_WALLPAPER_FIXTURE` (mirrors
// `DF_SETTINGS_FIXTURE`). The first entry's `artist` deliberately carries HTML
// so the pane's sanitization is exercised; both `localPath`s are scratch paths
// the Image loader never has to resolve for the tests to pass.
const char *kWallpaperFixtureItems = R"JSON([
  {"pageid":1,"title":"File:Alpine Lake.jpg","artist":"<a href=\"https://example.org/Alice\">Alice Example</a>","licenseShortName":"CC BY-SA 4.0","licenseUrl":"https://creativecommons.org/licenses/by-sa/4.0/","pageUrl":"https://commons.wikimedia.org/wiki/File:Alpine_Lake.jpg","description":"An alpine lake","category":"nature","width":3840,"height":2160,"mime":"image/jpeg","sourceUrl":"https://example.org/a.jpg","localPath":"/tmp/dragonfruit-fixture-nature.jpg","fetchedAt":1},
  {"pageid":2,"title":"File:Ocean Wave.jpg","artist":"Bob","licenseShortName":"CC BY 4.0","licenseUrl":"https://creativecommons.org/licenses/by/4.0/","pageUrl":"https://commons.wikimedia.org/wiki/File:Ocean_Wave.jpg","description":"An ocean wave","category":"water","width":3840,"height":2160,"mime":"image/jpeg","sourceUrl":"https://example.org/b.jpg","localPath":"/tmp/dragonfruit-fixture-water.jpg","fetchedAt":2}
])JSON";

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

    // T-15.1b: the Bluetooth seam. `DF_BLUETOOTH_FIXTURE` selects the
    // deterministic in-process client for the headless pane tests; otherwise
    // the live bridge-host client, whose absence is a normal state.
    if (qEnvironmentVariableIsSet("DF_BLUETOOTH_FIXTURE"))
        m_bluetooth = new MockBluetoothClient(this);
    else
        m_bluetooth = new DbusBluetoothClient(this);
    connect(m_bluetooth, &BluetoothClient::changed, this,
            [this](const QVariantMap &) { emit bluetoothChanged(); });
    connect(m_bluetooth, &BluetoothClient::availableChanged, this,
            [this](bool) { emit bluetoothChanged(); });

    // T-15.2b: the storage seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_STORAGE_FIXTURE"))
        m_storage = new MockStorageClient(this);
    else
        m_storage = new DbusStorageClient(this);
    connect(m_storage, &StorageClient::changed, this,
            [this](const QVariantMap &) { emit storageChanged(); });
    connect(m_storage, &StorageClient::availableChanged, this,
            [this](bool) { emit storageChanged(); });

    // T-15.3b: the Sound seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_SOUND_FIXTURE"))
        m_sound = new MockSoundClient(this);
    else
        m_sound = new DbusSoundClient(this);
    connect(m_sound, &SoundClient::changed, this,
            [this](const QVariantMap &) { emit soundChanged(); });
    connect(m_sound, &SoundClient::availableChanged, this,
            [this](bool) { emit soundChanged(); });

    // T-15.4b: the input inventory seam, selected the same way. Read-only.
    if (qEnvironmentVariableIsSet("DF_INPUT_FIXTURE"))
        m_input = new MockInputClient(this);
    else
        m_input = new DbusInputClient(this);
    connect(m_input, &InputClient::changed, this,
            [this](const QVariantMap &) { emit inputChanged(); });
    connect(m_input, &InputClient::availableChanged, this,
            [this](bool) { emit inputChanged(); });

    // T-15.6b: the battery / power-profiles seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_BATTERY_FIXTURE"))
        m_battery = new MockBatteryClient(this);
    else
        m_battery = new DbusBatteryClient(this);
    connect(m_battery, &BatteryClient::changed, this,
            [this](const QVariantMap &) { emit batteryChanged(); });
    connect(m_battery, &BatteryClient::availableChanged, this,
            [this](bool) { emit batteryChanged(); });

    // T-15.7b: the Notifications/Focus seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_NOTIFICATIONS_FIXTURE"))
        m_notifications = new MockNotificationsClient(this);
    else
        m_notifications = new DbusNotificationsClient(this);
    connect(m_notifications, &NotificationsClient::changed, this,
            [this](const QVariantMap &) { emit notificationsChanged(); });
    connect(m_notifications, &NotificationsClient::availableChanged, this,
            [this](bool) { emit notificationsChanged(); });

    // T-15.10b: the General/About/Updates seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_UPDATES_FIXTURE"))
        m_updates = new MockUpdatesClient(this);
    else
        m_updates = new DbusUpdatesClient(this);
    connect(m_updates, &UpdatesClient::changed, this,
            [this](const QVariantMap &) { emit updatesChanged(); });
    connect(m_updates, &UpdatesClient::availableChanged, this,
            [this](bool) { emit updatesChanged(); });

    // T-15.11b: the Users & Groups seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_ACCOUNTS_FIXTURE"))
        m_accounts = new MockAccountsClient(this);
    else
        m_accounts = new DbusAccountsClient(this);
    connect(m_accounts, &AccountsClient::changed, this,
            [this](const QVariantMap &) { emit accountsChanged(); });
    connect(m_accounts, &AccountsClient::availableChanged, this,
            [this](bool) { emit accountsChanged(); });

    // T-15.12b: the Printers & Scanners seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_PRINTERS_FIXTURE"))
        m_printers = new MockPrintersClient(this);
    else
        m_printers = new DbusPrintersClient(this);
    connect(m_printers, &PrintersClient::changed, this,
            [this](const QVariantMap &) { emit printersChanged(); });
    connect(m_printers, &PrintersClient::availableChanged, this,
            [this](bool) { emit printersChanged(); });

    // T-15.13b: the Privacy & Security seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_PRIVACY_FIXTURE"))
        m_privacy = new MockPrivacyClient(this);
    else
        m_privacy = new DbusPrivacyClient(this);
    connect(m_privacy, &PrivacyClient::changed, this,
            [this](const QVariantMap &) { emit privacyChanged(); });
    connect(m_privacy, &PrivacyClient::availableChanged, this,
            [this](bool) { emit privacyChanged(); });

    // T-15.14b: the Accessibility seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_ACCESSIBILITY_FIXTURE"))
        m_accessibility = new MockAccessibilityClient(this);
    else
        m_accessibility = new DbusAccessibilityClient(this);
    connect(m_accessibility, &AccessibilityClient::changed, this,
            [this](const QVariantMap &) { emit accessibilityChanged(); });
    connect(m_accessibility, &AccessibilityClient::availableChanged, this,
            [this](bool) { emit accessibilityChanged(); });

    // T-15.15b: the Network advanced (VPN) seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_VPN_FIXTURE"))
        m_vpn = new MockVpnClient(this);
    else
        m_vpn = new DbusVpnClient(this);
    connect(m_vpn, &VpnClient::changed, this,
            [this](const QVariantMap &) { emit vpnChanged(); });
    connect(m_vpn, &VpnClient::availableChanged, this,
            [this](bool) { emit vpnChanged(); });

    buildWallpaperPresets();
    connectPortalWatcher();
    m_wallpaperFixture = qEnvironmentVariableIsSet("DF_WALLPAPER_FIXTURE");
    if (m_wallpaperFixture)
        // The fixture is deterministic and in-process: seed the ready
        // catalogue so a pane that opens before any test drives a status
        // still has tiles, never a bus round trip.
        setWallpaperFixture(QStringLiteral("ready"));
    else
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

QVariantMap SettingsBridge::bluetooth() const
{
    return m_bluetooth ? m_bluetooth->view() : QVariantMap();
}

bool SettingsBridge::bluetoothAvailable() const
{
    return m_bluetooth && m_bluetooth->available();
}

QVariantMap SettingsBridge::storage() const
{
    return m_storage ? m_storage->view() : QVariantMap();
}

bool SettingsBridge::storageAvailable() const
{
    return m_storage && m_storage->available();
}

QVariantMap SettingsBridge::sound() const
{
    return m_sound ? m_sound->view() : QVariantMap();
}

bool SettingsBridge::soundAvailable() const
{
    return m_sound && m_sound->available();
}

QVariantMap SettingsBridge::input() const
{
    return m_input ? m_input->view() : QVariantMap();
}

bool SettingsBridge::inputAvailable() const
{
    return m_input && m_input->available();
}

QVariantMap SettingsBridge::battery() const
{
    return m_battery ? m_battery->view() : QVariantMap();
}

bool SettingsBridge::batteryAvailable() const
{
    return m_battery && m_battery->available();
}

QVariantMap SettingsBridge::notifications() const
{
    return m_notifications ? m_notifications->view() : QVariantMap();
}

bool SettingsBridge::notificationsAvailable() const
{
    return m_notifications && m_notifications->available();
}

QVariantMap SettingsBridge::updates() const
{
    return m_updates ? m_updates->view() : QVariantMap();
}

bool SettingsBridge::updatesAvailable() const
{
    return m_updates && m_updates->available();
}

QVariantMap SettingsBridge::accounts() const
{
    return m_accounts ? m_accounts->view() : QVariantMap();
}

bool SettingsBridge::accountsAvailable() const
{
    return m_accounts && m_accounts->available();
}

QVariantMap SettingsBridge::printers() const
{
    return m_printers ? m_printers->view() : QVariantMap();
}

bool SettingsBridge::printersAvailable() const
{
    return m_printers && m_printers->available();
}

QVariantMap SettingsBridge::privacy() const
{
    return m_privacy ? m_privacy->view() : QVariantMap();
}

bool SettingsBridge::privacyAvailable() const
{
    return m_privacy && m_privacy->available();
}

QVariantMap SettingsBridge::accessibility() const
{
    return m_accessibility ? m_accessibility->view() : QVariantMap();
}

bool SettingsBridge::accessibilityAvailable() const
{
    return m_accessibility && m_accessibility->available();
}

QVariantMap SettingsBridge::vpn() const
{
    return m_vpn ? m_vpn->view() : QVariantMap();
}

bool SettingsBridge::vpnAvailable() const
{
    return m_vpn && m_vpn->available();
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

int SettingsBridge::providerPreloadCount() const
{
    return m_providerPreloads;
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
        // The pane renders the attribution as plain text, but never trust the
        // provider's HTML: strip tags from the free-text fields (T-18.2).
        for (const char *key : { "artist", "description" }) {
            const QString name = QString::fromLatin1(key);
            if (entry.contains(name))
                entry.insert(name, sanitizeHtmlText(entry.value(name).toString()));
        }
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
    if (m_wallpaperFixture) {
        // The fixture provider is in-process: record the eager request so the
        // pane-open contract is observable, and leave the catalogue in place.
        ++m_providerPreloads;
        emit providerChanged();
        return;
    }
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

void SettingsBridge::refreshBluetooth()
{
    if (m_bluetooth)
        m_bluetooth->refresh();
}

void SettingsBridge::setBluetoothPowered(bool powered)
{
    if (m_bluetooth)
        m_bluetooth->setPowered(powered);
}

void SettingsBridge::setBluetoothDiscovering(bool discovering)
{
    if (m_bluetooth)
        m_bluetooth->setDiscovering(discovering);
}

void SettingsBridge::pairBluetooth(const QString &address)
{
    if (m_bluetooth)
        m_bluetooth->pair(address);
}

void SettingsBridge::setBluetoothConnected(const QString &address, bool connected)
{
    if (m_bluetooth)
        m_bluetooth->setConnected(address, connected);
}

void SettingsBridge::refreshStorage()
{
    if (m_storage)
        m_storage->refresh();
}

void SettingsBridge::mountStorage(const QString &volumePath)
{
    if (m_storage && !volumePath.isEmpty())
        m_storage->mount(volumePath);
}

void SettingsBridge::unmountStorage(const QString &volumePath)
{
    if (m_storage && !volumePath.isEmpty())
        m_storage->unmount(volumePath);
}

void SettingsBridge::ejectStorage(const QString &drivePath)
{
    if (m_storage && !drivePath.isEmpty())
        m_storage->eject(drivePath);
}

void SettingsBridge::refreshSound()
{
    if (m_sound)
        m_sound->refresh();
}

void SettingsBridge::setSoundVolume(double volume)
{
    if (m_sound)
        m_sound->setVolume(volume);
}

void SettingsBridge::setSoundMute(bool muted)
{
    if (m_sound)
        m_sound->setMute(muted);
}

void SettingsBridge::setSoundDefaultSink(int id)
{
    if (m_sound)
        m_sound->setDefaultSink(id);
}

void SettingsBridge::setSoundDefaultSource(int id)
{
    if (m_sound)
        m_sound->setDefaultSource(id);
}

void SettingsBridge::refreshInput()
{
    if (m_input)
        m_input->refresh();
}

void SettingsBridge::refreshBattery()
{
    if (m_battery)
        m_battery->refresh();
}

void SettingsBridge::setPowerProfile(const QString &profile)
{
    if (m_battery && !profile.isEmpty())
        m_battery->setActiveProfile(profile);
}

void SettingsBridge::refreshNotifications()
{
    if (m_notifications)
        m_notifications->refresh();
}

void SettingsBridge::setFocusMode(const QString &mode)
{
    if (m_notifications && !mode.isEmpty())
        m_notifications->setFocusMode(mode);
}

void SettingsBridge::setFocusApp(const QString &app, bool allowed)
{
    if (!m_notifications || app.trimmed().isEmpty())
        return;
    // The adapter's allow list is a whole-list replace, so read the current
    // list, change exactly one entry, and write it back.
    QStringList list = m_notifications->view()
                           .value(QStringLiteral("allowList"))
                           .toStringList();
    const int index = list.indexOf(QRegularExpression(
        QStringLiteral("^%1$").arg(QRegularExpression::escape(app.trimmed())),
        QRegularExpression::CaseInsensitiveOption));
    if (allowed && index < 0)
        list.append(app.trimmed());
    else if (!allowed && index >= 0)
        list.removeAt(index);
    else
        return;
    m_notifications->setFocusAllowList(list);
}

void SettingsBridge::refreshUpdates()
{
    if (m_updates)
        m_updates->refresh();
}

void SettingsBridge::checkUpdates()
{
    if (m_updates)
        m_updates->check();
}

void SettingsBridge::installUpdates()
{
    if (m_updates)
        m_updates->install();
}

void SettingsBridge::rebootUpdates()
{
    if (m_updates)
        m_updates->reboot();
}

void SettingsBridge::resetUpdatesFixture()
{
    if (m_updates)
        m_updates->resetForTest();
}

void SettingsBridge::refreshAccounts()
{
    if (m_accounts)
        m_accounts->refresh();
}

void SettingsBridge::createAccount(const QString &userName, const QString &realName,
                                   const QString &accountType)
{
    if (m_accounts && !userName.trimmed().isEmpty())
        m_accounts->createUser(userName, realName, accountType);
}

void SettingsBridge::deleteAccount(int uid)
{
    if (m_accounts)
        m_accounts->deleteUser(uid);
}

void SettingsBridge::setAccountType(int uid, const QString &accountType)
{
    if (m_accounts && !accountType.isEmpty())
        m_accounts->setAccountType(uid, accountType);
}

void SettingsBridge::setAccountLocked(int uid, bool locked)
{
    if (m_accounts)
        m_accounts->setLocked(uid, locked);
}

void SettingsBridge::setAccountAutomaticLogin(int uid, bool automaticLogin)
{
    if (m_accounts)
        m_accounts->setAutomaticLogin(uid, automaticLogin);
}

void SettingsBridge::createAccountGroup(const QString &name)
{
    if (m_accounts && !name.trimmed().isEmpty())
        m_accounts->createGroup(name);
}

void SettingsBridge::deleteAccountGroup(const QString &name)
{
    if (m_accounts && !name.isEmpty())
        m_accounts->deleteGroup(name);
}

void SettingsBridge::setAccountGroupMembers(const QString &name, const QStringList &members)
{
    if (m_accounts && !name.isEmpty())
        m_accounts->setGroupMembers(name, members);
}

void SettingsBridge::resetAccountsFixture()
{
    if (m_accounts)
        m_accounts->resetForTest();
}

void SettingsBridge::refreshPrinters()
{
    if (m_printers)
        m_printers->refresh();
}

void SettingsBridge::setDefaultPrinter(const QString &name)
{
    if (m_printers && !name.isEmpty())
        m_printers->setDefaultPrinter(name);
}

void SettingsBridge::setPrinterAcceptingJobs(const QString &name, bool accepting)
{
    if (m_printers && !name.isEmpty())
        m_printers->setPrinterAcceptingJobs(name, accepting);
}

void SettingsBridge::cancelPrinterJob(int jobId)
{
    if (m_printers)
        m_printers->cancelJob(jobId);
}

void SettingsBridge::resetPrintersFixture()
{
    if (m_printers)
        m_printers->resetForTest();
}

void SettingsBridge::refreshPrivacy()
{
    if (m_privacy)
        m_privacy->refresh();
}

void SettingsBridge::setPrivacyPermission(const QString &table, const QString &resourceId,
                                          const QString &app, const QString &permission)
{
    if (m_privacy && !table.isEmpty() && !resourceId.isEmpty() && !app.isEmpty())
        m_privacy->setPermission(table, resourceId, app, permission);
}

void SettingsBridge::deletePrivacyPermission(const QString &table, const QString &resourceId,
                                             const QString &app)
{
    if (m_privacy && !table.isEmpty() && !resourceId.isEmpty() && !app.isEmpty())
        m_privacy->deletePermission(table, resourceId, app);
}

void SettingsBridge::resetPrivacyFixture()
{
    if (m_privacy)
        m_privacy->resetForTest();
}

void SettingsBridge::refreshAccessibility()
{
    if (m_accessibility)
        m_accessibility->refresh();
}

void SettingsBridge::resetAccessibilityFixture()
{
    if (m_accessibility)
        m_accessibility->resetForTest();
}

void SettingsBridge::refreshVpn()
{
    if (m_vpn)
        m_vpn->refresh();
}

void SettingsBridge::connectVpn(const QString &uuid)
{
    if (m_vpn && !uuid.isEmpty())
        m_vpn->connectVpn(uuid);
}

void SettingsBridge::disconnectVpn(const QString &uuid)
{
    if (m_vpn && !uuid.isEmpty())
        m_vpn->disconnectVpn(uuid);
}

void SettingsBridge::resetVpnFixture()
{
    if (m_vpn)
        m_vpn->resetForTest();
}

void SettingsBridge::setWallpaperFixture(const QString &status)
{
    if (!m_wallpaperFixture)
        return;
    const bool ready = status == QStringLiteral("ready");
    applyProviderStatus(status);
    applyProviderItems(ready ? QString::fromLatin1(kWallpaperFixtureItems)
                             : QStringLiteral("[]"));
    const QString firstPath =
        ready ? QStringLiteral("/tmp/dragonfruit-fixture-nature.jpg") : QString();
    applyProviderDefault(firstPath);
    // Leave `BuiltinDefaultSource` empty so the shared shipped-asset resolver
    // supplies the Built-in "Default" tile exactly as production does.
    applyProviderBuiltin(QString());
}

QString SettingsBridge::sanitizeHtmlText(const QString &value)
{
    QString out;
    out.reserve(value.size());
    bool inTag = false;
    for (const QChar ch : value) {
        if (ch == u'<') {
            inTag = true;
            continue;
        }
        if (ch == u'>') {
            inTag = false;
            continue;
        }
        if (!inTag)
            out.append(ch);
    }
    out.replace(QStringLiteral("&amp;"), QStringLiteral("&"));
    out.replace(QStringLiteral("&lt;"), QStringLiteral("<"));
    out.replace(QStringLiteral("&gt;"), QStringLiteral(">"));
    out.replace(QStringLiteral("&quot;"), QStringLiteral("\""));
    out.replace(QStringLiteral("&#39;"), QStringLiteral("'"));
    out.replace(QStringLiteral("&nbsp;"), QStringLiteral(" "));
    return out.simplified();
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