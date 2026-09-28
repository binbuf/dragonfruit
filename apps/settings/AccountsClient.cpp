// SPDX-License-Identifier: MIT
#include "AccountsClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

#include <algorithm>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Accounts");

} // namespace

// --- DbusAccountsClient ----------------------------------------------------

DbusAccountsClient::DbusAccountsClient(QObject *parent)
    : AccountsClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_available = available();
    m_watcher = new QDBusServiceWatcher(
        m_service, bus, QDBusServiceWatcher::WatchForRegistration
                          | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) {
                m_available = true;
                emit availableChanged(true);
                refresh();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
            });
    refresh();
}

DbusAccountsClient::~DbusAccountsClient() = default;

bool DbusAccountsClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusAccountsClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusAccountsClient::createUser(const QString &userName, const QString &realName,
                                    const QString &accountType)
{
    call(QStringLiteral("CreateUser"), {userName, realName, accountType});
}

void DbusAccountsClient::deleteUser(int uid)
{
    call(QStringLiteral("DeleteUser"), {uid});
}

void DbusAccountsClient::setAccountType(int uid, const QString &accountType)
{
    call(QStringLiteral("SetAccountType"), {uid, accountType});
}

void DbusAccountsClient::setLocked(int uid, bool locked)
{
    call(QStringLiteral("SetLocked"), {uid, locked});
}

void DbusAccountsClient::setAutomaticLogin(int uid, bool automaticLogin)
{
    call(QStringLiteral("SetAutomaticLogin"), {uid, automaticLogin});
}

void DbusAccountsClient::createGroup(const QString &name)
{
    call(QStringLiteral("CreateGroup"), {name});
}

void DbusAccountsClient::deleteGroup(const QString &name)
{
    call(QStringLiteral("DeleteGroup"), {name});
}

void DbusAccountsClient::setGroupMembers(const QString &name, const QStringList &members)
{
    call(QStringLiteral("SetGroupMembers"), {name, members});
}

void DbusAccountsClient::call(const QString &method, const QVariantList &arguments)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
    const bool isState = method == QStringLiteral("State");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    refresh();
            });
}

void DbusAccountsClient::applyReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_view)
        return;
    m_view = view;
    emit changed(m_view);
}

// --- MockAccountsClient ----------------------------------------------------

MockAccountsClient::MockAccountsClient(QObject *parent)
    : AccountsClient(parent)
{
    resetForTest();
}

void MockAccountsClient::resetForTest()
{
    m_users = {
        { 1000, QStringLiteral("dan"), QStringLiteral("Dan Doe"),
          QStringLiteral("administrator"), false, true, false },
        { 1001, QStringLiteral("sam"), QStringLiteral("Sam Smith"),
          QStringLiteral("standard"), true, false, false },
        { 0, QStringLiteral("root"), QStringLiteral("root"),
          QStringLiteral("administrator"), false, false, true },
    };
    m_groups = {
        { QStringLiteral("wheel"), 10, {QStringLiteral("dan")}, false },
        { QStringLiteral("users"), 100, {QStringLiteral("dan"), QStringLiteral("sam")},
          true },
    };
    m_groupsAvailable = true;
    m_nextUid = 1002;
    rebuild();
}

void MockAccountsClient::refresh()
{
    rebuild();
}

void MockAccountsClient::createUser(const QString &userName, const QString &realName,
                                    const QString &accountType)
{
    if (userName.trimmed().isEmpty())
        return;
    User user;
    user.uid = m_nextUid++;
    user.userName = userName.trimmed();
    user.realName = realName.trimmed();
    user.accountType = accountType == QStringLiteral("administrator")
        ? QStringLiteral("administrator") : QStringLiteral("standard");
    m_users.append(user);
    rebuild();
}

void MockAccountsClient::deleteUser(int uid)
{
    QString removedName;
    for (int i = 0; i < m_users.size(); ++i) {
        if (m_users[i].uid != uid)
            continue;
        removedName = m_users[i].userName;
        m_users.removeAt(i);
        break;
    }
    if (removedName.isEmpty())
        return;
    // Membership is by name; drop the deleted user from every group.
    for (Group &group : m_groups)
        group.members.removeAll(removedName);
    rebuild();
}

void MockAccountsClient::setAccountType(int uid, const QString &accountType)
{
    for (User &user : m_users) {
        if (user.uid != uid)
            continue;
        user.accountType = accountType == QStringLiteral("administrator")
            ? QStringLiteral("administrator") : QStringLiteral("standard");
    }
    rebuild();
}

void MockAccountsClient::setLocked(int uid, bool locked)
{
    for (User &user : m_users)
        if (user.uid == uid)
            user.locked = locked;
    rebuild();
}

void MockAccountsClient::setAutomaticLogin(int uid, bool automaticLogin)
{
    for (User &user : m_users) {
        if (user.uid == uid)
            user.automaticLogin = automaticLogin;
        else if (automaticLogin)
            // AccountsService keeps one automatic login user.
            user.automaticLogin = false;
    }
    rebuild();
}

