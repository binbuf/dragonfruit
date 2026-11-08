// SPDX-License-Identifier: MIT
// Shared client for the T-08 settings daemon
// (`org.dragonfruit.Settings1`, services/settingsd).
//
// This is not a shell-private header: the shell's Dock/Theme/compositor-policy
// controllers and the Settings app's QML `Settings` singleton (T-09.1b) both
// link this library (ADR 0036), so there is one D-Bus dialect and one
// schema-default table across the desktop.
//
// The abstract seam keeps the consumers free of D-Bus: the live
// `DbusSettingsClient` talks to the daemon and subscribes to its `Changed`
// signal (never polling); `MockSettingsClient` serves the schema defaults plus
// in-process writes for headless unit tests, the isolated-client test, and
// `DF_SETTINGS_FIXTURE` on the Settings app.
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
#include <QHash>
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
    // Apply a key through the one write path (T224): validate against the
    // schema, update the local store optimistically, then report success or
    // revert on failure. Returns false when the value was rejected before any
    // change (unknown key, wrong type, out of range, not an allowed enum).
    virtual bool set(const QString &key, const QVariant &value) = 0;
    // Reset every key to its schema default (T226). The live client calls
    // `org.dragonfruit.Settings1.Reset`; the mock applies the defaults in
    // process. Consumers converge through the usual `changed` signals.
    virtual void reset() = 0;

    QVariant value(const QString &key, const QVariant &fallback = {}) const;
    bool boolean(const QString &key, bool fallback) const;
    double real(const QString &key, double fallback) const;
    qlonglong integer(const QString &key, qlonglong fallback) const;
    QString string(const QString &key, const QString &fallback) const;
    QStringList stringList(const QString &key) const;
    QVariantMap values() const { return m_values; }

    // T224: the one validation entry point. Empty means valid; otherwise a
    // message naming the key and the problem (the same wording settingsd
    // uses). The value is coerced to the schema type first, so a QML double
    // for an integer key is judged as the integer it becomes.
    QString validate(const QString &key, const QVariant &value) const;
    // The last write error per key (`error` state only).
    QVariantMap writeErrors() const;
    // `idle`, `pending`, `deferred` (no daemon; kept in memory), or `error`.
    QVariantMap writeStates() const;
    QString writeError(const QString &key) const;
    QString writeState(const QString &key) const;

signals:
    void availableChanged(bool available);
    // A key's value really changed (local write or daemon signal). Emitted
    // only when the value differs from the current one.
    void changed(const QString &key, const QVariant &value);
    // A `refresh()` finished applying the daemon's snapshot (or, for a mock,
    // immediately).
    void refreshed();
    // T224: one write is in flight (the local value is already optimistic).
    void writeStarted(const QString &key, const QVariant &value);
    // T224: one write resolved. `ok` false means the value was rejected or the
    // owner failed; the local value has been reverted to `previous` by then.
    void writeFinished(const QString &key, const QVariant &value, bool ok,
                       const QString &message);
    void writeStateChanged(const QString &key);

protected:
    // Update one value and emit `changed` when it really differs.
    void applyValue(const QString &key, const QVariant &value);
    // Update every key in `values`, emitting `changed` per real difference.
    void applyValues(const QVariantMap &values);
    void setAvailable(bool available);

    // T224 common apply path. `beginWrite` validates, records the previous
    // value and a generation, marks the write pending, and applies the value
    // optimistically. `completeWrite` either clears the pending state or
    // reverts when the generation is still current; a stale reply from an
    // older write is ignored so a fast slider cannot clobber a newer value.
    bool beginWrite(const QString &key, const QVariant &value, quint64 *generation);
    void completeWrite(const QString &key, quint64 generation, bool ok,
                       const QString &message, bool deferred = false);
    // The daemon reported a value (an echo or an external change); clear any
    // pending error so the control stops claiming a failure the owner fixed.
    void noteOwnerValue(const QString &key, const QVariant &value);
    // A `GetAll` snapshot is authoritative (ADR 0032): drop deferred/error
    // states for keys that have no write in flight. A write made while the
    // daemon was away is in-memory for the session only, so the restart
    // resync reconciles it away instead of keeping an unpersisted change.
    void clearOwnerWriteStates();
    void clearWriteError(const QString &key);
    void setWriteState(const QString &key, const QString &state);

    QVariantMap m_values;
    bool m_available = false;

private:
    struct PendingWrite {
        QVariant previous;
        QVariant value;
        quint64 generation = 0;
    };
    QHash<QString, PendingWrite> m_writes;
    QHash<QString, quint64> m_generations;
    QHash<QString, QString> m_writeErrors;
    QHash<QString, QString> m_writeStates;
};

// The `org.dragonfruit.Settings1` schema defaults, mirrored from
// services/settingsd/src/schema.rs. Adding a key there means adding it here
// (and to the consumers that read it); the values are the frozen v1 contract.
QVariantMap settingsSchemaDefaults();

// T224: one key's declared type/default and its range/enum constraints, for
// rows that want to configure a bounded control from the schema. Empty when
// the key is unknown.
QVariantMap settingsSchemaInfo(const QString &key);

// The shipped original wallpaper `Default.jpg` resolved on this host, matching
// `services/wallpaperd/src/defaults.rs` (T-18.1b): `DF_DEFAULT_WALLPAPER`,
// then `$XDG_DATA_DIRS/dragonfruit/wallpapers/Default.jpg`, then the in-tree
// asset. Empty when the asset is nowhere (a normal state; the shell then falls
// back to the provider or the solid color). Both the shell's wallpaper
// forwarder and the Settings app's bridge share this one resolver.
QString shippedDefaultWallpaperPath();

// The live client over the user session bus.
class DbusSettingsClient : public SettingsClient
{
    Q_OBJECT

public:
    explicit DbusSettingsClient(QObject *parent = nullptr);

    bool isAvailable() const override;
    void refresh() override;
    bool set(const QString &key, const QVariant &value) override;
    void reset() override;

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

    bool isAvailable() const override { return m_available; }
    void refresh() override;
    bool set(const QString &key, const QVariant &value) override;
    void reset() override;

    // Test seam (`DF_SETTINGS_FIXTURE` only): make the next write of `key`
    // apply optimistically and then fail on the next event-loop turn, so a
    // QML test can observe both the optimistic value and the revert. A no-op
    // on the live client.
    void failNextWrite(const QString &key, const QString &message);
    // Test seam (`DF_SETTINGS_FIXTURE` only): pretend the owner disappeared or
    // returned, so a pane's while-open absence flip is observable without a
    // bus. A no-op on the live client.
    void setAvailableFixture(bool available);

private:
    QHash<QString, QString> m_failures;
};
