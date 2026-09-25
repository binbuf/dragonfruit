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
#include <QDBusVariant>
#include <QDir>
#include <cstdio>
#include <QFileInfo>
#include <QImage>
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