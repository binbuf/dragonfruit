// SPDX-License-Identifier: MIT
// The Settings app's Bluetooth seam (T-15.1b).
//
// The Bluetooth pane never touches D-Bus: it binds the `Settings` singleton,
// and the bridge forwards reads and writes here. The live
// `DbusBluetoothClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Bluetooth` interface (the same adapter the
// Control Center tile reads), and `MockBluetoothClient` serves a deterministic
// fixture for the headless pane tests (`DF_BLUETOOTH_FIXTURE`). Absence is a
// normal state: `available()` is false and the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

class QDBusServiceWatcher;

class BluetoothClient : public QObject
{
    Q_OBJECT

public:
    explicit BluetoothClient(QObject *parent = nullptr) : QObject(parent) {}
    ~BluetoothClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `bluetooth` view (`{state, present, powered, discovering,
    // label, adapterName, knownDevices, nearbyDevices}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // The four explicit writes; the host re-reads and pushes the new view.
    virtual void setPowered(bool powered) = 0;
    virtual void setDiscovering(bool discovering) = 0;
    virtual void pair(const QString &address) = 0;
    virtual void setConnected(const QString &address, bool connected) = 0;
    // T271: forget (remove) a device from the adapter.
    virtual void forget(const QString &address) = 0;
    // Test/capture seam: restore the fixture's initial state. A no-op on the
    // live client.
    virtual void resetForTest() {}
    // Test/capture seam: `live`, `off`, `empty`, `error`, or `absent`.
    virtual void setFixture(const QString &mode) { Q_UNUSED(mode); }
    // T271 test seam: make the next write of `operation` fail.
    virtual void failNextWrite(const QString &operation, const QString &message)
    {
        Q_UNUSED(operation);
        Q_UNUSED(message);
    }

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
    // T271: the outcome of one write (`applied`, `absent`, `denied`, or
    // `failed`) and the operation (`power`, `discovery`, `pair`, `connect`,
    // `forget`).
    void writeFinished(const QString &operation, const QString &outcome,
                       const QString &message);
};

// The live client over the user session bus.
class DbusBluetoothClient : public BluetoothClient
{
    Q_OBJECT

public:
    explicit DbusBluetoothClient(QObject *parent = nullptr);
    ~DbusBluetoothClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setPowered(bool powered) override;
    void setDiscovering(bool discovering) override;
    void pair(const QString &address) override;
    void setConnected(const QString &address, bool connected) override;
    void forget(const QString &address) override;

private:
    void call(const QString &method, const QVariantList &arguments,
              const QString &operation = QString(), bool isState = false);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_BLUETOOTH_FIXTURE`: a powered adapter with a
// known headset, and a nearby speaker that appears once discovery starts.
// Writes mutate the fixture and re-emit, so a pane control's round-trip is
// observable with no bus.
class MockBluetoothClient : public BluetoothClient
{
    Q_OBJECT

public:
    explicit MockBluetoothClient(QObject *parent = nullptr);

    bool available() const override { return !m_absent; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setPowered(bool powered) override;
    void setDiscovering(bool discovering) override;
    void pair(const QString &address) override;
    void setConnected(const QString &address, bool connected) override;
    void forget(const QString &address) override;
    void resetForTest() override;
    void setFixture(const QString &mode) override;
    void failNextWrite(const QString &operation, const QString &message) override;

private:
    void rebuild();
    bool failWrite(const QString &operation);
    void finishWrite(const QString &operation, const QString &outcome,
                     const QString &message = QString());
    bool m_absent = false;
    bool m_error = false;
    bool m_powered = true;
    bool m_discovering = false;
    bool m_headsetConnected = false;
    QStringList m_forgotten;
    QString m_failOperation;
    QString m_failMessage;
    QVariantMap m_view;
};