// SPDX-License-Identifier: MIT
// Shell-side client for the T-07.5a bridge host
// (`org.dragonfruit.SystemStatus1`, services/system-status).
//
// The abstract seam keeps the menu controller free of D-Bus: the live
// `DbusSystemStatusClient` talks to the host, and `MockSystemStatusClient`
// serves a fixture for headless use and the capture script (selected with the
// `DF_STATUS_FIXTURE` environment variable, since `--placeholders` is gone).
// Both publish the host's JSON payloads unchanged, so the model decode path is
// the same in the demo and in a real session.
#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>
#include <QVariantList>

class SystemStatusClient : public QObject
{
    Q_OBJECT

public:
    explicit SystemStatusClient(QObject *parent = nullptr) : QObject(parent) {}
    ~SystemStatusClient() override = default;

    // Whether the host is reachable. When false the menus have nothing to
    // show and the items hide.
    virtual bool isAvailable() const = 0;

    virtual void refreshWifi() = 0;
    virtual void refreshAudio() = 0;
    // The battery item is read-only: there is no write action.
    virtual void refreshBattery() = 0;
    // Bluetooth (T-15.1b): a live state read plus the four explicit writes the
    // Control Center tile and the Settings pane raise.
    virtual void refreshBluetooth() = 0;
    virtual void setBluetoothPowered(bool powered) = 0;
    virtual void setBluetoothDiscovering(bool discovering) = 0;
    virtual void pairBluetooth(const QString &address) = 0;
    virtual void setBluetoothConnected(const QString &address, bool connected) = 0;
    // Storage (T-15.2b): a live state read plus the three explicit writes the
    // Control Center tile and the Settings pane raise.
    virtual void refreshStorage() = 0;
    virtual void mountStorage(const QString &volumePath) = 0;
    virtual void unmountStorage(const QString &volumePath) = 0;
    virtual void ejectStorage(const QString &drivePath) = 0;
    // Input (T-15.4b): a read-only inventory. Refresh only; libinput has no
    // setter (ADR 0124).
    virtual void refreshInput() = 0;
    // General/About/Updates (T-15.10b): a live host-stack read plus the three
    // explicit update writes the Control Center tile and the Settings pane
    // raise.
    virtual void refreshUpdates() = 0;
    virtual void checkUpdates() = 0;
    virtual void installUpdates() = 0;
    virtual void rebootUpdates() = 0;
    // Users and Groups (T-15.11b): a read-only summary. The tile reflects the
    // user/group state; the writes live in the Settings pane (ADR 0139).
    virtual void refreshAccounts() = 0;
    // Printers and Scanners (T-15.12b): a read-only summary. The tile reflects
    // the bridge host's CUPS/SANE view; the queue writes live in the Settings
    // pane (ADR 0141).
    virtual void refreshPrinters() = 0;
    // Privacy and Security (T-15.13b): a read-only summary. The tile reflects
    // the bridge host's portal PermissionStore view; the permission writes live
    // in the Settings pane (ADR 0142).
    virtual void refreshPrivacy() = 0;
    // Accessibility (T-15.14b): a read-only summary. The tile reflects the
    // bridge host's live AT-SPI view (`org.a11y.Status`); the adapter has no
    // setter and the pane's durable preferences are settingsd keys (ADR 0144).
    virtual void refreshAccessibility() = 0;
    virtual void join(const QString &ssid, const QString &secret) = 0;
    virtual void setVolume(double volume) = 0;
    virtual void setMute(bool muted) = 0;

signals:
    void availableChanged(bool available);
    void wifiState(const QByteArray &json);
    void audioState(const QByteArray &json);
    void batteryState(const QByteArray &json);
    void bluetoothState(const QByteArray &json);
    void storageState(const QByteArray &json);
    void inputState(const QByteArray &json);
    void updatesState(const QByteArray &json);
    void accountsState(const QByteArray &json);
    void printersState(const QByteArray &json);
    void privacyState(const QByteArray &json);
    void accessibilityState(const QByteArray &json);
    void joinReport(const QByteArray &json);
    void writeReport(const QByteArray &json);
};

