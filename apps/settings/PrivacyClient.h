// SPDX-License-Identifier: MIT
// The Settings app's Privacy and Security seam (T-15.13b).
//
// The Privacy & Security pane never touches D-Bus: it binds the `Settings`
// singleton, and the bridge forwards reads and writes here. The live
// `DbusPrivacyClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Privacy` interface (the same adapter the
// Control Center tile reads), and `MockPrivacyClient` serves a deterministic
// fixture for the headless pane tests (`DF_PRIVACY_FIXTURE`). Absence is a
// normal state: `available()` is false and the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

class QDBusServiceWatcher;

class PrivacyClient : public QObject
{
    Q_OBJECT

public:
    explicit PrivacyClient(QObject *parent = nullptr) : QObject(parent) {}
    ~PrivacyClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `privacy` view (`{state, glyph, label, present, appCount,
    // grantedCount, deniedCount, categoryCount, categories}`); empty when
    // absent. Each category is `{id, label, summary, appCount, grantedCount,
    // resources}`; each resource `{id, appCount, apps}`; each app `{app, state,
    // stateLabel, permissions}`.
    virtual QVariantMap view() const = 0;
    // Re-read the permission store once (the pane calls this on open).
    virtual void refresh() = 0;
    // Store the tristate (`allowed`/`denied`/`ask`) for one application on one
    // resource, or remove its record. The host re-reads and pushes the new
    // view.
    virtual void setPermission(const QString &table, const QString &resourceId,
                               const QString &app, const QString &permission) = 0;
    virtual void deletePermission(const QString &table, const QString &resourceId,
                                  const QString &app) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusPrivacyClient : public PrivacyClient
{
    Q_OBJECT

public:
    explicit DbusPrivacyClient(QObject *parent = nullptr);
    ~DbusPrivacyClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setPermission(const QString &table, const QString &resourceId,
                       const QString &app, const QString &permission) override;
    void deletePermission(const QString &table, const QString &resourceId,
                          const QString &app) override;

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

// The fixture client used by `DF_PRIVACY_FIXTURE`: a Dragonfruit workstation
// whose portal PermissionStore records four categories and five application
// permissions. The writes mutate the simulated store in place and re-emit, so
// every pane control's round-trip is observable with no bus.
class MockPrivacyClient : public PrivacyClient
{
    Q_OBJECT

public:
    explicit MockPrivacyClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setPermission(const QString &table, const QString &resourceId,
                       const QString &app, const QString &permission) override;
    void deletePermission(const QString &table, const QString &resourceId,
                          const QString &app) override;
    void resetForTest() override;

private:
    void rebuild();
    // The simulated PermissionStore: tables of resources of app permissions,
    // the T-15.13a `TableData`/`ResourceData`/`AppPermissionData` shape as Qt
    // values.
    struct AppPermission {
        QString app;
        QStringList permissions;
    };
    struct Resource {
        QString id;
        QList<AppPermission> apps;
    };
    struct Table {
        QString table;
        QList<Resource> resources;
    };
    QList<Table> m_tables;
    QVariantMap m_view;
};