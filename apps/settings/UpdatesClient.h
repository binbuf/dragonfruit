// SPDX-License-Identifier: MIT
// The Settings app's General/About/Updates seam (T-15.10b).
//
// The General pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and writes here. The live `DbusUpdatesClient`
// talks to the bridge host's `org.dragonfruit.SystemStatus1.Updates` interface
// (the same adapter the Control Center tile reads), and `MockUpdatesClient`
// serves a deterministic fixture for the headless pane tests
// (`DF_UPDATES_FIXTURE`). Absence is a normal state: `available()` is false and
// the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class UpdatesClient : public QObject
{
    Q_OBJECT

public:
    explicit UpdatesClient(QObject *parent = nullptr) : QObject(parent) {}
    ~UpdatesClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `updates` view (`{state, hostName, osLabel, memoryLabel,
    // updatesAvailable, phase, label, updates, ...}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host stack once (the pane calls this on open).
    virtual void refresh() = 0;
    // The three explicit writes; the host re-reads and pushes the new view.
    virtual void check() = 0;
    virtual void install() = 0;
    virtual void reboot() = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusUpdatesClient : public UpdatesClient
{
    Q_OBJECT

public:
    explicit DbusUpdatesClient(QObject *parent = nullptr);
    ~DbusUpdatesClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void check() override;
    void install() override;
    void reboot() override;

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

// The fixture client used by `DF_UPDATES_FIXTURE`: a Dragonfruit host identity
// with one security update on offer. `check`/`install`/`reboot` mutate the
// phase in place and re-emit, so a pane control's round-trip is observable with
// no bus.
class MockUpdatesClient : public UpdatesClient
{
    Q_OBJECT

public:
    explicit MockUpdatesClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void check() override;
    void install() override;
    void reboot() override;
    void resetForTest() override;

private:
    void rebuild();
    QString m_phase = QStringLiteral("available");
    int m_updateCount = 1;
    QVariantMap m_view;
};