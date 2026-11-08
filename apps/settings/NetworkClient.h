// SPDX-License-Identifier: MIT
// The Settings app's Network pane seam (T225).
//
// The Network pane and its Firewall sheet never touch D-Bus. The live client
// reads `org.dragonfruit.SystemStatus1.Network` (the combined Wi-Fi/service
// snapshot plus the firewall sub-view) and writes through the same interface
// for services, and through `org.dragonfruit.SystemStatus1.Firewall` for the
// firewall. `MockNetworkClient` serves a deterministic fixture for the
// headless pane tests and captures (`DF_NETWORK_FIXTURE`). Absence is a normal
// state: `available()` is false and the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>
#include <QVector>

class QDBusServiceWatcher;

class NetworkClient : public QObject
{
    Q_OBJECT

public:
    explicit NetworkClient(QObject *parent = nullptr) : QObject(parent) {}
    ~NetworkClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `network` view (`{state, services, firewall, ...}`); empty
    // when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // Service writes; the host re-reads and pushes the new view.
    virtual void activateService(const QString &uuid) = 0;
    virtual void deactivateService(const QString &uuid) = 0;
    virtual void moveService(const QString &uuid, int delta) = 0;
    virtual void addService(const QString &name, const QString &kind,
                            const QString &plugin, const QString &gateway) = 0;
    virtual void setAutoconnect(const QString &uuid, bool enabled) = 0;
    // Firewall writes.
    virtual void setFirewallEnabled(bool enabled) = 0;
    virtual void openPort(int port, const QString &protocol) = 0;
    virtual void closePort(int port, const QString &protocol) = 0;
    virtual void openService(const QString &service) = 0;
    virtual void closeService(const QString &service) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusNetworkClient : public NetworkClient
{
    Q_OBJECT

public:
    explicit DbusNetworkClient(QObject *parent = nullptr);
    ~DbusNetworkClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void activateService(const QString &uuid) override;
    void deactivateService(const QString &uuid) override;
    void moveService(const QString &uuid, int delta) override;
    void addService(const QString &name, const QString &kind,
                    const QString &plugin, const QString &gateway) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;
    void setFirewallEnabled(bool enabled) override;
    void openPort(int port, const QString &protocol) override;
    void closePort(int port, const QString &protocol) override;
    void openService(const QString &service) override;
    void closeService(const QString &service) override;

private:
    void call(const QString &interface, const QString &method,
              const QVariantList &arguments, bool isState);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_networkInterface;
    QString m_firewallInterface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_NETWORK_FIXTURE`: a wired + Wi-Fi service
// list, a bridge, and a firewalld with two ports/services. Writes mutate the
// simulated store and re-emit. `DF_NETWORK_FIXTURE=nftables` makes the
// firewall detected but status-unavailable; `absent` makes the whole host
// absent.
class MockNetworkClient : public NetworkClient
{
    Q_OBJECT

public:
    explicit MockNetworkClient(QObject *parent = nullptr);

    bool available() const override { return m_available; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void activateService(const QString &uuid) override;
    void deactivateService(const QString &uuid) override;
    void moveService(const QString &uuid, int delta) override;
    void addService(const QString &name, const QString &kind,
                    const QString &plugin, const QString &gateway) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;
    void setFirewallEnabled(bool enabled) override;
    void openPort(int port, const QString &protocol) override;
    void closePort(int port, const QString &protocol) override;
    void openService(const QString &service) override;
    void closeService(const QString &service) override;
    void resetForTest() override;

private:
    void rebuild();
    struct Service {
        QString id;
        QString uuid;
        QString kind;
        bool connected = false;
        bool autoconnect = true;
        int priority = 0;
        QString interface;
        QString hardware;
        QString address;
    };
    QVector<Service> m_services;
    bool m_available = true;
    bool m_firewallDetected = true;
    bool m_firewallStatusKnown = true;
    bool m_firewallEnabled = false;
    QString m_firewallBackend = QStringLiteral("firewalld");
    QString m_firewallNote;
    QVariantList m_ports;
    QStringList m_servicesList;
    QVariantMap m_view;
};
