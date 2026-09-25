// SPDX-License-Identifier: MIT
// Shell-side client for the T-08 settings daemon
// (`org.dragonfruit.Settings1`, services/settingsd).
//
// The abstract seam keeps the Dock controller free of D-Bus: the live
// `DbusSettingsClient` talks to the daemon and subscribes to its `Changed`
// signal (never polling); `MockSettingsClient` serves the schema defaults plus
// in-process writes for headless unit tests and the isolated-client test.
//
// Both are seeded with the schema defaults (mirrored from
// services/settingsd/src/schema.rs) so the shell has a complete, working Dock
// configuration even when no daemon is on the bus: the demo/dev tool
// deliberately does not start settingsd, and the Dock must still lay out. When
// the daemon appears, `refresh()` (GetAll) makes its persisted state
// authoritative; a write is applied locally first and mirrored to the daemon,
// whose `Changed` echo is de-duplicated.
#pragma once

#include <QDBusVariant>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantMap>

class QDBusServiceWatcher;

class SettingsClient : public QObject
{
    Q_OBJECT

public:
    explicit SettingsClient(QObject *parent = nullptr);
    ~SettingsClient() override;

    // Whether the daemon owns the session-bus name. When false the client
    // serves the schema defaults and local writes only.
    virtual bool isAvailable() const = 0;
    // Re-read every key (`GetAll`) and emit `changed` per key that differs.
    virtual void refresh() = 0;
    // Write a key. The local store updates immediately; when the daemon is
    // reachable the write is mirrored to it and it becomes the single writer.
    virtual void set(const QString &key, const QVariant &value) = 0;

    QVariant value(const QString &key, const QVariant &fallback = {}) const;
    bool boolean(const QString &key, bool fallback) const;
    double real(const QString &key, double fallback) const;
    qlonglong integer(const QString &key, qlonglong fallback) const;
    QString string(const QString &key, const QString &fallback) const;
    QStringList stringList(const QString &key) const;
    QVariantMap values() const { return m_values; }

signals:
    void availableChanged(bool available);
    // A key's value really changed (local write or daemon signal). Emitted
    // only when the value differs from the current one.
    void changed(const QString &key, const QVariant &value);
    // A `refresh()` finished applying the daemon's snapshot (or, for a mock,
    // immediately).
    void refreshed();

protected:
    // Update one value and emit `changed` when it really differs.
    void applyValue(const QString &key, const QVariant &value);
    // Update every key in `values`, emitting `changed` per real difference.
    void applyValues(const QVariantMap &values);
    void setAvailable(bool available);

    QVariantMap m_values;
    bool m_available = false;
};

// The `org.dragonfruit.Settings1` schema defaults, mirrored from
// services/settingsd/src/schema.rs. Adding a key there means adding it here
// (and to the consumers that read it); the values are the frozen v1 contract.
QVariantMap settingsSchemaDefaults();

// The live client over the user session bus.
class DbusSettingsClient : public SettingsClient
{
    Q_OBJECT

public:
    explicit DbusSettingsClient(QObject *parent = nullptr);

    bool isAvailable() const override;
    void refresh() override;
    void set(const QString &key, const QVariant &value) override;

private slots:
    void onRemoteChanged(const QString &key, const QDBusVariant &value);
    void onServiceRegistered(const QString &name);
    void onServiceUnregistered(const QString &name);

private:
    void subscribeToChanges();
    QString m_service;
    QString m_path;
    QString m_interface;
    // Watches the well-known name across daemon restarts. The bus
    // interface's own serviceRegistered/serviceUnregistered signals only
    // cover unique names, so this is required for restart resync (T-08.3).
    QDBusServiceWatcher *m_watcher = nullptr;
};

// A fixture-backed client: the schema defaults, with writes visible
// in-process. Used by the headless tests and available for capture fixtures.
class MockSettingsClient : public SettingsClient
{
    Q_OBJECT

public:
    explicit MockSettingsClient(QObject *parent = nullptr);

    bool isAvailable() const override { return true; }
    void refresh() override;
    void set(const QString &key, const QVariant &value) override;
};