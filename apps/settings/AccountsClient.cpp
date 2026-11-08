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
                refreshBiometrics();
                refreshOnlineAccounts();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
                if (!m_biometricsView.isEmpty()) {
                    m_biometricsView.clear();
                    emit biometricsChanged(m_biometricsView);
                }
                if (!m_onlineView.isEmpty()) {
                    m_onlineView.clear();
                    emit onlineAccountsChanged(m_onlineView);
                }
            });
    refresh();
    refreshBiometrics();
    refreshOnlineAccounts();
}

DbusAccountsClient::~DbusAccountsClient() = default;

bool DbusAccountsClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

bool DbusAccountsClient::biometricsAvailable() const
{
    return m_available && m_biometricsView.value(QStringLiteral("state")).toString()
        != QStringLiteral("unavailable");
}

bool DbusAccountsClient::onlineAccountsAvailable() const
{
    return m_available && m_onlineView.value(QStringLiteral("state")).toString()
        == QStringLiteral("available");
}

void DbusAccountsClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusAccountsClient::refreshBiometrics()
{
    call(QStringLiteral("RefreshBiometrics"), {});
}

void DbusAccountsClient::refreshOnlineAccounts()
{
    call(QStringLiteral("RefreshOnlineAccounts"), {});
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

void DbusAccountsClient::startFingerprintEnrollment(const QString &finger)
{
    call(QStringLiteral("StartFingerprintEnrollment"), {finger});
}

void DbusAccountsClient::cancelFingerprintEnrollment()
{
    call(QStringLiteral("CancelFingerprintEnrollment"), {});
}

void DbusAccountsClient::deleteFingerprint(const QString &finger)
{
    call(QStringLiteral("DeleteFingerprint"), {finger});
}

void DbusAccountsClient::changePassword(const QString &password)
{
    call(QStringLiteral("ChangePassword"), {password});
}

void DbusAccountsClient::addOnlineAccount(const QString &provider, const QString &identity)
{
    call(QStringLiteral("AddOnlineAccount"), {provider, identity});
}

void DbusAccountsClient::removeOnlineAccount(const QString &id)
{
    call(QStringLiteral("RemoveOnlineAccount"), {id});
}

void DbusAccountsClient::setOnlineAccountSync(const QString &id, const QString &capability,
                                              bool enabled)
{
    call(QStringLiteral("SetOnlineAccountSync"), {id, capability, enabled});
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
    const bool isBiometricsState = method == QStringLiteral("BiometricsState")
        || method == QStringLiteral("RefreshBiometrics");
    const bool isOnlineState = method == QStringLiteral("OnlineAccountsState")
        || method == QStringLiteral("RefreshOnlineAccounts");
    const bool isBiometricsWrite = method == QStringLiteral("StartFingerprintEnrollment")
        || method == QStringLiteral("CancelFingerprintEnrollment")
        || method == QStringLiteral("DeleteFingerprint")
        || method == QStringLiteral("ChangePassword");
    const bool isOnlineWrite = method == QStringLiteral("AddOnlineAccount")
        || method == QStringLiteral("RemoveOnlineAccount")
        || method == QStringLiteral("SetOnlineAccountSync");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, isBiometricsState, isOnlineState, isBiometricsWrite,
             isOnlineWrite, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else if (isBiometricsState)
                    applyBiometricsReply(reply.value().toUtf8());
                else if (isOnlineState)
                    applyOnlineReply(reply.value().toUtf8());
                else {
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    if (isBiometricsWrite)
                        refreshBiometrics();
                    else if (isOnlineWrite)
                        refreshOnlineAccounts();
                    else
                        refresh();
                }
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

void DbusAccountsClient::applyBiometricsReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_biometricsView)
        return;
    m_biometricsView = view;
    emit biometricsChanged(m_biometricsView);
}

void DbusAccountsClient::applyOnlineReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_onlineView)
        return;
    m_onlineView = view;
    emit onlineAccountsChanged(m_onlineView);
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

    // T228: the biometrics and online-accounts halves are independent
    // fixtures, selected by their own environment variables so the users
    // suite keeps its deterministic two-user stack.
    m_biometricsPresent = qEnvironmentVariableIsSet("DF_BIOMETRICS_FIXTURE");
    m_fingers = { QStringLiteral("right-index-finger") };
    m_pamAvailable = true;
    m_pamUnlock = true;
    m_pamAuth = true;
    m_passwordAvailable = true;

    m_onlinePresent = qEnvironmentVariableIsSet("DF_ONLINE_ACCOUNTS_FIXTURE");
    m_onlineAccounts = {
        { QStringLiteral("account-1"), QStringLiteral("google"), QStringLiteral("Google"),
          QStringLiteral("dan@example.com"),
          { { QStringLiteral("mail"), true },
            { QStringLiteral("calendar"), true },
            { QStringLiteral("contacts"), false } } },
    };

    rebuild();
    rebuildBiometrics();
    rebuildOnlineAccounts();
}

