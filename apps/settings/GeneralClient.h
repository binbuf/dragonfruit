// SPDX-License-Identifier: MIT
// The Settings app's General sub-panes seam (T226).
//
// Date & Time, Language & Region, Login Items, Sharing, Backup, Storage usage,
// and Startup Disk never touch D-Bus from QML: they bind the `Settings`
// singleton, and the bridge forwards reads and writes here. The live
// `DbusGeneralClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.General` interface; `MockGeneralClient`
// serves the deterministic fixture for the headless pane tests and captures
// (`DF_GENERAL_FIXTURE`). Absence is a normal state: `available()` is false and
// the view is empty.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class GeneralClient : public QObject
{
    Q_OBJECT

public:
    explicit GeneralClient(QObject *parent = nullptr) : QObject(parent) {}
    ~GeneralClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `general` view; empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read every sub-pane once (the pane calls this on open).
    virtual void refresh() = 0;
    // The explicit writes. The host re-reads and pushes the new view.
    virtual void setNtp(bool enabled) = 0;
    virtual void setTimezone(const QString &timezone) = 0;
    virtual void setTime(const QString &value) = 0;
    virtual void setLocale(const QString &locale) = 0;
    virtual void setAutostartEnabled(const QString &id, bool enabled) = 0;
    virtual void setUserUnitEnabled(const QString &unit, bool enabled) = 0;
    virtual void setSharingEnabled(const QString &id, bool enabled) = 0;
    virtual void runBackup(const QString &destination) = 0;
    virtual void openRestore() = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusGeneralClient : public GeneralClient
{
    Q_OBJECT

public:
    explicit DbusGeneralClient(QObject *parent = nullptr);
    ~DbusGeneralClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setNtp(bool enabled) override;
    void setTimezone(const QString &timezone) override;
    void setTime(const QString &value) override;
    void setLocale(const QString &locale) override;
    void setAutostartEnabled(const QString &id, bool enabled) override;
    void setUserUnitEnabled(const QString &unit, bool enabled) override;
    void setSharingEnabled(const QString &id, bool enabled) override;
    void runBackup(const QString &destination) override;
    void openRestore() override;

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

// The fixture client used by `DF_GENERAL_FIXTURE`: a deterministic host with
// every sub-pane populated. Writes mutate the fixture in place and re-emit, so
// a pane control's round-trip is observable with no bus.
class MockGeneralClient : public GeneralClient
{
    Q_OBJECT

public:
    explicit MockGeneralClient(QObject *parent = nullptr);

    bool available() const override { return !m_absent; }
    QVariantMap view() const override { return m_absent ? QVariantMap() : m_view; }
    void refresh() override;
    void setNtp(bool enabled) override;
    void setTimezone(const QString &timezone) override;
    void setTime(const QString &value) override;
    void setLocale(const QString &locale) override;
    void setAutostartEnabled(const QString &id, bool enabled) override;
    void setUserUnitEnabled(const QString &unit, bool enabled) override;
    void setSharingEnabled(const QString &id, bool enabled) override;
    void runBackup(const QString &destination) override;
    void openRestore() override;
    void resetForTest() override;
    // Test/capture seam: pretend the bridge host is absent/returned.
    void setAbsentFixture(bool absent);

private:
    void rebuild();
    void applyToMap(const QString &section, const QString &id, const QString &key,
                    const QVariant &value);
    QVariantMap m_view;
    QString m_lastTime;
    QString m_lastDestination;
    int m_restores = 0;
    bool m_absent = false;
};
