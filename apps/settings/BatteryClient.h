// SPDX-License-Identifier: MIT
// The Settings app's Battery seam (T-15.6b).
//
// The Battery pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and the one profile write here. The live
// `DbusBatteryClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Battery` interface (the same view the Control
// Center tile and menu bar read), and `MockBatteryClient` serves a
// deterministic fixture for the headless pane tests (`DF_BATTERY_FIXTURE`).
//
// Absence is layered and normal: the host itself may be absent (`available()`
// is false); UPower may be absent while power-profiles-daemon runs (no battery,
// but profiles); or a battery may be present with no profiles daemon. The view
// carries `present` and `profilesAvailable` so the pane can render each case
// without an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class BatteryClient : public QObject
{
    Q_OBJECT

public:
    explicit BatteryClient(QObject *parent = nullptr) : QObject(parent) {}
    ~BatteryClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `battery` view (`{state, present, percent, level, charging,
    // plugged, onBattery, chargeState, health, healthLabel, capacity,
    // chargeCycles, profilesAvailable, activeProfile, profileLabel, profiles}`);
    // empty when the host is absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // Select the active power profile by its stable id. One explicit write;
    // the host re-reads and pushes the new view.
    virtual void setActiveProfile(const QString &profile) = 0;

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusBatteryClient : public BatteryClient
{
    Q_OBJECT

public:
    explicit DbusBatteryClient(QObject *parent = nullptr);
    ~DbusBatteryClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setActiveProfile(const QString &profile) override;

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

// The fixture client used by `DF_BATTERY_FIXTURE`: a present, discharging
// battery with all three power profiles. The profile write mutates the active
// profile and re-emits, so a pane control's round-trip is observable with no
// bus.
class MockBatteryClient : public BatteryClient
{
    Q_OBJECT

public:
    explicit MockBatteryClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setActiveProfile(const QString &profile) override;

private:
    void rebuild();
    QString m_activeProfile = QStringLiteral("balanced");
    QVariantMap m_view;
};