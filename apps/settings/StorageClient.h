// SPDX-License-Identifier: MIT
// The Settings app's storage seam (T-15.2b).
//
// The Storage pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and writes here. The live `DbusStorageClient`
// talks to the bridge host's `org.dragonfruit.SystemStatus1.Storage` interface
// (the same adapter the Control Center tile reads), and `MockStorageClient`
// serves a deterministic fixture for the headless pane tests
// (`DF_STORAGE_FIXTURE`). Absence is a normal state: `available()` is false and
// the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class StorageClient : public QObject
{
    Q_OBJECT

public:
    explicit StorageClient(QObject *parent = nullptr) : QObject(parent) {}
    ~StorageClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `storage` view (`{state, present, label, mountedCount,
    // volumes, drives}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // The three explicit writes; the host re-reads and pushes the new view.
    virtual void mount(const QString &volumePath) = 0;
    virtual void unmount(const QString &volumePath) = 0;
    virtual void eject(const QString &drivePath) = 0;

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusStorageClient : public StorageClient
{
    Q_OBJECT

public:
    explicit DbusStorageClient(QObject *parent = nullptr);
    ~DbusStorageClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void mount(const QString &volumePath) override;
    void unmount(const QString &volumePath) override;
    void eject(const QString &drivePath) override;

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

// The fixture client used by `DF_STORAGE_FIXTURE`: one removable USB drive
// with a "Photos" volume that mount/unmount toggles in place and eject removes.
// Writes mutate the fixture and re-emit, so a pane control's round-trip is
// observable with no bus.
class MockStorageClient : public StorageClient
{
    Q_OBJECT

public:
    explicit MockStorageClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void mount(const QString &volumePath) override;
    void unmount(const QString &volumePath) override;
    void eject(const QString &drivePath) override;

private:
    void rebuild();
    bool m_drivePresent = true;
    bool m_volumeMounted = false;
    QVariantMap m_view;
};