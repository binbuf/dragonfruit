// SPDX-License-Identifier: MIT
#include "SettingsBridge.h"

#include "SettingsAuthorizer.h"
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
#include "GeneralClient.h"
#include "NetworkClient.h"
#include "VpnClient.h"
#include "WifiClient.h"
#include "displaycapabilities.h"
#include "settingsclient.h"

#include <QColor>
#include <QCoreApplication>
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
#include <QGuiApplication>
#include <QIcon>
#include <QScreen>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLinearGradient>
#include <QPainter>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QTimer>
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

// One deterministic provider fixture entry. The first Nature entry's `artist`
// deliberately carries HTML so the pane's sanitization is exercised.
QJsonObject wallpaperFixtureItem(int pageid, const QString &title, const QString &category,
                                 const QString &provider, const QString &artist,
                                 const QString &licenseShortName, const QString &licenseUrl,
                                 const QString &pageUrl, const QString &description,
                                 const QString &thumbPath, const QString &localPath,
                                 const QString &sourceUrl, const QString &fullUrl,
                                 int width, int height)
{
    return QJsonObject{
        { QStringLiteral("pageid"), pageid },
        { QStringLiteral("provider"), provider },
        { QStringLiteral("title"), title },
        { QStringLiteral("artist"), artist },
        { QStringLiteral("licenseShortName"), licenseShortName },
        { QStringLiteral("licenseUrl"), licenseUrl },
        { QStringLiteral("pageUrl"), pageUrl },
        { QStringLiteral("description"), description },
        { QStringLiteral("category"), category },
        { QStringLiteral("width"), width },
        { QStringLiteral("height"), height },
        { QStringLiteral("mime"), QStringLiteral("image/jpeg") },
        { QStringLiteral("sourceUrl"), sourceUrl },
        { QStringLiteral("fullUrl"), fullUrl },
        { QStringLiteral("thumbPath"), thumbPath },
        { QStringLiteral("localPath"), localPath },
        { QStringLiteral("fetchedAt"), 1 },
    };
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

    // T268: the one reactive surface is a per-key property map. A binding on
    // `Settings.values["dock.size"]` is notified only when that key changes;
    // the old `valuesChanged` full-map notification re-evaluated every pane
    // binding on every settingsd event.
    m_valuesMap = QQmlPropertyMap::create(this);
    m_writeErrorsMap = QQmlPropertyMap::create(this);
    m_writeStatesMap = QQmlPropertyMap::create(this);
    const QVariantMap initial = m_client->values();
    for (auto it = initial.constBegin(); it != initial.constEnd(); ++it)
        m_valuesMap->insert(it.key(), it.value());

    connect(m_client, &SettingsClient::changed, this,
            [this](const QString &key, const QVariant &value) {
                publishValue(key, value);
                // T274: the icon/widget-style rows change the toolkit's icon
                // theme provider. This is the one consumer of the two keys.
                if (key == QLatin1String("appearance.iconStyle")
                        || key == QLatin1String("appearance.iconTheme"))
                    applyIconTheme();
                emit valuesChanged();
                emit changed(key, value);
            });
    connect(m_client, &SettingsClient::availableChanged, this,
            [this](bool available) { emit availableChanged(available); });
    connect(m_client, &SettingsClient::writeStateChanged, this,
            [this](const QString &key) {
                refreshWriteState(key);
                emit writeStatesChanged();
            });
    connect(m_client, &SettingsClient::writeFinished, this,
            [this](const QString &key, const QVariant &value, bool ok,
                   const QString &message) {
                // A rejected write never emitted `changed`, so re-publish the
                // stored value to snap the control back; a reverted write did
                // emit one. Both paths must leave the row honest.
                if (!ok)
                    publishValue(key, m_client->value(key));
                refreshWriteState(key);
                emit writeStatesChanged();
                emit writeFinished(key, value, ok, message);
            });

    // T224: privileged actions go through polkit, never an app password field.
    m_authorizer = new SettingsAuthorizer(this);
    if (qEnvironmentVariableIsSet("DF_ACCOUNTS_FIXTURE")
            && !qEnvironmentVariableIsSet("DF_SETTINGS_AUTH_FIXTURE")) {
        // Existing headless pane tests use the deterministic account fixture;
        // it has no authority to ask, so the fixture is the authorizer.
        m_authorizer->setFixture(QStringLiteral("allow"));
    }
    connect(m_authorizer, &SettingsAuthorizer::finished, this,
            [this](const QString &requestId, bool authorized, const QString &error) {
                const auto action = m_pendingAuthActions.take(requestId);
                if (!authorized) {
                    setAccountsError(error.isEmpty()
                                         ? tr("Authentication was denied.")
                                         : error);
                } else if (action) {
                    setAccountsError(QString());
                    action();
                }
                emit authorizationFinished(requestId, authorized, error);
            });

    // T273: the Displays pane lists the real platform outputs. The fixture
    // mode is only read when `DF_DISPLAY_FIXTURE` is set, so the live app
    // always asks the platform.
    m_displayFixtureMode = QStringLiteral("single");
    if (qEnvironmentVariableIsSet("DF_DISPLAY_FIXTURE")) {
        const QString requested = qEnvironmentVariable("DF_DISPLAY_FIXTURE");
        if (requested == QLatin1String("dual")
                || requested == QLatin1String("absent"))
            m_displayFixtureMode = requested;
    }
    // T274: the icon/widget-style provider fixture. An unset environment keeps
    // the live provider (the host's installed icon themes).
    m_iconThemeFixtureMode = qEnvironmentVariableIsSet("DF_ICON_THEME_FIXTURE")
        ? (qEnvironmentVariable("DF_ICON_THEME_FIXTURE") == QLatin1String("none")
               ? QStringLiteral("none") : QStringLiteral("present"))
        : QString();
    m_cursorThemeFixtureMode = qEnvironmentVariableIsSet("DF_CURSOR_THEME_FIXTURE")
        ? (qEnvironmentVariable("DF_CURSOR_THEME_FIXTURE") == QLatin1String("none")
               ? QStringLiteral("none") : QStringLiteral("present"))
        : QString();
    applyIconTheme();

    if (qGuiApp) {
        connect(qGuiApp, &QGuiApplication::screenAdded, this,
                [this](QScreen *screen) {
                    watchDisplayScreen(screen);
                    ++m_displayOutputsRevision;
                    emit displayOutputsChanged();
                });
        connect(qGuiApp, &QGuiApplication::screenRemoved, this,
                [this](QScreen *) { ++m_displayOutputsRevision;
                    emit displayOutputsChanged(); });
        connect(qGuiApp, &QGuiApplication::primaryScreenChanged, this,
                [this](QScreen *) { ++m_displayOutputsRevision;
                    emit displayOutputsChanged(); });
        for (QScreen *screen : QGuiApplication::screens())
            watchDisplayScreen(screen);
    }

    // T268: no adapter client is constructed here. Each pane's `refreshX()`
    // creates exactly its own client on first open (see `ensure*`), so a cold
    // start no longer makes ~20 D-Bus calls before the first pane paints.

    buildWallpaperPresets();
    connectPortalWatcher();
    m_wallpaperFixture = qEnvironmentVariableIsSet("DF_WALLPAPER_FIXTURE");
    if (m_wallpaperFixture) {
        // The fixture is deterministic and in-process: generate real local
        // images (so the pane's preview/apply path is exercised), then seed the
        // ready catalogue so a pane that opens before any test drives a status
        // still has tiles, never a bus round trip.
        buildWallpaperFixtureImages();
        setWallpaperFixture(QStringLiteral("ready"));
    } else {
        connectWallpaperProvider();
    }
}

SettingsBridge::~SettingsBridge()
{
    if (!m_wallpaperFixtureDir.isEmpty())
        QDir(m_wallpaperFixtureDir).removeRecursively();
}

void SettingsBridge::publishValue(const QString &key, const QVariant &value)
{
    if (m_valuesMap->contains(key) && m_valuesMap->value(key) == value) {
        // A rejected write leaves the stored value unchanged, but the control
        // has already moved itself. Drop the key so the binding re-evaluates.
        m_valuesMap->insert(key, QVariant());
    }
    m_valuesMap->insert(key, value);
}

void SettingsBridge::refreshWriteState(const QString &key)
{
    m_writeStatesMap->insert(key, m_client->writeState(key));
    m_writeErrorsMap->insert(key, m_client->writeError(key));
}

BluetoothClient *SettingsBridge::ensureBluetooth()
{
    if (m_bluetooth)
        return m_bluetooth;
    if (qEnvironmentVariableIsSet("DF_BLUETOOTH_FIXTURE"))
        m_bluetooth = new MockBluetoothClient(this);
    else
        m_bluetooth = new DbusBluetoothClient(this);
    connect(m_bluetooth, &BluetoothClient::changed, this,
            [this](const QVariantMap &) { emit bluetoothChanged(); });
    connect(m_bluetooth, &BluetoothClient::availableChanged, this,
            [this](bool) { emit bluetoothChanged(); });
    connect(m_bluetooth, &BluetoothClient::writeFinished, this,
            [this](const QString &operation, const QString &outcome,
                   const QString &message) {
                m_bluetoothWriteOperation = operation;
                m_bluetoothWriteState = outcome.isEmpty()
                    ? QStringLiteral("applied") : outcome;
                m_bluetoothWriteError = message;
                emit bluetoothWriteChanged();
            });
    // The mock built its view before the signal was connected; one refresh
    // publishes it. The live client's constructor read is already in flight.
    if (qobject_cast<MockBluetoothClient *>(m_bluetooth))
        m_bluetooth->refresh();
    noteAdapterCreated();
    return m_bluetooth;
}

StorageClient *SettingsBridge::ensureStorage()
{
    if (m_storage)
        return m_storage;
    if (qEnvironmentVariableIsSet("DF_STORAGE_FIXTURE"))
        m_storage = new MockStorageClient(this);
    else
        m_storage = new DbusStorageClient(this);
    connect(m_storage, &StorageClient::changed, this,
            [this](const QVariantMap &) { emit storageChanged(); });
    connect(m_storage, &StorageClient::availableChanged, this,
            [this](bool) { emit storageChanged(); });
    if (qobject_cast<MockStorageClient *>(m_storage))
        m_storage->refresh();
    noteAdapterCreated();
    return m_storage;
}