void MockAccountsClient::refresh()
{
    rebuild();
    rebuildBiometrics();
    rebuildOnlineAccounts();
}

void MockAccountsClient::refreshBiometrics()
{
    rebuildBiometrics();
}

void MockAccountsClient::refreshOnlineAccounts()
{
    rebuildOnlineAccounts();
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

void MockAccountsClient::startFingerprintEnrollment(const QString &finger)
{
    if (!m_biometricsPresent || finger.isEmpty())
        return;
    if (!m_fingers.contains(finger))
        m_fingers.append(finger);
    rebuildBiometrics();
}

void MockAccountsClient::cancelFingerprintEnrollment()
{
    if (!m_biometricsPresent)
        return;
    rebuildBiometrics();
}

void MockAccountsClient::deleteFingerprint(const QString &finger)
{
    if (!m_biometricsPresent)
        return;
    m_fingers.removeAll(finger);
    rebuildBiometrics();
}

void MockAccountsClient::changePassword(const QString &password)
{
    if (!m_biometricsPresent || password.isEmpty())
        return;
    // The fixture records only that the write happened; the password is not
    // stored anywhere, matching the live AccountsService path.
    rebuildBiometrics();
}

void MockAccountsClient::addOnlineAccount(const QString &provider, const QString &identity)
{
    if (!m_onlinePresent || provider.isEmpty() || identity.isEmpty())
        return;
    OnlineAccount account;
    account.id = QStringLiteral("account-%1").arg(m_onlineAccounts.size() + 1);
    account.provider = provider;
    account.providerName = provider == QStringLiteral("google")
        ? QStringLiteral("Google") : QStringLiteral("Nextcloud");
    account.identity = identity;
    account.capabilities = { { QStringLiteral("mail"), true } };
    m_onlineAccounts.append(account);
    rebuildOnlineAccounts();
}

void MockAccountsClient::removeOnlineAccount(const QString &id)
{
    if (!m_onlinePresent)
        return;
    for (int i = 0; i < m_onlineAccounts.size(); ++i)
        if (m_onlineAccounts[i].id == id)
            m_onlineAccounts.removeAt(i);
    rebuildOnlineAccounts();
}

void MockAccountsClient::setOnlineAccountSync(const QString &id, const QString &capability,
                                              bool enabled)
{
    if (!m_onlinePresent)
        return;
    for (OnlineAccount &account : m_onlineAccounts) {
        if (account.id != id)
            continue;
        for (QPair<QString, bool> &entry : account.capabilities)
            if (entry.first == capability)
                entry.second = enabled;
    }
    rebuildOnlineAccounts();
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
    view.insert(QStringLiteral("adminGroup"),
                m_groupsAvailable ? QStringLiteral("wheel") : QString());
    view.insert(QStringLiteral("adminGroupAvailable"), m_groupsAvailable);
    view.insert(QStringLiteral("directoryAvailable"), true);
    QVariantMap directory;
    directory.insert(QStringLiteral("domains"),
                     QStringList{QStringLiteral("example.com")});
    directory.insert(QStringLiteral("provider"), QStringLiteral("sssd"));
    directory.insert(QStringLiteral("joined"), true);
    directory.insert(QStringLiteral("status"), QStringLiteral("Joined: example.com"));
    directory.insert(QStringLiteral("note"), QString());
    view.insert(QStringLiteral("directory"), directory);
    view.insert(QStringLiteral("users"), userArray.toVariantList());
    view.insert(QStringLiteral("groups"), groupArray.toVariantList());
    m_view = view;
    emit changed(m_view);
}

void MockAccountsClient::rebuildBiometrics()
{
    if (!m_biometricsPresent) {
        m_biometricsView.clear();
        emit biometricsChanged(m_biometricsView);
        return;
    }
    QJsonArray fingers;
    for (const QString &finger : m_fingers) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), finger);
        QString label = finger;
        label.replace(QLatin1Char('-'), QLatin1Char(' '));
        if (!label.isEmpty())
            label[0] = label[0].toUpper();
        entry.insert(QStringLiteral("label"), label);
        fingers.append(entry);
    }
    QJsonObject device;
    device.insert(QStringLiteral("path"), QStringLiteral("/net/reactivated/Fprint/Device/0"));
    device.insert(QStringLiteral("name"), QStringLiteral("Goodix Fingerprint Sensor"));
    device.insert(QStringLiteral("scanType"), QStringLiteral("press"));
    device.insert(QStringLiteral("enrolled"), fingers);
    device.insert(QStringLiteral("enrolledCount"), m_fingers.size());
    device.insert(QStringLiteral("note"), QString());
    QJsonObject pam;
    pam.insert(QStringLiteral("available"), m_pamAvailable);
    pam.insert(QStringLiteral("unlock"), m_pamUnlock);
    pam.insert(QStringLiteral("auth"), m_pamAuth);
    pam.insert(QStringLiteral("files"),
               QJsonArray::fromStringList({QStringLiteral("system-auth"),
                                           QStringLiteral("fingerprint-auth")}));
    QJsonObject password;
    password.insert(QStringLiteral("available"), m_passwordAvailable);
    password.insert(QStringLiteral("method"), QStringLiteral("accountsservice"));
    password.insert(QStringLiteral("note"),
                    QStringLiteral("The password is changed through the account "
                                   "service after authorization."));

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("biometrics"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("fingerprint"));
    view.insert(QStringLiteral("label"),
                m_fingers.size() == 0 ? QStringLiteral("No Fingerprints")
                : m_fingers.size() == 1 ? QStringLiteral("1 Fingerprint")
                                        : QStringLiteral("%1 Fingerprints").arg(m_fingers.size()));
    view.insert(QStringLiteral("user"), QStringLiteral("dan"));
    view.insert(QStringLiteral("uid"), 1000);
    view.insert(QStringLiteral("fprintdAvailable"), true);
    view.insert(QStringLiteral("device"), device.toVariantMap());
    view.insert(QStringLiteral("pam"), pam.toVariantMap());
    view.insert(QStringLiteral("password"), password.toVariantMap());
    view.insert(QStringLiteral("enrolling"), QVariant());
    m_biometricsView = view;
    emit biometricsChanged(m_biometricsView);
}