void MockAccountsClient::createGroup(const QString &name)
{
    if (name.trimmed().isEmpty() || !m_groupsAvailable)
        return;
    Group group;
    group.name = name.trimmed();
    group.gid = 1000 + m_groups.size();
    m_groups.append(group);
    rebuild();
}

void MockAccountsClient::deleteGroup(const QString &name)
{
    if (!m_groupsAvailable)
        return;
    for (int i = 0; i < m_groups.size(); ++i)
        if (m_groups[i].name == name)
            m_groups.removeAt(i);
    rebuild();
}

void MockAccountsClient::setGroupMembers(const QString &name, const QStringList &members)
{
    if (!m_groupsAvailable)
        return;
    for (Group &group : m_groups)
        if (group.name == name)
            group.members = members;
    rebuild();
}

void MockAccountsClient::rebuild()
{
    QList<User> users = m_users;
    std::stable_sort(users.begin(), users.end(), [](const User &a, const User &b) {
        if (a.system != b.system)
            return !a.system;
        if (a.uid != b.uid)
            return a.uid < b.uid;
        return a.userName < b.userName;
    });
    QList<Group> groups = m_groups;
    std::stable_sort(groups.begin(), groups.end(), [](const Group &a, const Group &b) {
        if (a.system != b.system)
            return !a.system;
        return a.name < b.name;
    });

    int humanCount = 0;
    int adminCount = 0;
    int lockedCount = 0;
    for (const User &user : users) {
        if (user.system)
            continue;
        ++humanCount;
        if (user.accountType == QStringLiteral("administrator"))
            ++adminCount;
        if (user.locked)
            ++lockedCount;
    }

    QJsonArray userArray;
    for (const User &user : users) {
        QJsonObject entry;
        entry.insert(QStringLiteral("uid"), user.uid);
        entry.insert(QStringLiteral("userName"), user.userName);
        entry.insert(QStringLiteral("realName"), user.realName);
        entry.insert(QStringLiteral("displayName"),
                     user.realName.isEmpty() ? user.userName : user.realName);
        const QString source = user.realName.isEmpty() ? user.userName : user.realName;
        QString initial = QStringLiteral("?");
        for (const QChar ch : source)
            if (ch.isLetterOrNumber()) {
                initial = QString(ch).toUpper();
                break;
            }
        entry.insert(QStringLiteral("initial"), initial);
        entry.insert(QStringLiteral("accountType"), user.accountType);
        entry.insert(QStringLiteral("accountTypeLabel"),
                     user.accountType == QStringLiteral("administrator")
                         ? QStringLiteral("Admin") : QStringLiteral("Standard"));
        entry.insert(QStringLiteral("locked"), user.locked);
        entry.insert(QStringLiteral("system"), user.system);
        entry.insert(QStringLiteral("automaticLogin"), user.automaticLogin);
        entry.insert(QStringLiteral("hasAvatar"), false);
        userArray.append(entry);
    }

    QJsonArray groupArray;
    for (const Group &group : groups) {
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), group.name);
        entry.insert(QStringLiteral("gid"), group.gid);
        entry.insert(QStringLiteral("memberCount"), group.members.size());
        entry.insert(QStringLiteral("members"), QJsonArray::fromStringList(group.members));
        entry.insert(QStringLiteral("system"), group.system);
        groupArray.append(entry);
    }

    QString automaticLogin;
    QString automaticLoginUser;
    int automaticLoginUid = 0;
    for (const User &user : users) {
        if (!user.automaticLogin)
            continue;
        automaticLogin = user.realName.isEmpty() ? user.userName : user.realName;
        automaticLoginUser = user.userName;
        automaticLoginUid = user.uid;
        break;
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("accounts"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("users"));
    view.insert(QStringLiteral("label"),
                humanCount == 0 ? QStringLiteral("No Users")
                : humanCount == 1 ? QStringLiteral("1 User")
                                  : QStringLiteral("%1 Users").arg(humanCount));
    view.insert(QStringLiteral("present"), !users.isEmpty());
    view.insert(QStringLiteral("userCount"), users.size());
    view.insert(QStringLiteral("humanCount"), humanCount);
    view.insert(QStringLiteral("systemCount"), users.size() - humanCount);
    view.insert(QStringLiteral("adminCount"), adminCount);
    view.insert(QStringLiteral("lockedCount"), lockedCount);
    view.insert(QStringLiteral("groupsAvailable"), m_groupsAvailable);
    view.insert(QStringLiteral("groupCount"), groups.size());
    view.insert(QStringLiteral("automaticLogin"), automaticLogin);
    view.insert(QStringLiteral("automaticLoginUser"), automaticLoginUser);
    view.insert(QStringLiteral("automaticLoginUid"), automaticLoginUid);
    view.insert(QStringLiteral("users"), userArray.toVariantList());
    view.insert(QStringLiteral("groups"), groupArray.toVariantList());
    m_view = view;
    emit changed(m_view);
}