SoundClient *SettingsBridge::ensureSound()
{
    if (m_sound)
        return m_sound;
    if (qEnvironmentVariableIsSet("DF_SOUND_FIXTURE"))
        m_sound = new MockSoundClient(this);
    else
        m_sound = new DbusSoundClient(this);
    connect(m_sound, &SoundClient::changed, this,
            [this](const QVariantMap &) { emit soundChanged(); });
    connect(m_sound, &SoundClient::availableChanged, this,
            [this](bool) { emit soundChanged(); });
    if (qobject_cast<MockSoundClient *>(m_sound))
        m_sound->refresh();
    noteAdapterCreated();
    return m_sound;
}

InputClient *SettingsBridge::ensureInput()
{
    if (m_input)
        return m_input;
    if (qEnvironmentVariableIsSet("DF_INPUT_FIXTURE"))
        m_input = new MockInputClient(this);
    else
        m_input = new DbusInputClient(this);
    connect(m_input, &InputClient::changed, this,
            [this](const QVariantMap &) { emit inputChanged(); });
    connect(m_input, &InputClient::availableChanged, this,
            [this](bool) { emit inputChanged(); });
    connect(m_input, &InputClient::controllerTested, this,
            [this](const QVariantMap &report) {
                m_controllerTest = report;
                emit controllerTestChanged();
            });
    if (qobject_cast<MockInputClient *>(m_input))
        m_input->refresh();
    noteAdapterCreated();
    return m_input;
}

BatteryClient *SettingsBridge::ensureBattery()
{
    if (m_battery)
        return m_battery;
    if (qEnvironmentVariableIsSet("DF_BATTERY_FIXTURE"))
        m_battery = new MockBatteryClient(this);
    else
        m_battery = new DbusBatteryClient(this);
    connect(m_battery, &BatteryClient::changed, this,
            [this](const QVariantMap &) { emit batteryChanged(); });
    connect(m_battery, &BatteryClient::availableChanged, this,
            [this](bool) { emit batteryChanged(); });
    if (qobject_cast<MockBatteryClient *>(m_battery))
        m_battery->refresh();
    noteAdapterCreated();
    return m_battery;
}

NotificationsClient *SettingsBridge::ensureNotifications()
{
    if (m_notifications)
        return m_notifications;
    if (qEnvironmentVariableIsSet("DF_NOTIFICATIONS_FIXTURE"))
        m_notifications = new MockNotificationsClient(this);
    else
        m_notifications = new DbusNotificationsClient(this);
    connect(m_notifications, &NotificationsClient::changed, this,
            [this](const QVariantMap &) { emit notificationsChanged(); });
    connect(m_notifications, &NotificationsClient::availableChanged, this,
            [this](bool) { emit notificationsChanged(); });
    if (qobject_cast<MockNotificationsClient *>(m_notifications))
        m_notifications->refresh();
    noteAdapterCreated();
    return m_notifications;
}

UpdatesClient *SettingsBridge::ensureUpdates()
{
    if (m_updates)
        return m_updates;
    if (qEnvironmentVariableIsSet("DF_UPDATES_FIXTURE"))
        m_updates = new MockUpdatesClient(this);
    else
        m_updates = new DbusUpdatesClient(this);
    connect(m_updates, &UpdatesClient::changed, this,
            [this](const QVariantMap &) { emit updatesChanged(); });
    connect(m_updates, &UpdatesClient::availableChanged, this,
            [this](bool) { emit updatesChanged(); });
    if (qobject_cast<MockUpdatesClient *>(m_updates))
        m_updates->refresh();
    noteAdapterCreated();
    return m_updates;
}

GeneralClient *SettingsBridge::ensureGeneral()
{
    if (m_general)
        return m_general;
    if (qEnvironmentVariableIsSet("DF_GENERAL_FIXTURE"))
        m_general = new MockGeneralClient(this);
    else
        m_general = new DbusGeneralClient(this);
    connect(m_general, &GeneralClient::changed, this,
            [this](const QVariantMap &) { emit generalChanged(); });
    connect(m_general, &GeneralClient::availableChanged, this,
            [this](bool) { emit generalChanged(); });
    connect(m_general, &GeneralClient::writeFinished, this,
            [this](const QString &operation, const QString &outcome,
                   const QString &message) {
                m_generalWriteOperation = operation;
                m_generalWriteState = outcome.isEmpty()
                    ? QStringLiteral("applied") : outcome;
                m_generalWriteError = message;
                emit generalWriteChanged();
            });
    if (qobject_cast<MockGeneralClient *>(m_general))
        m_general->refresh();
    noteAdapterCreated();
    return m_general;
}

AccountsClient *SettingsBridge::ensureAccounts()
{
    if (m_accounts)
        return m_accounts;
    if (qEnvironmentVariableIsSet("DF_ACCOUNTS_FIXTURE")
        || qEnvironmentVariableIsSet("DF_BIOMETRICS_FIXTURE")
        || qEnvironmentVariableIsSet("DF_ONLINE_ACCOUNTS_FIXTURE"))
        m_accounts = new MockAccountsClient(this);
    else
        m_accounts = new DbusAccountsClient(this);
    connect(m_accounts, &AccountsClient::changed, this,
            [this](const QVariantMap &) { emit accountsChanged(); });
    connect(m_accounts, &AccountsClient::availableChanged, this,
            [this](bool) { emit accountsChanged(); });
    connect(m_accounts, &AccountsClient::biometricsChanged, this,
            [this](const QVariantMap &) { emit biometricsChanged(); });
    connect(m_accounts, &AccountsClient::onlineAccountsChanged, this,
            [this](const QVariantMap &) { emit onlineAccountsChanged(); });
    if (qobject_cast<MockAccountsClient *>(m_accounts))
        m_accounts->refresh();
    noteAdapterCreated();
    return m_accounts;
}

PrintersClient *SettingsBridge::ensurePrinters()
{
    if (m_printers)
        return m_printers;
    if (qEnvironmentVariableIsSet("DF_PRINTERS_FIXTURE"))
        m_printers = new MockPrintersClient(this);
    else
        m_printers = new DbusPrintersClient(this);
    connect(m_printers, &PrintersClient::changed, this,
            [this](const QVariantMap &) { emit printersChanged(); });
    connect(m_printers, &PrintersClient::availableChanged, this,
            [this](bool) { emit printersChanged(); });
    connect(m_printers, &PrintersClient::discoveryChanged, this,
            [this](const QVariantMap &report) {
                m_printerDiscovery = report;
                emit printerDiscoveryChanged();
            });
    connect(m_printers, &PrintersClient::optionsChanged, this,
            [this](const QVariantMap &report) {
                m_printerOptions = report;
                emit printerOptionsChanged();
            });
    if (qobject_cast<MockPrintersClient *>(m_printers))
        m_printers->refresh();
    noteAdapterCreated();
    return m_printers;
}

PrivacyClient *SettingsBridge::ensurePrivacy()
{
    if (m_privacy)
        return m_privacy;
    if (qEnvironmentVariableIsSet("DF_PRIVACY_FIXTURE"))
        m_privacy = new MockPrivacyClient(this);
    else
        m_privacy = new DbusPrivacyClient(this);
    connect(m_privacy, &PrivacyClient::changed, this,
            [this](const QVariantMap &) { emit privacyChanged(); });
    connect(m_privacy, &PrivacyClient::securityChanged, this,
            [this](const QVariantMap &) { emit securityChanged(); });
    connect(m_privacy, &PrivacyClient::availableChanged, this,
            [this](bool) { emit privacyChanged(); });
    if (qobject_cast<MockPrivacyClient *>(m_privacy))
        m_privacy->refresh();
    noteAdapterCreated();
    return m_privacy;
}

AccessibilityClient *SettingsBridge::ensureAccessibility()
{
    if (m_accessibility)
        return m_accessibility;
    if (qEnvironmentVariableIsSet("DF_ACCESSIBILITY_FIXTURE"))
        m_accessibility = new MockAccessibilityClient(this);
    else
        m_accessibility = new DbusAccessibilityClient(this);
    connect(m_accessibility, &AccessibilityClient::changed, this,
            [this](const QVariantMap &) { emit accessibilityChanged(); });
    connect(m_accessibility, &AccessibilityClient::availableChanged, this,
            [this](bool) { emit accessibilityChanged(); });
    if (qobject_cast<MockAccessibilityClient *>(m_accessibility))
        m_accessibility->refresh();
    noteAdapterCreated();
    return m_accessibility;
}

VpnClient *SettingsBridge::ensureVpn()
{
    if (m_vpn)
        return m_vpn;
    if (qEnvironmentVariableIsSet("DF_VPN_FIXTURE"))
        m_vpn = new MockVpnClient(this);
    else
        m_vpn = new DbusVpnClient(this);
    connect(m_vpn, &VpnClient::changed, this,
            [this](const QVariantMap &) { emit vpnChanged(); });
    connect(m_vpn, &VpnClient::availableChanged, this,
            [this](bool) { emit vpnChanged(); });
    connect(m_vpn, &VpnClient::writeFinished, this,
            [this](const QString &operation, const QString &outcome,
                   const QString &message) {
                m_vpnWriteOperation = operation;
                m_vpnWriteState = outcome.isEmpty()
                    ? QStringLiteral("applied") : outcome;
                m_vpnWriteError = message;
                emit vpnWriteChanged();
            });
    if (qobject_cast<MockVpnClient *>(m_vpn))
        m_vpn->refresh();
    noteAdapterCreated();
    return m_vpn;
}

WifiClient *SettingsBridge::ensureWifi()
{
    if (m_wifi)
        return m_wifi;
    if (qEnvironmentVariableIsSet("DF_WIFI_FIXTURE"))
        m_wifi = new MockWifiClient(this);
    else
        m_wifi = new DbusWifiClient(this);
    connect(m_wifi, &WifiClient::changed, this,
            [this](const QVariantMap &) { emit wifiChanged(); });
    connect(m_wifi, &WifiClient::availableChanged, this,
            [this](bool) { emit wifiChanged(); });
    connect(m_wifi, &WifiClient::writeFinished, this,
            [this](const QString &operation, const QString &outcome,
                   const QString &message) {
                m_wifiWriteOperation = operation;
                m_wifiWriteState = outcome.isEmpty()
                    ? QStringLiteral("applied") : outcome;
                m_wifiWriteError = message;
                emit wifiWriteChanged();
            });
    if (qobject_cast<MockWifiClient *>(m_wifi))
        m_wifi->refresh();
    noteAdapterCreated();
    return m_wifi;
}