void MockAccountsClient::rebuildOnlineAccounts()
{
    if (!m_onlinePresent) {
        m_onlineView.clear();
        emit onlineAccountsChanged(m_onlineView);
        return;
    }
    QJsonArray providers;
    const auto provider = [&providers](const QString &id, const QString &name) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("icon"), id);
        entry.insert(QStringLiteral("supported"), true);
        providers.append(entry);
    };
    provider(QStringLiteral("google"), QStringLiteral("Google"));
    provider(QStringLiteral("nextcloud"), QStringLiteral("Nextcloud"));

    QJsonArray accounts;
    for (const OnlineAccount &account : m_onlineAccounts) {
        QJsonArray capabilities;
        for (const QPair<QString, bool> &capability : account.capabilities) {
            QJsonObject entry;
            entry.insert(QStringLiteral("id"), capability.first);
            QString label = capability.first;
            if (!label.isEmpty())
                label[0] = label[0].toUpper();
            entry.insert(QStringLiteral("label"), label);
            entry.insert(QStringLiteral("enabled"), capability.second);
            capabilities.append(entry);
        }
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), account.id);
        entry.insert(QStringLiteral("path"),
                     QStringLiteral("/org/gnome/OnlineAccounts/Accounts/account_1")); // df-allow-desktop-name: GOA object path
        entry.insert(QStringLiteral("provider"), account.provider);
        entry.insert(QStringLiteral("providerName"), account.providerName);
        entry.insert(QStringLiteral("identity"), account.identity);
        entry.insert(QStringLiteral("presentationIdentity"), account.identity);
        entry.insert(QStringLiteral("locked"), false);
        entry.insert(QStringLiteral("attentionNeeded"), false);
        entry.insert(QStringLiteral("capabilities"), capabilities);
        accounts.append(entry);
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("online-accounts"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("cloud"));
    view.insert(QStringLiteral("label"),
                m_onlineAccounts.size() == 0 ? QStringLiteral("No Accounts")
                : m_onlineAccounts.size() == 1 ? QStringLiteral("1 Account")
                                                : QStringLiteral("%1 Accounts")
                                                      .arg(m_onlineAccounts.size()));
    view.insert(QStringLiteral("present"), !m_onlineAccounts.isEmpty());
    view.insert(QStringLiteral("providerCount"), providers.size());
    view.insert(QStringLiteral("accountCount"), accounts.size());
    view.insert(QStringLiteral("providers"), providers.toVariantList());
    view.insert(QStringLiteral("accounts"), accounts.toVariantList());
    m_onlineView = view;
    emit onlineAccountsChanged(m_onlineView);
}
