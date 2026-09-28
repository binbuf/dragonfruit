// SPDX-License-Identifier: MIT
// The Settings app's Network advanced (VPN) seam (T-15.15b).
//
// The Network pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and writes here. The live `DbusVpnClient` talks to
// the bridge host's `org.dragonfruit.SystemStatus1.Vpn` interface (the same
// NetworkManager VPN projection the Control Center tile reads), and
// `MockVpnClient` serves a deterministic fixture for the headless pane tests
// (`DF_VPN_FIXTURE`). The adapter owns the connection state, so the pane has no
// settingsd key of its own; the two writes (`Connect`/`Deactivate`, by UUID)
// are the adapter's. Absence is a normal state: `available()` is false and the
// view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>
#include <QVector>

class QDBusServiceWatcher;

class VpnClient : public QObject
{
    Q_OBJECT

public:
    explicit VpnClient(QObject *parent = nullptr) : QObject(parent) {}
    ~VpnClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `vpn` view (`{state, glyph, label, present, connectionCount,
    // connectedCount, activeUuid, activeName, readOnly, note, connections}`);
    // empty when absent. Each connection is `{id, uuid, kind, kindLabel, state,
    // stateLabel, connected, autoconnect, label}`.
    virtual QVariantMap view() const = 0;
    // Re-read NetworkManager's VPN connections once (the pane calls this on
    // open).
    virtual void refresh() = 0;
    // Connect (activate) or disconnect (deactivate) the connection with
    // `uuid`. The host re-reads and pushes the new view.
    virtual void connectVpn(const QString &uuid) = 0;
    virtual void disconnectVpn(const QString &uuid) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusVpnClient : public VpnClient
{
    Q_OBJECT

public:
    explicit DbusVpnClient(QObject *parent = nullptr);
    ~DbusVpnClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void connectVpn(const QString &uuid) override;
    void disconnectVpn(const QString &uuid) override;

private:
    void call(const QString &method, const QVariantList &arguments);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_VPN_FIXTURE`: a Dragonfruit workstation with
// an OpenVPN connection up and a WireGuard connection idle. The writes mutate
// the simulated store in place and re-emit, so every pane control's round-trip
// is observable with no bus.
class MockVpnClient : public VpnClient
{
    Q_OBJECT

public:
    explicit MockVpnClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void connectVpn(const QString &uuid) override;
    void disconnectVpn(const QString &uuid) override;
    void resetForTest() override;

private:
    void rebuild();
    struct Connection {
        QString id;
        QString uuid;
        QString kind;
        QString kindLabel;
        bool connected = false;
        bool autoconnect = false;
    };
    QVector<Connection> m_connections;
    QVariantMap m_view;
};