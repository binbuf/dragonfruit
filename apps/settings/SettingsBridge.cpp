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
    connect(m_client, &SettingsClient::writeStateChanged, this,
            [this](const QString &) { emit writeStatesChanged(); });
    connect(m_client, &SettingsClient::writeFinished, this,
            [this](const QString &key, const QVariant &value, bool ok,
                   const QString &message) {
                // A rejected write never emitted `changed`, so re-evaluate
                // `Settings.values` bindings explicitly; a reverted write did
                // emit one. Both paths must snap the control back.
                if (!ok)
                    emit valuesChanged();
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

    // T226: the General sub-panes seam, selected the same way.
    if (qEnvironmentVariableIsSet("DF_GENERAL_FIXTURE"))
        m_general = new MockGeneralClient(this);
    else
        m_general = new DbusGeneralClient(this);
    connect(m_general, &GeneralClient::changed, this,
            [this](const QVariantMap &) { emit generalChanged(); });
    connect(m_general, &GeneralClient::availableChanged, this,
            [this](bool) { emit generalChanged(); });

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

    // T225: the Wi-Fi and Network/Firewall seams, selected the same way.
    if (qEnvironmentVariableIsSet("DF_WIFI_FIXTURE"))
        m_wifi = new MockWifiClient(this);
    else
        m_wifi = new DbusWifiClient(this);
    connect(m_wifi, &WifiClient::changed, this,
            [this](const QVariantMap &) { emit wifiChanged(); });
    connect(m_wifi, &WifiClient::availableChanged, this,
            [this](bool) { emit wifiChanged(); });

    if (qEnvironmentVariableIsSet("DF_NETWORK_FIXTURE"))
        m_network = new MockNetworkClient(this);
    else
        m_network = new DbusNetworkClient(this);
    connect(m_network, &NetworkClient::changed, this,
            [this](const QVariantMap &) { emit networkChanged(); });
    connect(m_network, &NetworkClient::availableChanged, this,
            [this](bool) { emit networkChanged(); });

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
    // `material_preset::Rollout::from_env`: anything other than an explicit
    // `replacement` is the T208-safe legacy look.
    const QString requested =
        qEnvironmentVariable("DRAGONFRUIT_GLASS_ROLLOUT").trimmed().toLower();
    return requested == QLatin1String("replacement")
        ? QStringLiteral("replacement") : QStringLiteral("legacy");
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

void SettingsBridge::refreshGeneral()
{
    if (m_general)
        m_general->refresh();
}

void SettingsBridge::setNtp(bool enabled)
{
    if (m_general)
        m_general->setNtp(enabled);
}

void SettingsBridge::setTimezone(const QString &timezone)
{
    if (m_general)
        m_general->setTimezone(timezone);
}

void SettingsBridge::setTime(const QString &value)
{
    if (m_general)
        m_general->setTime(value);
}

void SettingsBridge::setLocale(const QString &locale)
{
    if (m_general)
        m_general->setLocale(locale);
}

void SettingsBridge::setAutostartEnabled(const QString &id, bool enabled)
{
    if (m_general)
        m_general->setAutostartEnabled(id, enabled);
}

void SettingsBridge::setUserUnitEnabled(const QString &unit, bool enabled)
{
    if (m_general)
        m_general->setUserUnitEnabled(unit, enabled);
}

void SettingsBridge::setSharingEnabled(const QString &id, bool enabled)
{
    if (m_general)
        m_general->setSharingEnabled(id, enabled);
}

void SettingsBridge::runBackup(const QString &destination)
{
    if (m_general)
        m_general->runBackup(destination);
}

void SettingsBridge::openBackupRestore()
{
    if (m_general)
        m_general->openRestore();
}

void SettingsBridge::applyUpdatePolicy(bool autoCheck, bool autoInstall)
{
    if (m_updates)
        m_updates->setAutoPolicy(autoCheck, autoInstall);
}

void SettingsBridge::resetGeneralFixture()
{
    if (m_general)
        m_general->resetForTest();
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

void SettingsBridge::addVpn(const QString &plugin, const QString &name,
                            const QString &gateway)
{
    if (m_vpn && !plugin.isEmpty() && !name.isEmpty())
        m_vpn->addVpn(plugin, name, gateway);
}

void SettingsBridge::setVpnAutoconnect(const QString &uuid, bool enabled)
{
    if (m_vpn && !uuid.isEmpty())
        m_vpn->setAutoconnect(uuid, enabled);
}

void SettingsBridge::resetVpnFixture()
{
    if (m_vpn)
        m_vpn->resetForTest();
}

void SettingsBridge::setVpnFixture(const QString &mode)
{
    if (m_vpn)
        m_vpn->setFixture(mode);
}

void SettingsBridge::refreshWifi()
{
    if (m_wifi)
        m_wifi->refresh();
}

void SettingsBridge::setWifiEnabled(bool enabled)
{
    if (m_wifi)
        m_wifi->setEnabled(enabled);
}

void SettingsBridge::joinWifi(const QString &ssid, const QString &secret)
{
    if (m_wifi && !ssid.isEmpty())
        m_wifi->join(ssid, secret);
}

void SettingsBridge::joinHiddenWifi(const QString &ssid, const QString &secret)
{
    if (m_wifi && !ssid.isEmpty())
        m_wifi->joinHidden(ssid, secret);
}

void SettingsBridge::forgetWifi(const QString &uuid)
{
    if (m_wifi && !uuid.isEmpty())
        m_wifi->forget(uuid);
}

void SettingsBridge::setWifiAutoconnect(const QString &uuid, bool enabled)
{
    if (m_wifi && !uuid.isEmpty())
        m_wifi->setAutoconnect(uuid, enabled);
}

void SettingsBridge::resetWifiFixture()
{
    if (m_wifi)
        m_wifi->resetForTest();
}

void SettingsBridge::setWifiFixture(const QString &mode)
{
    if (m_wifi)
        m_wifi->setFixture(mode);
}

void SettingsBridge::refreshNetwork()
{
    if (m_network)
        m_network->refresh();
}

void SettingsBridge::activateNetworkService(const QString &uuid)
{
    if (m_network && !uuid.isEmpty())
        m_network->activateService(uuid);
}

void SettingsBridge::deactivateNetworkService(const QString &uuid)
{
    if (m_network && !uuid.isEmpty())
        m_network->deactivateService(uuid);
}

void SettingsBridge::moveNetworkService(const QString &uuid, int delta)
{
    if (m_network && !uuid.isEmpty() && delta != 0)
        m_network->moveService(uuid, delta);
}

void SettingsBridge::addNetworkService(const QString &name, const QString &kind,
                                       const QString &plugin,
                                       const QString &gateway)
{
    if (m_network && !name.isEmpty())
        m_network->addService(name, kind, plugin, gateway);
}

void SettingsBridge::setNetworkAutoconnect(const QString &uuid, bool enabled)
{
    if (m_network && !uuid.isEmpty())
        m_network->setAutoconnect(uuid, enabled);
}

void SettingsBridge::setFirewallEnabled(bool enabled)
{
    if (m_network)
        m_network->setFirewallEnabled(enabled);
}

void SettingsBridge::openFirewallPort(int port, const QString &protocol)
{
    if (m_network)
        m_network->openPort(port, protocol);
}

void SettingsBridge::closeFirewallPort(int port, const QString &protocol)
{
    if (m_network)
        m_network->closePort(port, protocol);
}

void SettingsBridge::openFirewallService(const QString &service)
{
    if (m_network && !service.isEmpty())
        m_network->openService(service);
}

void SettingsBridge::closeFirewallService(const QString &service)
{
    if (m_network && !service.isEmpty())
        m_network->closeService(service);
}

void SettingsBridge::resetNetworkFixture()
{
    if (m_network)
        m_network->resetForTest();
}

void SettingsBridge::setNetworkFixture(const QString &mode)
{
    if (m_network)
        m_network->setFixture(mode);
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