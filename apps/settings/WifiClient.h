// SPDX-License-Identifier: MIT
// The Settings app's Wi-Fi seam (T225).
//
// The Wi-Fi pane and the shell's Wi-Fi quick sheet read the same bridge-host
// view: the radio state, the current network, the visible networks, the saved
// services, and the known Wi-Fi networks. The pane never touches D-Bus; the
// bridge forwards reads and writes here. `DbusWifiClient` talks to
// `org.dragonfruit.SystemStatus1.Wifi`, and `MockWifiClient` serves a
// deterministic fixture for the headless pane tests and captures
// (`DF_WIFI_FIXTURE`). Absence is a normal state: `available()` is false and
// the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>
#include <QVariantList>
#include <QVector>

class QDBusServiceWatcher;

class WifiClient : public QObject
{
    Q_OBJECT

public:
    explicit WifiClient(QObject *parent = nullptr) : QObject(parent) {}
    ~WifiClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `wifi` view (`{state, glyph, label, radioEnabled, wifiState,
    // connectivity, activeSsid, activeAddress, readOnly, note, networks,
    // knownNetworks, services}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // The explicit writes; the host re-reads and pushes the new view.
    virtual void setEnabled(bool enabled) = 0;
    virtual void join(const QString &ssid, const QString &secret) = 0;
    virtual void joinHidden(const QString &ssid, const QString &secret) = 0;
    virtual void forget(const QString &uuid) = 0;
    virtual void setAutoconnect(const QString &uuid, bool enabled) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}
    // Test/capture seam: `live`, `off`, or `absent`. A no-op on the live
    // client.
    virtual void setFixture(const QString &mode) { Q_UNUSED(mode); }

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusWifiClient : public WifiClient
{
    Q_OBJECT

public:
    explicit DbusWifiClient(QObject *parent = nullptr);
    ~DbusWifiClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setEnabled(bool enabled) override;
    void join(const QString &ssid, const QString &secret) override;
    void joinHidden(const QString &ssid, const QString &secret) override;
    void forget(const QString &uuid) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;

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

// The fixture client used by `DF_WIFI_FIXTURE`: a connected secured network,
// two nearby networks, and two saved networks. Writes mutate the simulated
// store and re-emit, so a join/forget round-trip is observable with no bus.
class MockWifiClient : public WifiClient
{
    Q_OBJECT

public:
    explicit MockWifiClient(QObject *parent = nullptr);

    bool available() const override { return m_available; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setEnabled(bool enabled) override;
    void join(const QString &ssid, const QString &secret) override;
    void joinHidden(const QString &ssid, const QString &secret) override;
    void forget(const QString &uuid) override;
    void setAutoconnect(const QString &uuid, bool enabled) override;
    void resetForTest() override;
    void setFixture(const QString &mode) override;

private:
    void rebuild();
    struct Network {
        QString ssid;
        int strength = 0;
        QString security;
        bool secured = false;
        bool active = false;
        int frequency = 2412;
    };
    struct Known {
        QString ssid;
        QString uuid;
        bool autoconnect = true;
        bool connected = false;
        qint64 lastUsed = 0;
        QString security;
        bool secured = false;
    };
    QVector<Network> m_networks;
    QVector<Known> m_known;
    bool m_available = true;
    bool m_enabled = true;
    QString m_activeSsid;
    QVariantMap m_view;
};
