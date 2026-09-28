// SPDX-License-Identifier: MIT
// The Settings app's Users and Groups seam (T-15.11b).
//
// The Users & Groups pane never touches D-Bus: it binds the `Settings`
// singleton, and the bridge forwards reads and writes here. The live
// `DbusAccountsClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Accounts` interface (the same adapter the
// Control Center tile reads), and `MockAccountsClient` serves a deterministic
// fixture for the headless pane tests (`DF_ACCOUNTS_FIXTURE`). Absence is a
// normal state: `available()` is false and the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

class QDBusServiceWatcher;

class AccountsClient : public QObject
{
    Q_OBJECT

public:
    explicit AccountsClient(QObject *parent = nullptr) : QObject(parent) {}
    ~AccountsClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `accounts` view (`{state, glyph, label, present, humanCount,
    // adminCount, lockedCount, groupsAvailable, automaticLogin, users, groups,
    // ...}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host stack once (the pane calls this on open).
    virtual void refresh() = 0;
    // The eight explicit writes; the host re-reads and pushes the new view.
    virtual void createUser(const QString &userName, const QString &realName,
                            const QString &accountType) = 0;
    virtual void deleteUser(int uid) = 0;
    virtual void setAccountType(int uid, const QString &accountType) = 0;
    virtual void setLocked(int uid, bool locked) = 0;
    virtual void setAutomaticLogin(int uid, bool automaticLogin) = 0;
    virtual void createGroup(const QString &name) = 0;
    virtual void deleteGroup(const QString &name) = 0;
    virtual void setGroupMembers(const QString &name, const QStringList &members) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusAccountsClient : public AccountsClient
{
    Q_OBJECT

public:
    explicit DbusAccountsClient(QObject *parent = nullptr);
    ~DbusAccountsClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void createUser(const QString &userName, const QString &realName,
                    const QString &accountType) override;
    void deleteUser(int uid) override;
    void setAccountType(int uid, const QString &accountType) override;
    void setLocked(int uid, bool locked) override;
    void setAutomaticLogin(int uid, bool automaticLogin) override;
    void createGroup(const QString &name) override;
    void deleteGroup(const QString &name) override;
    void setGroupMembers(const QString &name, const QStringList &members) override;

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

// The fixture client used by `DF_ACCOUNTS_FIXTURE`: a Dragonfruit workstation
// with two human users (an admin set for automatic login, a locked standard
// user) and one user group. The writes mutate the simulated stack in place and
// re-emit, so every pane control's round-trip is observable with no bus.
class MockAccountsClient : public AccountsClient
{
    Q_OBJECT

public:
    explicit MockAccountsClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void createUser(const QString &userName, const QString &realName,
                    const QString &accountType) override;
    void deleteUser(int uid) override;
    void setAccountType(int uid, const QString &accountType) override;
    void setLocked(int uid, bool locked) override;
    void setAutomaticLogin(int uid, bool automaticLogin) override;
    void createGroup(const QString &name) override;
    void deleteGroup(const QString &name) override;
    void setGroupMembers(const QString &name, const QStringList &members) override;
    void resetForTest() override;

private:
    void rebuild();
    // The simulated stack: the T-15.11a `AccountsData` shape as Qt values.
    struct User {
        int uid = 0;
        QString userName;
        QString realName;
        QString accountType = QStringLiteral("standard");
        bool locked = false;
        bool automaticLogin = false;
        bool system = false;
    };
    struct Group {
        QString name;
        int gid = 0;
        QStringList members;
        bool system = false;
    };
    QList<User> m_users;
    QList<Group> m_groups;
    bool m_groupsAvailable = true;
    int m_nextUid = 1002;
    QVariantMap m_view;
};