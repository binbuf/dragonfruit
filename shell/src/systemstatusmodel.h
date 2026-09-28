// SPDX-License-Identifier: MIT
// Shell-side system-status model (T-07.5a).
//
// The menu bar's Wi-Fi and volume menus render from the bridge host's JSON
// views (services/system-status, `org.dragonfruit.SystemStatus1`). This model
// is the one decode seam on the shell side: the host's `State()` payload (and
// its action reports) become the maps the QML popovers draw, and the user's
// menu gestures become the request signals the controller forwards to the
// host. Nothing here names a daemon or a D-Bus type, so it is unit-testable
// with plain maps.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class SystemStatusModel : public QObject
{
    Q_OBJECT

public:
    explicit SystemStatusModel(QObject *parent = nullptr);

    // The decoded Wi-Fi / audio / battery views, shaped exactly as the QML
    // consumes them. An unknown kind or a malformed payload clears the view
    // (the safe "absent" default: the item hides).
    QVariantMap wifi() const { return m_wifi; }
    QVariantMap audio() const { return m_audio; }
    QVariantMap battery() const { return m_battery; }
    QVariantMap bluetooth() const { return m_bluetooth; }
    QVariantMap storage() const { return m_storage; }
    QVariantMap input() const { return m_input; }
    QVariantMap updates() const { return m_updates; }
    QVariantMap accounts() const { return m_accounts; }
    QVariantMap printers() const { return m_printers; }
    QVariantMap privacy() const { return m_privacy; }

    // Whether the item is drawn at all (`state != "unavailable"`; the battery
    // also hides when the machine has no present battery).
    bool wifiVisible() const;
    bool audioVisible() const;
    bool batteryVisible() const;
    bool bluetoothVisible() const;
    bool storageVisible() const;
    bool inputVisible() const;
    bool updatesVisible() const;
    bool accountsVisible() const;
    bool printersVisible() const;
    bool privacyVisible() const;

    // Decode a host `State()` payload (JSON object). Returns the normalized
    // map; an empty map on a parse failure, with `error` set when non-null.
    static QVariantMap parseView(const QByteArray &json, const QString &kind,
                                 QString *error = nullptr);

    // The host's action report (`accepted`/`denied`/`absent`/`failed`), for
    // logging and for tests. Returns empty on a parse failure.
    static QString outcomeOf(const QByteArray &json);

public slots:
    // Apply a decoded view from the host. A payload whose `kind` does not
    // match is ignored, so the three menus cannot cross-pollute.
    void applyWifi(const QVariantMap &view);
    void applyAudio(const QVariantMap &view);
    void applyBattery(const QVariantMap &view);
    void applyBluetooth(const QVariantMap &view);
    void applyStorage(const QVariantMap &view);
    void applyInput(const QVariantMap &view);
    void applyUpdates(const QVariantMap &view);
    void applyAccounts(const QVariantMap &view);
    void applyPrinters(const QVariantMap &view);
    void applyPrivacy(const QVariantMap &view);
    void applyWifiJson(const QByteArray &json);
    void applyAudioJson(const QByteArray &json);
    void applyBatteryJson(const QByteArray &json);
    void applyBluetoothJson(const QByteArray &json);
    void applyStorageJson(const QByteArray &json);
    void applyInputJson(const QByteArray &json);
    void applyUpdatesJson(const QByteArray &json);
    void applyAccountsJson(const QByteArray &json);
    void applyPrintersJson(const QByteArray &json);
    void applyPrivacyJson(const QByteArray &json);

    // User gestures from the popovers; the controller forwards each to the
    // bridge host. They do not mutate the view (the host re-read is the only
    // source of truth). The battery is read-only: refresh only.
    void requestJoin(const QString &ssid, const QString &secret);
    void requestVolume(double volume);
    void requestMute(bool muted);
    void requestRefreshWifi();
    void requestRefreshAudio();
    void requestRefreshBattery();
    void requestRefreshBluetooth();
    void requestRefreshStorage();
    // The input inventory is read-only: refresh only, no write.
    void requestRefreshInput();
    // General/About/Updates: refresh plus the three explicit update writes.
    void requestRefreshUpdates();
    void requestCheckUpdates();
    void requestInstallUpdates();
    void requestRebootUpdates();
    // Users and Groups is a read-only summary: refresh only, no write.
    void requestRefreshAccounts();
    // Printers and Scanners is a read-only summary: refresh only, no write.
    void requestRefreshPrinters();
    // Privacy and Security is a read-only summary: refresh only, no write.
    void requestRefreshPrivacy();

signals:
    void changed();
    void joinRequested(const QString &ssid, const QString &secret);
    void volumeRequested(double volume);
    void muteRequested(bool muted);
    void refreshWifiRequested();
    void refreshAudioRequested();
    void refreshBatteryRequested();
    void refreshBluetoothRequested();
    void refreshStorageRequested();
    void refreshInputRequested();
    void refreshUpdatesRequested();
    void checkUpdatesRequested();
    void installUpdatesRequested();
    void rebootUpdatesRequested();
    void refreshAccountsRequested();
    void refreshPrintersRequested();
    void refreshPrivacyRequested();

private:
    static QVariantMap normalize(const QVariantMap &view, const QString &kind);
    QVariantMap m_wifi;
    QVariantMap m_audio;
    QVariantMap m_battery;
    QVariantMap m_bluetooth;
    QVariantMap m_storage;
    QVariantMap m_input;
    QVariantMap m_updates;
    QVariantMap m_accounts;
    QVariantMap m_printers;
    QVariantMap m_privacy;
};