NetworkClient *SettingsBridge::ensureNetwork()
{
    if (m_network)
        return m_network;
    if (qEnvironmentVariableIsSet("DF_NETWORK_FIXTURE"))
        m_network = new MockNetworkClient(this);
    else
        m_network = new DbusNetworkClient(this);
    connect(m_network, &NetworkClient::changed, this,
            [this](const QVariantMap &) { emit networkChanged(); });
    connect(m_network, &NetworkClient::availableChanged, this,
            [this](bool) { emit networkChanged(); });
    connect(m_network, &NetworkClient::writeFinished, this,
            [this](const QString &operation, const QString &outcome,
                   const QString &message) {
                m_networkWriteOperation = operation;
                m_networkWriteState = outcome.isEmpty()
                    ? QStringLiteral("applied") : outcome;
                m_networkWriteError = message;
                emit networkWriteChanged();
            });
    if (qobject_cast<MockNetworkClient *>(m_network))
        m_network->refresh();
    noteAdapterCreated();
    return m_network;
}

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

QVariantMap SettingsBridge::controllerTest() const
{
    return m_controllerTest;
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

QVariantMap SettingsBridge::general() const
{
    return m_general ? m_general->view() : QVariantMap();
}

bool SettingsBridge::generalAvailable() const
{
    return m_general && m_general->available();
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

QVariantMap SettingsBridge::printerDiscovery() const
{
    return m_printerDiscovery;
}

QVariantMap SettingsBridge::printerOptions() const
{
    return m_printerOptions;
}

QVariantMap SettingsBridge::privacy() const
{
    return m_privacy ? m_privacy->view() : QVariantMap();
}

bool SettingsBridge::privacyAvailable() const
{
    return m_privacy && m_privacy->available();
}

QVariantMap SettingsBridge::security() const
{
    return m_privacy ? m_privacy->securityView() : QVariantMap();
}

bool SettingsBridge::securityAvailable() const
{
    if (!m_privacy)
        return false;
    const QVariantMap view = m_privacy->securityView();
    if (view.isEmpty())
        return false;
    return view.value(QStringLiteral("state")).toString()
        != QStringLiteral("unavailable");
}

QVariantMap SettingsBridge::biometrics() const
{
    return m_accounts ? m_accounts->biometricsView() : QVariantMap();
}

bool SettingsBridge::biometricsAvailable() const
{
    return m_accounts && m_accounts->biometricsAvailable();
}

QVariantMap SettingsBridge::onlineAccounts() const
{
    return m_accounts ? m_accounts->onlineAccountsView() : QVariantMap();
}

bool SettingsBridge::onlineAccountsAvailable() const
{
    return m_accounts && m_accounts->onlineAccountsAvailable();
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

QVariantMap SettingsBridge::wifi() const
{
    return m_wifi ? m_wifi->view() : QVariantMap();
}

bool SettingsBridge::wifiAvailable() const
{
    return m_wifi && m_wifi->available();
}

QVariantMap SettingsBridge::network() const
{
    return m_network ? m_network->view() : QVariantMap();
}

bool SettingsBridge::networkAvailable() const
{
    return m_network && m_network->available();
}

int SettingsBridge::adapterClientCount() const
{
    int count = 0;
    for (const QObject *client : { static_cast<const QObject *>(m_bluetooth),
                                   static_cast<const QObject *>(m_storage),
                                   static_cast<const QObject *>(m_sound),
                                   static_cast<const QObject *>(m_input),
                                   static_cast<const QObject *>(m_battery),
                                   static_cast<const QObject *>(m_notifications),
                                   static_cast<const QObject *>(m_updates),
                                   static_cast<const QObject *>(m_general),
                                   static_cast<const QObject *>(m_accounts),
                                   static_cast<const QObject *>(m_printers),
                                   static_cast<const QObject *>(m_privacy),
                                   static_cast<const QObject *>(m_accessibility),
                                   static_cast<const QObject *>(m_vpn),
                                   static_cast<const QObject *>(m_wifi),
                                   static_cast<const QObject *>(m_network) }) {
        if (client)
            ++count;
    }
    return count;
}

void SettingsBridge::noteAdapterCreated()
{
    emit adapterClientCountChanged();
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

QString SettingsBridge::startDeepLink() const
{
    return qEnvironmentVariable("DF_SETTINGS_START_DEEP_LINK");
}

bool SettingsBridge::windowStatePersistence() const
{
    if (!qEnvironmentVariableIsSet("DF_SETTINGS_FIXTURE"))
        return true;
    return qEnvironmentVariableIsSet("DF_SETTINGS_WINDOW_STATE");
}

QString SettingsBridge::mapDocPath() const
{
    const QString override =
        qEnvironmentVariable("DF_SETTINGS_MAP_DOC").trimmed();
    if (!override.isEmpty())
        return override;

    const QString relative =
        QStringLiteral("docs/design/settings-business-logic-map.md");
    // The test binary and the installed app both live below the build tree;
    // walk up until the document resolves so the Help link works from a
    // headless test and a normal dev launch.
    QDir dir(QCoreApplication::applicationDirPath());
    for (int depth = 0; depth < 10; ++depth) {
        const QString candidate = dir.absoluteFilePath(relative);
        if (QFileInfo::exists(candidate))
            return candidate;
        if (!dir.cdUp())
            break;
    }
    const QString cwdCandidate = QDir::current().absoluteFilePath(relative);
    if (QFileInfo::exists(cwdCandidate))
        return cwdCandidate;
    return relative;
}

QUrl SettingsBridge::mapDocUrl() const
{
    const QString path = mapDocPath();
    if (path.isEmpty() || !QFileInfo::exists(path))
        return {};
    return QUrl::fromLocalFile(path);
}

bool SettingsBridge::debugView() const
{
    return qEnvironmentVariableIsSet("DF_SETTINGS_DEBUG");
}

QString SettingsBridge::glassRollout() const
{
    // The same selection rule as `shell/src/shellcontroller.cpp` and
    // `material_preset::Rollout::from_env`: only an explicit
    // `legacy`/`replacement` overrides the shipped default. An empty result
    // lets the QML shell read `Theme.glass.rollout.defaultMode`, so the token
    // file remains the single source of the default.
    const QString requested =
        qEnvironmentVariable("DRAGONFRUIT_GLASS_ROLLOUT").trimmed().toLower();
    return (requested == QLatin1String("legacy")
            || requested == QLatin1String("replacement"))
        ? requested : QString();
}

bool SettingsBridge::appChromeSupported() const
{
    return !qEnvironmentVariableIsSet("DF_APP_CHROME_SOLID");
}

bool SettingsBridge::trace() const
{
    return qEnvironmentVariableIsSet("DF_SETTINGS_TRACE");
}

QVariant SettingsBridge::value(const QString &key, const QVariant &fallback) const
{
    return m_client->value(key, fallback);
}

QVariantMap SettingsBridge::writeErrors() const
{
    return m_client->writeErrors();
}

QVariantMap SettingsBridge::writeStates() const
{
    return m_client->writeStates();
}

bool SettingsBridge::apply(const QString &key, const QVariant &value)
{
    return m_client->set(key, value);
}

bool SettingsBridge::set(const QString &key, const QVariant &value)
{
    return apply(key, value);
}

bool SettingsBridge::setList(const QString &key, const QStringList &value)
{
    return apply(key, value);
}

QString SettingsBridge::validationError(const QString &key, const QVariant &value) const
{
    return m_client->validate(key, value);
}

QString SettingsBridge::errorFor(const QString &key) const
{
    return m_client->writeError(key);
}

QString SettingsBridge::stateFor(const QString &key) const
{
    return m_client->writeState(key);
}

QVariantMap SettingsBridge::schemaFor(const QString &key) const
{
    // One schema table, shared with the validator: a row can size a slider or
    // an enum popup from the same bounds the owner enforces.
    return settingsSchemaInfo(key);
}

QVariantList SettingsBridge::displayAdvancedRows() const
{
    return displayAdvancedRowsVariant();
}

void SettingsBridge::watchDisplayScreen(QScreen *screen)
{
    if (!screen)
        return;
    connect(screen, &QScreen::geometryChanged, this,
            [this](const QRect &) { ++m_displayOutputsRevision;
                    emit displayOutputsChanged(); });
    connect(screen, &QScreen::refreshRateChanged, this,
            [this](qreal) { ++m_displayOutputsRevision;
                    emit displayOutputsChanged(); });
}

QVariantList SettingsBridge::displayOutputs() const
{
    // T273: the Displays pane's per-output list is the real platform list, not
    // a hardcoded "Built-in Display". On a Wayland session Qt's screen name is
    // the compositor's output name; the model/manufacturer are the fallback.
    // `DF_DISPLAY_FIXTURE` makes the list deterministic for headless tests and
    // captures (one output, two outputs, or no output).
    QVariantList outputs;
    const bool fixture = qEnvironmentVariableIsSet("DF_DISPLAY_FIXTURE");
    if (fixture) {
        if (m_displayFixtureMode == QLatin1String("absent"))
            return outputs;
        QVariantMap builtIn;
        builtIn.insert(QStringLiteral("id"), QStringLiteral("builtin"));
        builtIn.insert(QStringLiteral("name"), QStringLiteral("Built-in Display"));
        builtIn.insert(QStringLiteral("width"), 1512);
        builtIn.insert(QStringLiteral("height"), 982);
        builtIn.insert(QStringLiteral("refreshRate"), 120.0);
        builtIn.insert(QStringLiteral("primary"), true);
        builtIn.insert(QStringLiteral("manufacturer"), QStringLiteral("Dragonfruit"));
        builtIn.insert(QStringLiteral("model"), QStringLiteral("Built-in HiDPI Display"));
        outputs.append(builtIn);
        if (m_displayFixtureMode == QLatin1String("dual")) {
            QVariantMap studio;
            studio.insert(QStringLiteral("id"), QStringLiteral("external"));
            studio.insert(QStringLiteral("name"), QStringLiteral("External Display"));
            studio.insert(QStringLiteral("width"), 2560);
            studio.insert(QStringLiteral("height"), 1440);
            studio.insert(QStringLiteral("refreshRate"), 60.0);
            studio.insert(QStringLiteral("primary"), false);
            studio.insert(QStringLiteral("manufacturer"), QStringLiteral("Example"));
            studio.insert(QStringLiteral("model"), QStringLiteral("Example Panel"));
            outputs.append(studio);
        }
        return outputs;
    }

    QScreen *primary = QGuiApplication::primaryScreen();
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (int index = 0; index < screens.size(); ++index) {
        QScreen *screen = screens[index];
        if (!screen)
            continue;
        const QRect geometry = screen->geometry();
        QString name = screen->name().trimmed();
        if (name.isEmpty())
            name = screen->model().trimmed();
        if (name.isEmpty())
            name = QObject::tr("Display %1").arg(index + 1);
        QVariantMap entry;
        entry.insert(QStringLiteral("id"),
                     screen->name().isEmpty()
                         ? QStringLiteral("output-%1").arg(index) : screen->name());
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("width"), geometry.width());
        entry.insert(QStringLiteral("height"), geometry.height());
        entry.insert(QStringLiteral("refreshRate"), screen->refreshRate());
        entry.insert(QStringLiteral("primary"), screen == primary);
        entry.insert(QStringLiteral("manufacturer"), screen->manufacturer());
        entry.insert(QStringLiteral("model"), screen->model());
        outputs.append(entry);
    }
    return outputs;
}

void SettingsBridge::setDisplayFixture(const QString &mode)
{
    if (!qEnvironmentVariableIsSet("DF_DISPLAY_FIXTURE"))
        return;
    m_displayFixtureMode = mode == QLatin1String("dual")
            || mode == QLatin1String("absent")
        ? mode : QStringLiteral("single");
    ++m_displayOutputsRevision;
                    emit displayOutputsChanged();
}

QStringList SettingsBridge::installedIconThemes() const
{
    // T274: a real icon-theme provider. A theme is an installed directory with
    // an `index.theme`; this is the same inventory the freedesktop icon lookup
    // uses, not a hardcoded list.
    if (m_iconThemeFixtureMode == QLatin1String("none"))
        return {};
    if (m_iconThemeFixtureMode == QLatin1String("present")) {
        return { QStringLiteral("Plain"), QStringLiteral("Plain Dark"),
                 QStringLiteral("High Contrast") };
    }
    QStringList themes;
    for (const QString &root : QIcon::themeSearchPaths()) {
        const QDir dir(root);
        const QFileInfoList entries =
            dir.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
        for (const QFileInfo &entry : entries) {
            if (QFileInfo::exists(entry.filePath()
                                  + QStringLiteral("/index.theme"))
                    && !themes.contains(entry.fileName())) {
                themes << entry.fileName();
            }
        }
    }
    themes.sort(Qt::CaseInsensitive);
    return themes;
}

QStringList SettingsBridge::installedCursorThemes() const
{
    // T274: cursor themes are directories with a `cursors/` subdirectory. The
    // compositor reads `XCURSOR_THEME`/`XCURSOR_SIZE` at session start, so
    // these rows store a preference with a next-login consumer.
    if (m_cursorThemeFixtureMode == QLatin1String("none"))
        return {};
    if (m_cursorThemeFixtureMode == QLatin1String("present"))
        return { QStringLiteral("Plain Cursors"), QStringLiteral("Large Cursors") };
    QStringList roots = QIcon::themeSearchPaths();
    const QString dataHome =
        QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (!dataHome.isEmpty())
        roots << dataHome + QStringLiteral("/icons");
    roots << QDir::homePath() + QStringLiteral("/.icons");
    QStringList themes;
    for (const QString &root : roots) {
        const QDir dir(root);
        const QFileInfoList entries =
            dir.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
        for (const QFileInfo &entry : entries) {
            if (QFileInfo::exists(entry.filePath() + QStringLiteral("/cursors"))
                    && !themes.contains(entry.fileName())) {
                themes << entry.fileName();
            }
        }
    }
    themes.sort(Qt::CaseInsensitive);
    return themes;
}

QString SettingsBridge::iconThemeForStyle(const QString &style,
                                          const QStringList &themes) const
{
    if (themes.isEmpty())
        return QString();
    if (style == QLatin1String("default"))
        return themes.first();
    const QStringList needles = style == QLatin1String("dark")
        ? QStringList{ QStringLiteral("dark") }
        : style == QLatin1String("clear")
            ? QStringList{ QStringLiteral("clear"), QStringLiteral("glass"),
                           QStringLiteral("transparent") }
            : style == QLatin1String("tinted")
                ? QStringList{ QStringLiteral("tint"), QStringLiteral("color") }
                : QStringList();
    for (const QString &theme : themes) {
        for (const QString &needle : needles) {
            if (theme.contains(needle, Qt::CaseInsensitive))
                return theme;
        }
    }
    return QString();
}

QVariantList SettingsBridge::iconThemes() const
{
    QVariantList out;
    QVariantMap automatic;
    automatic.insert(QStringLiteral("value"), QString());
    automatic.insert(QStringLiteral("label"), tr("Default"));
    out << automatic;
    const QStringList themes = installedIconThemes();
    for (const QString &theme : themes) {
        QVariantMap entry;
        entry.insert(QStringLiteral("value"), theme);
        entry.insert(QStringLiteral("label"), theme);
        out << entry;
    }
    return out;
}

QVariantList SettingsBridge::iconStyleOptions() const
{
    const QStringList themes = installedIconThemes();
    struct Style { const char *value; const char *label; const char *reason; };
    const Style styles[] = {
        { "default", QT_TR_NOOP("Default"), QT_TR_NOOP("No icon theme is installed.") },
        { "dark", QT_TR_NOOP("Dark"), QT_TR_NOOP("No Dark icon theme is installed.") },
        { "clear", QT_TR_NOOP("Clear"), QT_TR_NOOP("No Clear icon theme is installed.") },
        { "tinted", QT_TR_NOOP("Tinted"), QT_TR_NOOP("No Tinted icon theme is installed.") },
    };
    QVariantList out;
    for (const Style &style : styles) {
        const QString value = QString::fromLatin1(style.value);
        const QString theme = iconThemeForStyle(value, themes);
        const bool enabled = !theme.isEmpty();
        QVariantMap entry;
        entry.insert(QStringLiteral("value"), value);
        entry.insert(QStringLiteral("label"), tr(style.label));
        entry.insert(QStringLiteral("enabled"), enabled);
        entry.insert(QStringLiteral("theme"), theme);
        entry.insert(QStringLiteral("reason"),
                     enabled ? QString() : tr(style.reason));
        out << entry;
    }
    return out;
}

QVariantList SettingsBridge::cursorThemes() const
{
    QVariantList out;
    QVariantMap automatic;
    automatic.insert(QStringLiteral("value"), QString());
    automatic.insert(QStringLiteral("label"), tr("Default"));
    out << automatic;
    const QStringList themes = installedCursorThemes();
    for (const QString &theme : themes) {
        QVariantMap entry;
        entry.insert(QStringLiteral("value"), theme);
        entry.insert(QStringLiteral("label"), theme);
        out << entry;
    }
    return out;
}

QString SettingsBridge::effectiveIconTheme() const
{
    const QString explicitTheme =
        m_client->value(QStringLiteral("appearance.iconTheme")).toString();
    if (!explicitTheme.isEmpty()
            && installedIconThemes().contains(explicitTheme)) {
        return explicitTheme;
    }
    const QString style =
        m_client->value(QStringLiteral("appearance.iconStyle")).toString();
    return iconThemeForStyle(style.isEmpty() ? QStringLiteral("default") : style,
                             installedIconThemes());
}

void SettingsBridge::applyIconTheme()
{
    const QString theme = effectiveIconTheme();
    if (!theme.isEmpty() && QIcon::themeName() != theme)
        QIcon::setThemeName(theme);
}

void SettingsBridge::setIconThemeFixture(const QString &mode)
{
    if (!qEnvironmentVariableIsSet("DF_ICON_THEME_FIXTURE"))
        return;
    m_iconThemeFixtureMode = mode == QLatin1String("none")
        ? QStringLiteral("none") : QStringLiteral("present");
    ++m_iconThemesRevision;
    applyIconTheme();
    emit iconThemesChanged();
}

void SettingsBridge::setCursorThemeFixture(const QString &mode)
{
    if (!qEnvironmentVariableIsSet("DF_CURSOR_THEME_FIXTURE"))
        return;
    m_cursorThemeFixtureMode = mode == QLatin1String("none")
        ? QStringLiteral("none") : QStringLiteral("present");
    ++m_iconThemesRevision;
    emit iconThemesChanged();
}

void SettingsBridge::failNextWrite(const QString &key, const QString &message)
{
    if (auto *mock = qobject_cast<MockSettingsClient *>(m_client))
        mock->failNextWrite(key, message);
}

void SettingsBridge::resetSettings()
{
    if (m_client)
        m_client->reset();
}

void SettingsBridge::setAvailableFixture(bool available)
{
    if (auto *mock = qobject_cast<MockSettingsClient *>(m_client))
        mock->setAvailableFixture(available);
}

void SettingsBridge::setAuthFixture(const QString &mode)
{
    if (m_authorizer)
        m_authorizer->setFixture(mode);
}

QString SettingsBridge::requestAuthorization(const QString &actionId,
                                             const QString &message)
{
    const QString requestId = QStringLiteral("auth-%1").arg(++m_authRequestCounter);
    if (m_authorizer)
        m_authorizer->check(requestId, actionId, message);
    return requestId;
}

void SettingsBridge::authorizeAccountAction(const QString &actionId,
                                            const QString &message,
                                            const std::function<void()> &action)
{
    const QString requestId = QStringLiteral("auth-%1").arg(++m_authRequestCounter);
    m_pendingAuthActions.insert(requestId, action);
    if (m_authorizer)
        m_authorizer->check(requestId, actionId, message);
}

void SettingsBridge::setAccountsError(const QString &message)
{
    if (m_accountsError == message)
        return;
    m_accountsError = message;
    emit accountsErrorChanged();
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

QUrl SettingsBridge::helpUrl(const QString &topic) const
{
    QUrl url = mapDocUrl();
    if (url.isEmpty())
        return {};
    if (!topic.trimmed().isEmpty())
        url.setFragment(topic.trimmed());
    return url;
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
    // T268: one asynchronous `Properties.GetAll` instead of four blocking
    // `Get` calls. The constructor no longer waits on a hung provider, and a
    // burst of provider signals coalesces into one follow-up read.
    if (m_providerRefreshPending) {
        m_providerRefreshQueued = true;
        return;
    }
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_providerRefreshPending = true;
    QDBusMessage call = QDBusMessage::createMethodCall(
        kWallpaperService, kWallpaperPath,
        QStringLiteral("org.freedesktop.DBus.Properties"),
        QStringLiteral("GetAll"));
    call << kWallpaperInterface;
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *pending) {
                pending->deleteLater();
                m_providerRefreshPending = false;
                const QDBusPendingReply<QVariantMap> reply = *pending;
                if (reply.isValid()) {
                    const QVariantMap properties = reply.value();
                    const auto text = [](const QVariant &raw) {
                        QVariant value = raw;
                        if (value.userType() == qMetaTypeId<QDBusVariant>())
                            value = value.value<QDBusVariant>().variant();
                        return value.toString();
                    };
                    if (properties.contains(QStringLiteral("Items")))
                        applyProviderItems(
                            text(properties.value(QStringLiteral("Items"))));
                    if (properties.contains(QStringLiteral("Status")))
                        applyProviderStatus(
                            text(properties.value(QStringLiteral("Status"))));
                    if (properties.contains(QStringLiteral("DefaultSource")))
                        applyProviderDefault(
                            text(properties.value(QStringLiteral("DefaultSource"))));
                    if (properties.contains(QStringLiteral("BuiltinDefaultSource")))
                        applyProviderBuiltin(text(properties.value(
                            QStringLiteral("BuiltinDefaultSource"))));
                }
                // An absent provider is a normal state: the reply is an error
                // and every property keeps its previous value.
                if (m_providerRefreshQueued) {
                    m_providerRefreshQueued = false;
                    refreshWallpaperProvider();
                }
            });
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
        for (const char *key : { "title", "artist", "description" }) {
            const QString name = QString::fromLatin1(key);
            if (entry.contains(name))
                entry.insert(name, sanitizeHtmlText(entry.value(name).toString()));
        }
        const QString localPath = entry.value(QStringLiteral("localPath")).toString();
        QString thumbPath = entry.value(QStringLiteral("thumbPath")).toString();
        if (thumbPath.isEmpty())
            thumbPath = localPath;
        const QString thumbUrl = thumbPath.isEmpty()
                ? QString()
                : QUrl::fromLocalFile(thumbPath).toString();
        const QString previewUrl = localPath.isEmpty()
                ? thumbUrl
                : QUrl::fromLocalFile(localPath).toString();
        // `source` stays the full-resolution path the compositor consumes;
        // `url` is the grid thumbnail. `previewUrl` is the best image the hero
        // can show before a deferred full-resolution download.
        entry.insert(QStringLiteral("source"), localPath);
        entry.insert(QStringLiteral("url"), thumbUrl);
        entry.insert(QStringLiteral("thumbUrl"), thumbUrl);
        entry.insert(QStringLiteral("previewUrl"), previewUrl);
        if (!entry.contains(QStringLiteral("provider")))
            entry.insert(QStringLiteral("provider"), QStringLiteral("wikimedia"));
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
    else
        ensureBluetooth();
}

void SettingsBridge::beginBluetoothWrite(const QString &operation)
{
    m_bluetoothWriteOperation = operation;
    m_bluetoothWriteState = QStringLiteral("busy");
    m_bluetoothWriteError.clear();
    emit bluetoothWriteChanged();
}

void SettingsBridge::setBluetoothPowered(bool powered)
{
    beginBluetoothWrite(QStringLiteral("power"));
    if (m_bluetooth)
        m_bluetooth->setPowered(powered);
}

void SettingsBridge::setBluetoothDiscovering(bool discovering)
{
    beginBluetoothWrite(QStringLiteral("discovery"));
    if (m_bluetooth)
        m_bluetooth->setDiscovering(discovering);
}

void SettingsBridge::pairBluetooth(const QString &address)
{
    if (!m_bluetooth || address.isEmpty())
        return;
    beginBluetoothWrite(QStringLiteral("pair"));
    m_bluetooth->pair(address);
}

void SettingsBridge::setBluetoothConnected(const QString &address, bool connected)
{
    if (!m_bluetooth || address.isEmpty())
        return;
    beginBluetoothWrite(QStringLiteral("connect"));
    m_bluetooth->setConnected(address, connected);
}

void SettingsBridge::forgetBluetooth(const QString &address)
{
    if (!m_bluetooth || address.isEmpty())
        return;
    beginBluetoothWrite(QStringLiteral("forget"));
    m_bluetooth->forget(address);
}

void SettingsBridge::resetBluetoothFixture()
{
    if (m_bluetooth)
        m_bluetooth->resetForTest();
    m_bluetoothWriteOperation.clear();
    m_bluetoothWriteState = QStringLiteral("idle");
    m_bluetoothWriteError.clear();
    emit bluetoothWriteChanged();
}

void SettingsBridge::setBluetoothFixture(const QString &mode)
{
    if (m_bluetooth)
        m_bluetooth->setFixture(mode);
}

void SettingsBridge::failNextBluetoothWrite(const QString &operation,
                                            const QString &message)
{
    if (auto *mock = qobject_cast<MockBluetoothClient *>(m_bluetooth))
        mock->failNextWrite(operation, message);
}

void SettingsBridge::refreshStorage()
{
    if (m_storage)
        m_storage->refresh();
    else
        ensureStorage();
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

void SettingsBridge::setStorageFixture(const QString &mode)
{
    if (auto *mock = qobject_cast<MockStorageClient *>(m_storage))
        mock->setFixture(mode);
}

void SettingsBridge::resetStorageFixture()
{
    if (auto *mock = qobject_cast<MockStorageClient *>(m_storage))
        mock->resetForTest();
}

void SettingsBridge::refreshSound()
{
    if (m_sound)
        m_sound->refresh();
    else
        ensureSound();
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

void SettingsBridge::playTestSound()
{
    if (m_sound)
        m_sound->playTestSound();
}

void SettingsBridge::refreshInput()
{
    if (m_input)
        m_input->refresh();
    else
        ensureInput();
}

void SettingsBridge::testController(const QString &path)
{
    if (m_input)
        m_input->testController(path);
}

void SettingsBridge::refreshBattery()
{
    if (m_battery)
        m_battery->refresh();
    else
        ensureBattery();
}

void SettingsBridge::setPowerProfile(const QString &profile)
{
    if (m_battery && !profile.isEmpty())
        m_battery->setActiveProfile(profile);
}

void SettingsBridge::setBatteryFixture(const QString &mode)
{
    if (auto *mock = qobject_cast<MockBatteryClient *>(m_battery))
        mock->setFixture(mode);
}

void SettingsBridge::resetBatteryFixture()
{
    if (auto *mock = qobject_cast<MockBatteryClient *>(m_battery))
        mock->resetForTest();
}

void SettingsBridge::refreshNotifications()
{
    if (m_notifications)
        m_notifications->refresh();
    else
        ensureNotifications();
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

void SettingsBridge::setFocusProfile(const QString &id)
{
    if (m_notifications && !id.trimmed().isEmpty())
        m_notifications->setFocusProfile(id.trimmed());
}

void SettingsBridge::setFocusPerson(const QString &person, bool allowed)
{
    if (!m_notifications || person.trimmed().isEmpty())
        return;
    // Like `setFocusApp`, the adapter takes a whole-list replace.
    QStringList list = m_notifications->view()
                           .value(QStringLiteral("allowedPeople"))
                           .toStringList();
    const int index = list.indexOf(QRegularExpression(
        QStringLiteral("^%1$").arg(QRegularExpression::escape(person.trimmed())),
        QRegularExpression::CaseInsensitiveOption));
    if (allowed && index < 0)
        list.append(person.trimmed());
    else if (!allowed && index >= 0)
        list.removeAt(index);
    else
        return;
    m_notifications->setFocusAllowedPeople(list);
}

void SettingsBridge::setNotificationAppPreference(const QString &app,
                                                  const QString &style,
                                                  bool sound, bool badge,
                                                  bool allowed)
{
    if (m_notifications && !app.trimmed().isEmpty() && !style.isEmpty())
        m_notifications->setAppPreference(app.trimmed(), style, sound, badge,
                                          allowed);
}

void SettingsBridge::refreshUpdates(){
    if (m_updates)
        m_updates->refresh();
    else
        ensureUpdates();
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

void SettingsBridge::setUpdatesAbsentFixture(bool absent)
{
    if (auto *mock = qobject_cast<MockUpdatesClient *>(m_updates))
        mock->setAbsentFixture(absent);
}

void SettingsBridge::setUpdatesProviderFixture(bool available)
{
    if (auto *mock = qobject_cast<MockUpdatesClient *>(m_updates))
        mock->setProviderAvailableFixture(available);
}

void SettingsBridge::refreshGeneral()
{
    if (m_general)
        m_general->refresh();
    else
        ensureGeneral();
}

void SettingsBridge::beginGeneralWrite(const QString &operation)
{
    m_generalWriteOperation = operation;
    m_generalWriteState = QStringLiteral("busy");
    m_generalWriteError.clear();
    emit generalWriteChanged();
}

void SettingsBridge::setNtp(bool enabled)
{
    beginGeneralWrite(QStringLiteral("ntp"));
    if (m_general)
        m_general->setNtp(enabled);
}

void SettingsBridge::setTimezone(const QString &timezone)
{
    beginGeneralWrite(QStringLiteral("timezone"));
    if (m_general)
        m_general->setTimezone(timezone);
}

void SettingsBridge::setTime(const QString &value)
{
    beginGeneralWrite(QStringLiteral("time"));
    if (m_general)
        m_general->setTime(value);
}

void SettingsBridge::setLocale(const QString &locale)
{
    beginGeneralWrite(QStringLiteral("locale"));
    if (m_general)
        m_general->setLocale(locale);
}

void SettingsBridge::setAutostartEnabled(const QString &id, bool enabled)
{
    beginGeneralWrite(QStringLiteral("login"));
    if (m_general)
        m_general->setAutostartEnabled(id, enabled);
}

void SettingsBridge::addAutostart(const QString &appId)
{
    beginGeneralWrite(QStringLiteral("login"));
    if (m_general)
        m_general->addAutostart(appId);
}

void SettingsBridge::removeAutostart(const QString &id)
{
    beginGeneralWrite(QStringLiteral("login"));
    if (m_general)
        m_general->removeAutostart(id);
}

void SettingsBridge::setHostname(const QString &hostname)
{
    beginGeneralWrite(QStringLiteral("hostname"));
    if (m_general)
        m_general->setHostname(hostname);
}

void SettingsBridge::restartSystem()
{
    beginGeneralWrite(QStringLiteral("restart"));
    if (m_general)
        m_general->restartSystem();
}

void SettingsBridge::setUserUnitEnabled(const QString &unit, bool enabled)
{
    beginGeneralWrite(QStringLiteral("login"));
    if (m_general)
        m_general->setUserUnitEnabled(unit, enabled);
}

void SettingsBridge::setSharingEnabled(const QString &id, bool enabled)
{
    beginGeneralWrite(QStringLiteral("sharing"));
    if (m_general)
        m_general->setSharingEnabled(id, enabled);
}

void SettingsBridge::runBackup(const QString &destination)
{
    beginGeneralWrite(QStringLiteral("backup"));
    if (m_general)
        m_general->runBackup(destination);
}

void SettingsBridge::openBackupRestore()
{
    beginGeneralWrite(QStringLiteral("backup"));
    if (m_general)
        m_general->openRestore();
}

void SettingsBridge::openPasswordManager(const QString &id)
{
    if (m_general && !id.trimmed().isEmpty())
        m_general->openPasswordManager(id.trimmed());
}

void SettingsBridge::applyUpdatePolicy(bool autoCheck, bool autoInstall)
{
    if (m_updates)
        m_updates->setAutoPolicy(autoCheck, autoInstall);
}

void SettingsBridge::resetGeneralFixture()
{
    m_generalWriteOperation.clear();
    m_generalWriteState = QStringLiteral("idle");
    m_generalWriteError.clear();
    emit generalWriteChanged();
    if (m_general)
        m_general->resetForTest();
}

void SettingsBridge::failNextGeneralWrite(const QString &operation,
                                          const QString &message)
{
    if (auto *mock = qobject_cast<MockGeneralClient *>(m_general))
        mock->failNextWrite(operation, message);
}

void SettingsBridge::setGeneralAvailableFixture(bool available)
{
    if (auto *mock = qobject_cast<MockGeneralClient *>(m_general))
        mock->setAbsentFixture(!available);
}

void SettingsBridge::refreshAccounts()
{
    setAccountsError(QString());
    if (m_accounts)
        m_accounts->refresh();
    else
        ensureAccounts();
}

// All account/group writes are privileged. They are authorized by the host
// polkit authority (which raises the shell's agent) before the adapter is
// asked; a denial leaves the view untouched and publishes `accountsError`.
// The app never renders a password field of its own.
static const QString kAccountAdminAction =
    QStringLiteral("org.freedesktop.accounts.user-administration");

void SettingsBridge::createAccount(const QString &userName, const QString &realName,
                                   const QString &accountType)
{
    if (!m_accounts || userName.trimmed().isEmpty())
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to create a user account."),
        [this, userName, realName, accountType]() {
            m_accounts->createUser(userName, realName, accountType);
        });
}

void SettingsBridge::deleteAccount(int uid)
{
    if (!m_accounts)
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to delete a user account."),
        [this, uid]() { m_accounts->deleteUser(uid); });
}

void SettingsBridge::setAccountType(int uid, const QString &accountType)
{
    if (!m_accounts || accountType.isEmpty())
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to change a user account type."),
        [this, uid, accountType]() {
            m_accounts->setAccountType(uid, accountType);
        });
}

void SettingsBridge::setAccountLocked(int uid, bool locked)
{
    if (!m_accounts)
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to change a user account."),
        [this, uid, locked]() { m_accounts->setLocked(uid, locked); });
}

void SettingsBridge::setAccountAutomaticLogin(int uid, bool automaticLogin)
{
    if (!m_accounts)
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to change automatic login."),
        [this, uid, automaticLogin]() {
            m_accounts->setAutomaticLogin(uid, automaticLogin);
        });
}

void SettingsBridge::createAccountGroup(const QString &name)
{
    if (!m_accounts || name.trimmed().isEmpty())
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to create a group."),
        [this, name]() { m_accounts->createGroup(name); });
}

