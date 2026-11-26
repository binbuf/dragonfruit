// SPDX-License-Identifier: GPL-3.0-or-later
// The Settings app's Network advanced (VPN) seam (T-15.15b).
//
// The Network pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and writes here. The live `DbusVpnClient` talks to
// the bridge host's `org.dragonfruit.SystemStatus1.Vpn` interface (the same
// NetworkManager VPN projection the Quick Settings tile reads), and
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
    // Add a saved VPN for `plugin` (a NetworkManager service type). The pane
    // supplies a name and an optional gateway; the profile is created
    // incomplete and handed to the installed editor.
    virtual void addVpn(const QString &plugin, const QString &name,
                        const QString &gateway) = 0;
    // Add a complete native WireGuard profile (NetworkManager's `wireguard`
    // type, never a `vpn`/WireGuard service type).
    virtual void addWireguard(const QString &name, const QString &privateKey,
                              const QString &address, const QString &peerPublicKey,
                              const QString &endpoint, const QString &dns,
                              int mtu) = 0;
    // Open the installed editor for an incomplete profile.
    virtual void editVpn(const QString &uuid) = 0;
    // Change whether the saved VPN comes up automatically.
    virtual void setAutoconnect(const QString &uuid, bool enabled) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}
    // Test/capture seam: `live` or `no-plugin`. A no-op on the live client.
    virtual void setFixture(const QString &mode) { Q_UNUSED(mode); }
    // Test seam: make the next write of `operation` fail. A no-op on the live
    // client.
    virtual void failNextWrite(const QString &operation, const QString &message)
    {
        Q_UNUSED(operation);
        Q_UNUSED(message);
    }

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
    // T272: the outcome of one write (`applied`, `absent`, `denied`, or
    // `failed`) and the operation that produced it (`connect`, `disconnect`,
    // `add`, `autoconnect`).
    void writeFinished(const QString &operation, const QString &outcome,
                       const QString &message);
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
    void addVpn(const QString &plugin, const QString &name,
                const QString &gateway) override;
    void addWireguard(const QString &name, const QString &privateKey,
                      const QString &address, const QString &peerPublicKey,
                      const QString &endpoint, const QString &dns,
                      int mtu) override;
    void editVpn(const QString &uuid) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;

private:
    void call(const QString &method, const QVariantList &arguments,
              const QString &operation = QString());
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

    bool available() const override { return !m_absent; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void connectVpn(const QString &uuid) override;
    void disconnectVpn(const QString &uuid) override;
    void addVpn(const QString &plugin, const QString &name,
                const QString &gateway) override;
    void addWireguard(const QString &name, const QString &privateKey,
                      const QString &address, const QString &peerPublicKey,
                      const QString &endpoint, const QString &dns,
                      int mtu) override;
    void editVpn(const QString &uuid) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;
    void resetForTest() override;
    void setFixture(const QString &mode) override;
    void failNextWrite(const QString &operation, const QString &message) override;

private:
    void rebuild();
    bool failWrite(const QString &operation);
    void finishWrite(const QString &operation, const QString &outcome,
                     const QString &message = QString());
    struct Connection {
        QString id;
        QString uuid;
        QString kind;
        QString kindLabel;
        bool connected = false;
        bool autoconnect = false;
        bool needsSetup = false;
    };
    struct Plugin {
        QString serviceType;
        QString label;
    };
    QVector<Connection> m_connections;
    QVector<Plugin> m_plugins;
    bool m_wireguardNative = true;
    bool m_editorAvailable = true;
    QString m_editor = QStringLiteral("nm-connection-editor");
    bool m_readOnly = false;
    bool m_absent = false;
    QString m_readOnlyNote;
    QString m_failOperation;
    QString m_failMessage;
    QVariantMap m_view;
};