// The live client over the user session bus.
class DbusSystemStatusClient : public SystemStatusClient
{
    Q_OBJECT

public:
    explicit DbusSystemStatusClient(QObject *parent = nullptr);

    bool isAvailable() const override;
    void refreshWifi() override;
    void refreshAudio() override;
    void refreshBattery() override;
    void refreshBluetooth() override;
    void setBluetoothPowered(bool powered) override;
    void setBluetoothDiscovering(bool discovering) override;
    void pairBluetooth(const QString &address) override;
    void setBluetoothConnected(const QString &address, bool connected) override;
    void refreshStorage() override;
    void mountStorage(const QString &volumePath) override;
    void unmountStorage(const QString &volumePath) override;
    void ejectStorage(const QString &drivePath) override;
    void refreshInput() override;
    void refreshUpdates() override;
    void checkUpdates() override;
    void installUpdates() override;
    void rebootUpdates() override;
    void refreshAccounts() override;
    void refreshPrinters() override;
    void refreshPrivacy() override;
    void refreshAccessibility() override;
    void join(const QString &ssid, const QString &secret) override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;

private:
    using ReplySignal = void (SystemStatusClient::*)(const QByteArray &);
    void call(const QString &interface, const QString &method,
              const QVariantList &arguments, ReplySignal replySignal);
    // The service name/path are compiled in so the demo and the live shell
    // address the same object.
    QString m_service;
    QString m_path;
    bool m_available = false;
};

// The fixture client used by `DF_STATUS_FIXTURE` and the headless tests. It
// simulates the host well enough for the demo: volume/mute mutate its view
// and re-emit, so the status glyph and slider move. Its battery view reports
// a present charging battery so the demo can render the item on a host with
// no battery of its own.
class MockSystemStatusClient : public SystemStatusClient
{
    Q_OBJECT

public:
    explicit MockSystemStatusClient(QObject *parent = nullptr);

    bool isAvailable() const override { return true; }
    void refreshWifi() override;
    void refreshAudio() override;
    void refreshBattery() override;
    void refreshBluetooth() override;
    void setBluetoothPowered(bool powered) override;
    void setBluetoothDiscovering(bool discovering) override;
    void pairBluetooth(const QString &address) override;
    void setBluetoothConnected(const QString &address, bool connected) override;
    void refreshStorage() override;
    void mountStorage(const QString &volumePath) override;
    void unmountStorage(const QString &volumePath) override;
    void ejectStorage(const QString &drivePath) override;
    void refreshInput() override;
    void refreshUpdates() override;
    void checkUpdates() override;
    void installUpdates() override;
    void rebootUpdates() override;
    void refreshAccounts() override;
    void refreshPrinters() override;
    void refreshPrivacy() override;
    void refreshAccessibility() override;
    void join(const QString &ssid, const QString &secret) override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;

private:
    QString m_activeSsid = QStringLiteral("dragonfruit");
    double m_volume = 0.6;
    bool m_muted = false;
    // The audio routing fixture (T-15.3b): the default output/input names the
    // tile reflects; the fixture never switches them (routing is the Settings
    // pane's write path).
    QString m_defaultSinkName = QStringLiteral("speakers");
    QString m_defaultSourceName = QStringLiteral("microphone");
    // The Bluetooth fixture state (T-15.1b): a powered adapter with one known
    // device and one discoverable device. Writes mutate it and re-emit, so the
    // Control Center tile's round-trip is observable headlessly.
    bool m_btPowered = true;
    bool m_btDiscovering = false;
    bool m_btDeviceConnected = false;
    // The storage fixture state (T-15.2b): one removable drive with a volume
    // that mount/unmount toggles in place, so the tile's round-trip is
    // observable headlessly (`DF_STATUS_FIXTURE`).
    bool m_storageVolumeMounted = false;
    bool m_storageDrivePresent = true;
    // The General/About/Updates fixture state (T-15.10b): one security update
    // on offer. check/install/reboot mutate the phase in place and re-emit, so
    // the Control Center tile's round-trip is observable headlessly.
    QString m_updatesPhase = QStringLiteral("available");
    int m_updatesCount = 1;
};