void SettingsBridge::deleteAccountGroup(const QString &name)
{
    if (!m_accounts || name.isEmpty())
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to delete a group."),
        [this, name]() { m_accounts->deleteGroup(name); });
}

void SettingsBridge::setAccountGroupMembers(const QString &name, const QStringList &members)
{
    if (!m_accounts || name.isEmpty())
        return;
    authorizeAccountAction(
        kAccountAdminAction,
        tr("Authentication is required to change group membership."),
        [this, name, members]() { m_accounts->setGroupMembers(name, members); });
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
    else
        ensurePrinters();
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

void SettingsBridge::discoverPrinters()
{
    if (m_printers)
        m_printers->discover();
}

void SettingsBridge::addPrinter(const QString &name, const QString &uri,
                                const QString &driver)
{
    if (m_printers && !name.isEmpty() && !uri.isEmpty())
        m_printers->addPrinter(name, uri, driver);
}

void SettingsBridge::loadPrinterOptions(const QString &name)
{
    if (m_printers && !name.isEmpty())
        m_printers->printerOptions(name);
}

void SettingsBridge::setPrinterOption(const QString &name, const QString &key,
                                      const QString &value)
{
    if (m_printers && !name.isEmpty() && !key.isEmpty())
        m_printers->setPrinterOption(name, key, value);
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
    else
        ensurePrivacy();
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

void SettingsBridge::refreshSecurity()
{
    if (m_privacy)
        m_privacy->refreshSecurity();
    else
        ensurePrivacy()->refreshSecurity();
}

void SettingsBridge::setSecuritySourceEnabled(const QString &id, bool enabled)
{
    if (m_privacy && !id.isEmpty())
        m_privacy->setSecuritySourceEnabled(id, enabled);
}

void SettingsBridge::setPrivacyAvailableFixture(bool available)
{
    if (m_privacy)
        m_privacy->setPrivacyAvailableFixture(available);
}

void SettingsBridge::refreshBiometrics()
{
    if (m_accounts)
        m_accounts->refreshBiometrics();
    else
        ensureAccounts()->refreshBiometrics();
}

void SettingsBridge::startFingerprintEnrollment(const QString &finger)
{
    if (m_accounts && !finger.isEmpty())
        m_accounts->startFingerprintEnrollment(finger);
}

void SettingsBridge::cancelFingerprintEnrollment()
{
    if (m_accounts)
        m_accounts->cancelFingerprintEnrollment();
}

void SettingsBridge::deleteFingerprint(const QString &finger)
{
    if (m_accounts && !finger.isEmpty())
        m_accounts->deleteFingerprint(finger);
}

void SettingsBridge::changePassword(const QString &password)
{
    if (m_accounts && !password.isEmpty())
        m_accounts->changePassword(password);
}

void SettingsBridge::resetBiometricsFixture()
{
    if (m_accounts)
        m_accounts->resetForTest();
}

void SettingsBridge::refreshOnlineAccounts()
{
    if (m_accounts)
        m_accounts->refreshOnlineAccounts();
    else
        ensureAccounts()->refreshOnlineAccounts();
}

void SettingsBridge::addOnlineAccount(const QString &provider, const QString &identity)
{
    if (m_accounts && !provider.isEmpty() && !identity.isEmpty())
        m_accounts->addOnlineAccount(provider, identity);
}

void SettingsBridge::removeOnlineAccount(const QString &id)
{
    if (m_accounts && !id.isEmpty())
        m_accounts->removeOnlineAccount(id);
}

void SettingsBridge::setOnlineAccountSync(const QString &id, const QString &capability,
                                          bool enabled)
{
    if (m_accounts && !id.isEmpty() && !capability.isEmpty())
        m_accounts->setOnlineAccountSync(id, capability, enabled);
}

void SettingsBridge::resetOnlineAccountsFixture()
{
    if (m_accounts)
        m_accounts->resetForTest();
}

void SettingsBridge::refreshAccessibility()
{
    if (m_accessibility)
        m_accessibility->refresh();
    else
        ensureAccessibility();
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
    else
        ensureVpn();
}

void SettingsBridge::connectVpn(const QString &uuid)
{
    if (m_vpn && !uuid.isEmpty()) {
        beginVpnWrite(QStringLiteral("connect"));
        m_vpn->connectVpn(uuid);
    }
}

void SettingsBridge::disconnectVpn(const QString &uuid)
{
    if (m_vpn && !uuid.isEmpty()) {
        beginVpnWrite(QStringLiteral("disconnect"));
        m_vpn->disconnectVpn(uuid);
    }
}

void SettingsBridge::addVpn(const QString &plugin, const QString &name,
                            const QString &gateway)
{
    if (m_vpn && !plugin.isEmpty() && !name.isEmpty()) {
        beginVpnWrite(QStringLiteral("add"));
        m_vpn->addVpn(plugin, name, gateway);
    }
}

void SettingsBridge::addVpnWireguard(const QString &name,
                                     const QString &privateKey,
                                     const QString &address,
                                     const QString &peerPublicKey,
                                     const QString &endpoint,
                                     const QString &dns, int mtu)
{
    if (m_vpn && !name.isEmpty() && !privateKey.isEmpty() && !address.isEmpty()
        && !peerPublicKey.isEmpty() && !endpoint.isEmpty()) {
        beginVpnWrite(QStringLiteral("add-wireguard"));
        m_vpn->addWireguard(name, privateKey, address, peerPublicKey, endpoint,
                            dns, mtu);
    }
}

void SettingsBridge::editVpn(const QString &uuid)
{
    if (m_vpn && !uuid.isEmpty()) {
        beginVpnWrite(QStringLiteral("edit"));
        m_vpn->editVpn(uuid);
    }
}

void SettingsBridge::setVpnAutoconnect(const QString &uuid, bool enabled)
{
    if (m_vpn && !uuid.isEmpty()) {
        beginVpnWrite(QStringLiteral("autoconnect"));
        m_vpn->setAutoconnect(uuid, enabled);
    }
}

void SettingsBridge::resetVpnFixture()
{
    if (m_vpn)
        m_vpn->resetForTest();
    m_vpnWriteOperation.clear();
    m_vpnWriteState = QStringLiteral("idle");
    m_vpnWriteError.clear();
    emit vpnWriteChanged();
}

void SettingsBridge::setVpnFixture(const QString &mode)
{
    if (m_vpn)
        m_vpn->setFixture(mode);
}

void SettingsBridge::failNextVpnWrite(const QString &operation,
                                      const QString &message)
{
    if (m_vpn)
        m_vpn->failNextWrite(operation, message);
}

void SettingsBridge::beginVpnWrite(const QString &operation)
{
    m_vpnWriteOperation = operation;
    m_vpnWriteState = QStringLiteral("busy");
    m_vpnWriteError.clear();
    emit vpnWriteChanged();
}

void SettingsBridge::refreshWifi()
{
    if (m_wifi)
        m_wifi->refresh();
    else
        ensureWifi();
}

void SettingsBridge::beginWifiWrite(const QString &operation)
{
    m_wifiWriteOperation = operation;
    m_wifiWriteState = QStringLiteral("busy");
    m_wifiWriteError.clear();
    emit wifiWriteChanged();
}

void SettingsBridge::setWifiEnabled(bool enabled)
{
    beginWifiWrite(QStringLiteral("power"));
    if (m_wifi)
        m_wifi->setEnabled(enabled);
}

void SettingsBridge::joinWifi(const QString &ssid, const QString &secret)
{
    if (!m_wifi || ssid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("join"));
    m_wifi->join(ssid, secret);
}

void SettingsBridge::joinHiddenWifi(const QString &ssid, const QString &secret)
{
    if (!m_wifi || ssid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("joinHidden"));
    m_wifi->joinHidden(ssid, secret);
}

void SettingsBridge::forgetWifi(const QString &uuid)
{
    if (!m_wifi || uuid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("forget"));
    m_wifi->forget(uuid);
}

void SettingsBridge::setWifiAutoconnect(const QString &uuid, bool enabled)
{
    if (!m_wifi || uuid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("autoconnect"));
    m_wifi->setAutoconnect(uuid, enabled);
}

void SettingsBridge::scanWifi()
{
    beginWifiWrite(QStringLiteral("scan"));
    if (m_wifi)
        m_wifi->scan();
}

void SettingsBridge::disconnectWifi(const QString &uuid)
{
    if (!m_wifi || uuid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("disconnect"));
    m_wifi->disconnect(uuid);
}

void SettingsBridge::startWifiHotspot(const QString &ssid, const QString &secret)
{
    if (!m_wifi || ssid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("hotspot"));
    m_wifi->startHotspot(ssid, secret);
}

void SettingsBridge::stopWifiHotspot(const QString &uuid)
{
    if (!m_wifi || uuid.isEmpty())
        return;
    beginWifiWrite(QStringLiteral("hotspot"));
    m_wifi->stopHotspot(uuid);
}

void SettingsBridge::resetWifiFixture()
{
    if (m_wifi)
        m_wifi->resetForTest();
    m_wifiWriteOperation.clear();
    m_wifiWriteState = QStringLiteral("idle");
    m_wifiWriteError.clear();
    emit wifiWriteChanged();
}

void SettingsBridge::setWifiFixture(const QString &mode)
{
    if (m_wifi)
        m_wifi->setFixture(mode);
}

void SettingsBridge::failNextWifiWrite(const QString &operation,
                                       const QString &message)
{
    if (auto *mock = qobject_cast<MockWifiClient *>(m_wifi))
        mock->failNextWrite(operation, message);
}

void SettingsBridge::refreshNetwork()
{
    if (m_network)
        m_network->refresh();
    else
        ensureNetwork();
}

void SettingsBridge::activateNetworkService(const QString &uuid)
{
    if (m_network && !uuid.isEmpty()) {
        beginNetworkWrite(QStringLiteral("activate"));
        m_network->activateService(uuid);
    }
}

void SettingsBridge::deactivateNetworkService(const QString &uuid)
{
    if (m_network && !uuid.isEmpty()) {
        beginNetworkWrite(QStringLiteral("deactivate"));
        m_network->deactivateService(uuid);
    }
}

void SettingsBridge::moveNetworkService(const QString &uuid, int delta)
{
    if (m_network && !uuid.isEmpty() && delta != 0) {
        beginNetworkWrite(QStringLiteral("move"));
        m_network->moveService(uuid, delta);
    }
}

void SettingsBridge::addNetworkService(const QString &name, const QString &kind,
                                       const QString &plugin,
                                       const QString &gateway,
                                       const QString &interfaceName)
{
    if (m_network && !name.isEmpty()) {
        beginNetworkWrite(QStringLiteral("add"));
        m_network->addService(name, kind, plugin, gateway, interfaceName);
    }
}

void SettingsBridge::forgetNetworkService(const QString &uuid)
{
    if (m_network && !uuid.isEmpty()) {
        beginNetworkWrite(QStringLiteral("forget"));
        m_network->forgetService(uuid);
    }
}

void SettingsBridge::setNetworkAutoconnect(const QString &uuid, bool enabled)
{
    if (m_network && !uuid.isEmpty()) {
        beginNetworkWrite(QStringLiteral("autoconnect"));
        m_network->setAutoconnect(uuid, enabled);
    }
}

void SettingsBridge::setFirewallEnabled(bool enabled)
{
    if (m_network) {
        beginNetworkWrite(QStringLiteral("firewall"));
        m_network->setFirewallEnabled(enabled);
    }
}

void SettingsBridge::openFirewallPort(int port, const QString &protocol)
{
    if (m_network) {
        beginNetworkWrite(QStringLiteral("port"));
        m_network->openPort(port, protocol);
    }
}

void SettingsBridge::closeFirewallPort(int port, const QString &protocol)
{
    if (m_network) {
        beginNetworkWrite(QStringLiteral("port"));
        m_network->closePort(port, protocol);
    }
}

void SettingsBridge::openFirewallService(const QString &service)
{
    if (m_network && !service.isEmpty()) {
        beginNetworkWrite(QStringLiteral("service"));
        m_network->openService(service);
    }
}

void SettingsBridge::closeFirewallService(const QString &service)
{
    if (m_network && !service.isEmpty()) {
        beginNetworkWrite(QStringLiteral("service"));
        m_network->closeService(service);
    }
}

void SettingsBridge::resetNetworkFixture()
{
    if (m_network)
        m_network->resetForTest();
    m_networkWriteOperation.clear();
    m_networkWriteState = QStringLiteral("idle");
    m_networkWriteError.clear();
    emit networkWriteChanged();
}

void SettingsBridge::setNetworkFixture(const QString &mode)
{
    if (m_network)
        m_network->setFixture(mode);
}

void SettingsBridge::failNextNetworkWrite(const QString &operation,
                                          const QString &message)
{
    if (auto *mock = qobject_cast<MockNetworkClient *>(m_network))
        mock->failNextWrite(operation, message);
}

void SettingsBridge::beginNetworkWrite(const QString &operation)
{
    m_networkWriteOperation = operation;
    m_networkWriteState = QStringLiteral("busy");
    m_networkWriteError.clear();
    emit networkWriteChanged();
}

void SettingsBridge::buildWallpaperFixtureImages()
{
    m_wallpaperFixtureDir =
        QDir::tempPath()
        + QStringLiteral("/dragonfruit-wallpaper-fixture-%1")
              .arg(QCoreApplication::applicationPid());
    QDir().mkpath(m_wallpaperFixtureDir);
    for (int pageid = 1; pageid <= 12; ++pageid) {
        const QColor top = QColor::fromHsv((pageid * 37) % 360, 150, 190);
        const QColor bottom = QColor::fromHsv((pageid * 37 + 42) % 360, 205, 95);
        const QImage full = renderGradient(top, bottom)
                                .scaled(480, 300, Qt::IgnoreAspectRatio,
                                        Qt::SmoothTransformation);
        full.save(wallpaperFixturePath(pageid, false));
        full.scaled(240, 150, Qt::IgnoreAspectRatio, Qt::SmoothTransformation)
            .save(wallpaperFixturePath(pageid, true));
    }
}

QString SettingsBridge::wallpaperFixturePath(int pageid, bool thumb) const
{
    return m_wallpaperFixtureDir
        + QStringLiteral("/%1%2.png")
              .arg(pageid)
              .arg(thumb ? QStringLiteral("-thumb") : QString());
}

QString SettingsBridge::wallpaperFixtureJson(const QString &mode) const
{
    QJsonArray items;
    const auto add = [this, &items](int pageid, const QString &title,
                                    const QString &category, const QString &provider,
                                    const QString &artist, const QString &license,
                                    const QString &licenseUrl, const QString &pageUrl,
                                    const QString &description, bool deferred) {
        const QString thumb = wallpaperFixturePath(pageid, true);
        const QString local = deferred ? QString() : wallpaperFixturePath(pageid, false);
        items.append(wallpaperFixtureItem(
            pageid, title, category, provider, artist, license, licenseUrl, pageUrl,
            description, thumb, local,
            QStringLiteral("https://example.org/%1-thumb.jpg").arg(pageid),
            QStringLiteral("https://example.org/%1.jpg").arg(pageid), 3840, 2160));
    };

    if (mode == QStringLiteral("ready")) {
        add(1, QStringLiteral("File:Alpine Lake.jpg"), QStringLiteral("nature"),
            QStringLiteral("wikimedia"),
            QStringLiteral("<a href=\"https://example.org/Alice\">Alice Example</a>"),
            QStringLiteral("CC BY-SA 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by-sa/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Alpine_Lake.jpg"),
            QStringLiteral("An alpine lake"), false);
        add(2, QStringLiteral("File:Ocean Wave.jpg"), QStringLiteral("water"),
            QStringLiteral("wikimedia"), QStringLiteral("Bob"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Ocean_Wave.jpg"),
            QStringLiteral("An ocean wave"), false);
    } else if (mode == QStringLiteral("categories")) {
        add(1, QStringLiteral("File:Alpine Lake.jpg"), QStringLiteral("nature"),
            QStringLiteral("wikimedia"), QStringLiteral("Alice Example"),
            QStringLiteral("CC BY-SA 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by-sa/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Alpine_Lake.jpg"),
            QStringLiteral("An alpine lake"), false);
        add(2, QStringLiteral("File:Ocean Wave.jpg"), QStringLiteral("water"),
            QStringLiteral("wikimedia"), QStringLiteral("Bob"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Ocean_Wave.jpg"),
            QStringLiteral("An ocean wave"), false);
        add(3, QStringLiteral("File:Abstract Glass.jpg"), QStringLiteral("abstract"),
            QStringLiteral("wikimedia"), QStringLiteral("Carol"),
            QStringLiteral("CC0"), QStringLiteral("https://creativecommons.org/publicdomain/zero/1.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Abstract_Glass.jpg"),
            QStringLiteral("Abstract glass"), false);
        add(4, QStringLiteral("File:Night City.jpg"), QStringLiteral("cityscapes"),
            QStringLiteral("wikimedia"), QStringLiteral("Dan"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Night_City.jpg"),
            QStringLiteral("A city at night"), false);
        add(5, QStringLiteral("File:Blue Marble.jpg"), QStringLiteral("earth"),
            QStringLiteral("wikimedia"), QStringLiteral("NASA"),
            QStringLiteral("Public domain"), QStringLiteral("https://commons.wikimedia.org/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Blue_Marble.jpg"),
            QStringLiteral("Earth from space"), false);
        add(6, QStringLiteral("Space Nebula"), QStringLiteral("space"),
            QStringLiteral("nasa"), QStringLiteral("NASA/JPL-Caltech"),
            QStringLiteral("Public domain (NASA)"),
            QStringLiteral("https://www.nasa.gov/nasa-brand-center/images-and-media/"),
            QStringLiteral("https://images.nasa.gov/details/PIA14417"),
            QStringLiteral("A nebula seen by NASA"), true);
        add(7, QStringLiteral("File:Mountain Range.jpg"), QStringLiteral("scenery"),
            QStringLiteral("wikimedia"), QStringLiteral("Eve"),
            QStringLiteral("CC BY-SA 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by-sa/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Mountain_Range.jpg"),
            QStringLiteral("A mountain range"), false);
        add(8, QStringLiteral("File:Coral Reef.jpg"), QStringLiteral("underwater"),
            QStringLiteral("wikimedia"), QStringLiteral("Frank"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Coral_Reef.jpg"),
            QStringLiteral("A coral reef"), false);
    } else if (mode == QStringLiteral("many")) {
        for (int i = 1; i <= 6; ++i) {
            add(i, QStringLiteral("File:Alpine Lake %1.jpg").arg(i),
                QStringLiteral("nature"), QStringLiteral("wikimedia"),
                QStringLiteral("Alice Example"), QStringLiteral("CC BY-SA 4.0"),
                QStringLiteral("https://creativecommons.org/licenses/by-sa/4.0/"),
                QStringLiteral("https://commons.wikimedia.org/wiki/File:Alpine_Lake.jpg"),
                QStringLiteral("An alpine lake"), false);
        }
        add(7, QStringLiteral("File:Ocean Wave.jpg"), QStringLiteral("water"),
            QStringLiteral("wikimedia"), QStringLiteral("Bob"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Ocean_Wave.jpg"),
            QStringLiteral("An ocean wave"), false);
    } else if (mode == QStringLiteral("deferred")
               || mode == QStringLiteral("deferred-fail")) {
        add(1, QStringLiteral("File:Alpine Lake.jpg"), QStringLiteral("nature"),
            QStringLiteral("wikimedia"), QStringLiteral("Alice Example"),
            QStringLiteral("CC BY-SA 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by-sa/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Alpine_Lake.jpg"),
            QStringLiteral("An alpine lake"), false);
        add(2, QStringLiteral("File:Ocean Wave.jpg"), QStringLiteral("water"),
            QStringLiteral("wikimedia"), QStringLiteral("Bob"),
            QStringLiteral("CC BY 4.0"),
            QStringLiteral("https://creativecommons.org/licenses/by/4.0/"),
            QStringLiteral("https://commons.wikimedia.org/wiki/File:Ocean_Wave.jpg"),
            QStringLiteral("An ocean wave"), false);
        add(6, QStringLiteral("Space Nebula"), QStringLiteral("space"),
            QStringLiteral("nasa"), QStringLiteral("NASA/JPL-Caltech"),
            QStringLiteral("Public domain (NASA)"),
            QStringLiteral("https://www.nasa.gov/nasa-brand-center/images-and-media/"),
            QStringLiteral("https://images.nasa.gov/details/PIA14417"),
            QStringLiteral("A nebula seen by NASA"), true);
    }
    return QString::fromUtf8(QJsonDocument(items).toJson(QJsonDocument::Compact));
}

void SettingsBridge::setWallpaperFixture(const QString &status)
{
    if (!m_wallpaperFixture)
        return;
    m_wallpaperFixtureMode = status;
    QString providerStatus = status;
    if (status == QStringLiteral("categories") || status == QStringLiteral("many")
        || status == QStringLiteral("deferred")
        || status == QStringLiteral("deferred-fail")) {
        providerStatus = QStringLiteral("ready");
    }
    applyProviderStatus(providerStatus);
    const bool withItems = providerStatus == QStringLiteral("ready");
    applyProviderItems(withItems ? wallpaperFixtureJson(status) : QStringLiteral("[]"));
    applyProviderDefault(withItems ? wallpaperFixturePath(1, false) : QString());
    // Leave `BuiltinDefaultSource` empty so the shared shipped-asset resolver
    // supplies the Built-in "Default" tile exactly as production does.
    applyProviderBuiltin(QString());
}

void SettingsBridge::refreshWallpapers()
{
    if (m_wallpaperFixture) {
        const QString mode = (m_wallpaperFixtureMode == QStringLiteral("offline")
                              || m_wallpaperFixtureMode == QStringLiteral("fetching")
                              || m_wallpaperFixtureMode == QStringLiteral("error"))
                ? QStringLiteral("ready")
                : m_wallpaperFixtureMode;
        applyProviderStatus(QStringLiteral("fetching"));
        QTimer::singleShot(500, this, [this, mode] { setWallpaperFixture(mode); });
        return;
    }
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    const QDBusMessage call = QDBusMessage::createMethodCall(
        kWallpaperService, kWallpaperPath, kWallpaperInterface, QStringLiteral("Refresh"));
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call, 120000), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *watcher) {
                watcher->deleteLater();
                refreshWallpaperProvider();
            });
}

void SettingsBridge::fetchWallpaperItem(int pageid)
{
    if (m_wallpaperFixture) {
        if (m_wallpaperFixtureMode == QStringLiteral("deferred-fail")) {
            QTimer::singleShot(300, this, [this, pageid] {
                emit wallpaperFetchFinished(
                    pageid, false, QStringLiteral("fixture download failed"));
            });
            return;
        }
        const QString path = wallpaperFixturePath(pageid, false);
        bool found = false;
        for (int i = 0; i < m_providerItems.size(); ++i) {
            QVariantMap entry = m_providerItems.at(i).toMap();
            if (entry.value(QStringLiteral("pageid")).toInt() != pageid)
                continue;
            entry.insert(QStringLiteral("localPath"), path);
            entry.insert(QStringLiteral("source"), path);
            entry.insert(QStringLiteral("previewUrl"), QUrl::fromLocalFile(path).toString());
            m_providerItems.replace(i, entry);
            found = true;
            break;
        }
        if (found)
            emit providerChanged();
        QTimer::singleShot(300, this, [this, pageid, found] {
            emit wallpaperFetchFinished(
                pageid, found,
                found ? QString() : QStringLiteral("fixture item not found"));
        });
        return;
    }

    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        emit wallpaperFetchFinished(
            pageid, false, QStringLiteral("The wallpaper service is not available."));
        return;
    }
    QDBusMessage call = QDBusMessage::createMethodCall(
        kWallpaperService, kWallpaperPath, kWallpaperInterface, QStringLiteral("FetchItem"));
    call << QVariant::fromValue<qulonglong>(pageid);
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call, 60000), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, pageid](QDBusPendingCallWatcher *watcher) {
                const QDBusMessage reply = watcher->reply();
                bool ok = false;
                QString message;
                if (reply.type() == QDBusMessage::ErrorMessage) {
                    message = reply.errorMessage();
                } else if (!reply.arguments().isEmpty()) {
                    const QJsonObject object = QJsonDocument::fromJson(
                        reply.arguments().first().toString().toUtf8()).object();
                    ok = object.value(QStringLiteral("ok")).toBool();
                    message = object.value(QStringLiteral("error")).toString();
                }
                refreshWallpaperProvider();
                emit wallpaperFetchFinished(pageid, ok, message);
                watcher->deleteLater();
            });
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

    m_chooserPurpose = QStringLiteral("wallpaper");
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
    if (path.isEmpty())
        return;
    if (m_chooserPurpose == QStringLiteral("backup"))
        emit backupDestinationChosen(path);
    else
        emit wallpaperPhotoChosen(path);
    m_chooserPurpose.clear();
}

void SettingsBridge::chooseBackupDestination()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected() || !m_chooserAvailable)
        return;

    m_chooserPurpose = QStringLiteral("backup");
    QDBusMessage message = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.portal.Desktop"),
        QStringLiteral("/org/freedesktop/portal/desktop"),
        QStringLiteral("org.freedesktop.portal.FileChooser"),
        QStringLiteral("OpenFile"));
    QVariantMap options;
    options.insert(QStringLiteral("multiple"), false);
    options.insert(QStringLiteral("directory"), true);
    options.insert(QStringLiteral("handle_token"), QStringLiteral("dragonfruit_backup"));
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