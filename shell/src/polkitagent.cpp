// SPDX-License-Identifier: MIT
#include "polkitagent.h"

#include "polkitsession.h"

#include <QCoreApplication>
#include <QDBusArgument>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QFile>
#include <QLocale>

#include <pwd.h>

namespace dragonfruit {

// The `(sa{sv})` subject polkit's `RegisterAuthenticationAgent` takes. The
// attribute map is written as `a{sv}`; polkit expects `session-id` (`s`) for a
// unix-session subject and `pid` (`u`) + `start-time` (`t`) for unix-process.
struct PolkitSubject {
    QString kind;
    QVariantMap attributes;
};

QDBusArgument &operator<<(QDBusArgument &argument, const PolkitSubject &subject)
{
    argument.beginStructure();
    argument << subject.kind << subject.attributes;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, PolkitSubject &subject)
{
    argument.beginStructure();
    argument >> subject.kind >> subject.attributes;
    argument.endStructure();
    return argument;
}

// Parse a `a{ss}` details map.
static QVariantMap parseDetails(const QDBusArgument &argument)
{
    QVariantMap map;
    argument.beginMap();
    while (!argument.atEnd()) {
        argument.beginMapEntry();
        QString key;
        QString value;
        argument >> key >> value;
        argument.endMapEntry();
        map.insert(key, value);
    }
    argument.endMap();
    return map;
}

// Parse the `a(sa{sv})` identity list, returning one username per unix-user
// identity (empty for other kinds).
static QStringList parseIdentityUsernames(const QDBusArgument &argument, QVariantList *detailsOut)
{
    QStringList usernames;
    argument.beginArray();
    while (!argument.atEnd()) {
        argument.beginStructure();
        QString kind;
        QVariantMap attributes;
        argument >> kind >> attributes;
        argument.endStructure();
        if (kind == QLatin1String("unix-user")) {
            uint uid = 0;
            const QVariant uidValue = attributes.value(QStringLiteral("uid"));
            if (uidValue.isValid()) {
                bool ok = false;
                uid = uidValue.toUInt(&ok);
                if (!ok)
                    uid = 0;
            }
            const QString name = PolkitAgent::usernameForUid(uid);
            if (!name.isEmpty())
                usernames.append(name);
            if (detailsOut) {
                QVariantMap detail;
                detail.insert(QStringLiteral("key"), QStringLiteral("identity"));
                detail.insert(QStringLiteral("value"), name.isEmpty()
                                                             ? QString::number(uid)
                                                             : name);
                detailsOut->append(detail);
            }
        }
    }
    argument.endArray();
    return usernames;
}

} // namespace dragonfruit

// The agent object served at `kPolkitAgentPath`. A virtual object is used
// because polkit's `BeginAuthentication` carries `a(sa{sv})`, which has no
// built-in QtDBus C++ mapping; `handleMessage` parses the raw arguments.
class PolkitAgentObject : public QDBusVirtualObject
{
public:
    explicit PolkitAgentObject(PolkitAgent *agent)
        : m_agent(agent)
    {
    }

    QString introspect(const QString &path) const override
    {
        Q_UNUSED(path);
        return QStringLiteral(
            "<node><interface name='org.freedesktop.PolicyKit1.AuthenticationAgent'>"
            "<method name='BeginAuthentication'>"
            "<arg type='s' name='action_id' direction='in'/>"
            "<arg type='s' name='message' direction='in'/>"
            "<arg type='s' name='icon_name' direction='in'/>"
            "<arg type='a{ss}' name='details' direction='in'/>"
            "<arg type='s' name='cookie' direction='in'/>"
            "<arg type='a(sa{sv})' name='identities' direction='in'/>"
            "</method>"
            "<method name='CancelAuthentication'>"
            "<arg type='s' name='cookie' direction='in'/>"
            "</method>"
            "</interface></node>");
    }

    bool handleMessage(const QDBusMessage &message, const QDBusConnection &connection) override
    {
        if (message.interface() != QLatin1String(dragonfruit::kPolkitAgentInterface))
            return false;

        const QList<QVariant> args = message.arguments();
        if (message.member() == QLatin1String("BeginAuthentication")) {
            if (args.size() < 6)
                return false;
            const QString actionId = args.at(0).toString();
            const QString messageText = args.at(1).toString();
            const QString iconName = args.at(2).toString();
            QVariantMap rawDetails;
            if (args.at(3).canConvert<QDBusArgument>())
                rawDetails = dragonfruit::parseDetails(args.at(3).value<QDBusArgument>());
            const QString cookie = args.at(4).toString();
            QVariantList details;
            for (auto it = rawDetails.constBegin(); it != rawDetails.constEnd(); ++it) {
                QVariantMap detail;
                detail.insert(QStringLiteral("key"), it.key());
                detail.insert(QStringLiteral("value"), it.value());
                details.append(detail);
            }
            QStringList usernames;
            if (args.at(5).canConvert<QDBusArgument>())
                usernames = dragonfruit::parseIdentityUsernames(
                    args.at(5).value<QDBusArgument>(), &details);
            m_agent->beginAuthentication(message, connection, actionId, messageText, iconName,
                                         details, cookie, usernames);
            return true;
        }
        if (message.member() == QLatin1String("CancelAuthentication")) {
            const QString cookie = args.isEmpty() ? QString() : args.at(0).toString();
            m_agent->cancelAuthentication(cookie);
            connection.send(message.createReply());
            return true;
        }
        return false;
    }

private:
    PolkitAgent *m_agent = nullptr;
};

Q_DECLARE_METATYPE(dragonfruit::PolkitSubject)

PolkitAgent::PolkitAgent(QObject *parent)
    : QObject(parent)
{
}

PolkitAgent::~PolkitAgent()
{
    if (m_session) {
        m_session->cancel();
        m_session->deleteLater();
        m_session = nullptr;
    }
    if (m_registered) {
        QDBusConnection bus = QDBusConnection::systemBus();
        if (!m_connectionName.isEmpty())
            bus = QDBusConnection::connectToBus(m_busAddress, m_connectionName);
        QDBusMessage call = QDBusMessage::createMethodCall(
            QString::fromLatin1(dragonfruit::kPolkitAuthorityService),
            QString::fromLatin1(dragonfruit::kPolkitAuthorityPath),
            QString::fromLatin1(dragonfruit::kPolkitAuthorityInterface),
            QStringLiteral("UnregisterAuthenticationAgent"));
        call << m_subject << QString::fromLatin1(dragonfruit::kPolkitAgentPath);
        bus.call(call, QDBus::NoBlock);
    }
    if (m_object) {
        QDBusConnection bus = QDBusConnection::systemBus();
        if (!m_connectionName.isEmpty())
            bus = QDBusConnection::connectToBus(m_busAddress, m_connectionName);
        bus.unregisterObject(QString::fromLatin1(dragonfruit::kPolkitAgentPath));
        delete m_object;
        m_object = nullptr;
    }
}

void PolkitAgent::setBusAddress(const QString &address)
{
    m_busAddress = address;
}

void PolkitAgent::setServiceName(const QString &name)
{
    m_serviceName = name;
}

void PolkitAgent::setSubjectSessionId(const QString &sessionId)
{
    m_sessionId = sessionId;
}

void PolkitAgent::setSubjectProcess(qint64 pid)
{
    m_processPid = pid;
}

void PolkitAgent::setHelperPath(const QString &path)
{
    m_helperPath = path;
}

QString PolkitAgent::usernameForUid(uint uid)
{
    struct passwd *entry = ::getpwuid(static_cast<uid_t>(uid));
    if (!entry || !entry->pw_name)
        return QString();
    return QString::fromLocal8Bit(entry->pw_name);
}

QVariant PolkitAgent::buildSubject() const
{
    QString sessionId = m_sessionId;
    if (sessionId.isEmpty())
        sessionId = qEnvironmentVariable("DF_POLKIT_SUBJECT_SESSION");

    const QByteArray pidEnv = qgetenv("DF_POLKIT_SUBJECT_PID");
    qint64 pid = m_processPid;
    if (pid < 0 && !pidEnv.isEmpty())
        pid = pidEnv.toLongLong();

    // A process subject is an explicit override (tests, a dev session that
    // must not fight another agent); otherwise prefer the session.
    if (pid < 0 && sessionId.isEmpty())
        sessionId = qEnvironmentVariable("XDG_SESSION_ID");

    if (!sessionId.isEmpty()) {
        dragonfruit::PolkitSubject subject;
        subject.kind = QStringLiteral("unix-session");
        subject.attributes.insert(QStringLiteral("session-id"), sessionId);
        return QVariant::fromValue(subject);
    }

    if (pid <= 0)
        pid = QCoreApplication::applicationPid();
    quint64 startTime = 0;
    QFile stat(QStringLiteral("/proc/%1/stat").arg(pid));
    if (stat.open(QIODevice::ReadOnly)) {
        const QByteArray data = stat.readAll();
        const int close = data.lastIndexOf(')');
        if (close >= 0) {
            const QList<QByteArray> fields = data.mid(close + 1).trimmed().split(' ');
            // Field 22 (starttime) is index 19 after the comm field.
            if (fields.size() > 19)
                startTime = fields.at(19).toULongLong();
        }
    }
    dragonfruit::PolkitSubject subject;
    subject.kind = QStringLiteral("unix-process");
    subject.attributes.insert(QStringLiteral("pid"), static_cast<uint>(pid));
    subject.attributes.insert(QStringLiteral("start-time"), static_cast<qulonglong>(startTime));
    return QVariant::fromValue(subject);
}

bool PolkitAgent::start()
{
    if (m_started)
        return m_registered;
    m_started = true;

    qDBusRegisterMetaType<dragonfruit::PolkitSubject>();

    QDBusConnection bus = QDBusConnection::systemBus();
    if (!m_busAddress.isEmpty()) {
        m_connectionName = QStringLiteral("dragonfruit_polkit_agent");
        bus = QDBusConnection::connectToBus(m_busAddress, m_connectionName);
    }
    if (!bus.isConnected()) {
        m_registrationError = tr("No D-Bus connection is available for the polkit agent.");
        emit changed();
        return false;
    }
    if (!m_serviceName.isEmpty())
        bus.registerService(m_serviceName);

    m_object = new PolkitAgentObject(this);
    const bool objectOk = bus.registerVirtualObject(
        QString::fromLatin1(dragonfruit::kPolkitAgentPath), m_object, QDBusConnection::SubPath);
    if (!objectOk) {
        m_registrationError = tr("The polkit agent object could not be exported.");
        delete m_object;
        m_object = nullptr;
        emit changed();
        return false;
    }

    m_subject = buildSubject();
    m_locale = QLocale::system().name();
    QDBusMessage call = QDBusMessage::createMethodCall(
        QString::fromLatin1(dragonfruit::kPolkitAuthorityService),
        QString::fromLatin1(dragonfruit::kPolkitAuthorityPath),
        QString::fromLatin1(dragonfruit::kPolkitAuthorityInterface),
        QStringLiteral("RegisterAuthenticationAgent"));
    call << m_subject << m_locale << QString::fromLatin1(dragonfruit::kPolkitAgentPath);
    // Asynchronous so a same-thread fake authority (tests) can answer, and so
    // a busy or slow authority never blocks shell startup. `isRegistered()`
    // becomes true when the reply arrives; `changed()` announces it.
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, bus, watcher]() mutable {
                watcher->deleteLater();
                const QDBusPendingReply<> reply = *watcher;
                if (reply.isError()) {
                    m_registrationError = reply.error().message().isEmpty()
                            ? tr("The polkit authority refused the agent registration.")
                            : reply.error().message();
                    bus.unregisterObject(QString::fromLatin1(dragonfruit::kPolkitAgentPath));
                    delete m_object;
                    m_object = nullptr;
                    m_registered = false;
                } else {
                    m_registered = true;
                    m_registrationError.clear();
                }
                emit changed();
            });
    return true;
}

void PolkitAgent::presentLocal(const QString &actionId, const QString &message,
                               const QString &identityLabel, const QVariantList &details)
{
    resetRequest();
    m_localOnly = true;
    m_active = true;
    m_busy = false;
    m_actionId = actionId;
    m_message = message.isEmpty() ? tr("Authentication is required") : message;
    m_identityLabel = identityLabel;
    m_details = details;
    m_prompt = tr("Password:");
    m_promptEcho = false;
    emit started();
    emit changed();
}

void PolkitAgent::beginAuthentication(const QDBusMessage &dbusMessage,
                                      const QDBusConnection &connection, const QString &actionId,
                                      const QString &message, const QString &iconName,
                                      const QVariantList &details, const QString &cookie,
                                      const QStringList &usernames)
{
    if (usernames.isEmpty()) {
        // No identity we can drive through the host PAM stack: fail closed.
        connection.send(dbusMessage.createErrorReply(
            QStringLiteral("org.freedesktop.PolicyKit1.Error.Failed"),
            QStringLiteral("No supported identity")));
        return;
    }
    connection.send(dbusMessage.createReply());

    Request request;
    request.actionId = actionId;
    request.message = message;
    request.iconName = iconName;
    request.cookie = cookie;
    request.username = usernames.first();
    request.details = details;
    m_queue.append(request);
    if (!m_active)
        showNext();
}

void PolkitAgent::cancelAuthentication(const QString &cookie)
{
    if (m_active && cookie == m_cookie) {
        cancel();
        return;
    }
    for (int i = 0; i < m_queue.size(); ++i) {
        if (m_queue.at(i).cookie == cookie) {
            m_queue.removeAt(i);
            break;
        }
    }
}

void PolkitAgent::showNext()
{
    if (m_queue.isEmpty()) {
        m_active = false;
        emit changed();
        return;
    }
    const Request request = m_queue.takeFirst();
    m_localOnly = false;
    m_active = true;
    m_actionId = request.actionId;
    m_message = request.message.isEmpty() ? tr("Authentication is required") : request.message;
    m_iconName = request.iconName;
    m_identityLabel = request.username;
    m_details = request.details;
    m_cookie = request.cookie;
    m_prompt = tr("Password:");
    m_promptEcho = false;
    m_info.clear();
    m_errorText.clear();
    emit started();
    emit changed();

    m_session = new PolkitHelperSession(this);
    if (!m_helperPath.isEmpty())
        m_session->setHelperPath(m_helperPath);
    // Force the spawn path in tests that provide a fake helper but leave a
    // real socket in place.
    if (!m_helperPath.isEmpty())
        m_session->setSocketPath(QStringLiteral("/nonexistent"));
    connect(m_session, &PolkitHelperSession::prompt, this, &PolkitAgent::onPrompt);
    connect(m_session, &PolkitHelperSession::info, this, &PolkitAgent::onInfo);
    connect(m_session, &PolkitHelperSession::errorMessage, this, &PolkitAgent::onErrorMessage);
    connect(m_session, &PolkitHelperSession::completed, this, &PolkitAgent::onHelperCompleted);
    m_busy = true;
    if (!m_session->start(request.username, request.cookie)) {
        m_errorText = tr("The host authentication stack is unavailable.");
        onHelperCompleted(false);
    }
}

void PolkitAgent::onPrompt(const QString &text, bool echo)
{
    m_prompt = text.isEmpty() ? tr("Password:") : text;
    m_promptEcho = echo;
    emit changed();
}

void PolkitAgent::onInfo(const QString &text)
{
    m_info = text;
    emit changed();
}

void PolkitAgent::onErrorMessage(const QString &text)
{
    m_errorText = text;
    emit changed();
}

void PolkitAgent::onHelperCompleted(bool success)
{
    m_busy = false;
    if (m_session) {
        disconnect(m_session, nullptr, this, nullptr);
        m_session->deleteLater();
        m_session = nullptr;
    }
    // Clear the response-facing state but keep the request identity until the
    // `finished` signal has been delivered (the controller reads it).
    emit finished(success);
    resetRequest();
    if (!m_queue.isEmpty())
        showNext();
    else {
        m_active = false;
        emit changed();
    }
}

void PolkitAgent::respond(const QString &response)
{
    if (m_localOnly) {
        // The fixture has no helper; a typed answer simply closes it.
        m_localOnly = false;
        m_active = false;
        emit finished(!response.isEmpty());
        resetRequest();
        return;
    }
    if (!m_active || !m_session)
        return;
    m_errorText.clear();
    m_session->respond(response);
    emit changed();
}

void PolkitAgent::cancel()
{
    if (m_localOnly) {
        m_localOnly = false;
        m_active = false;
        emit finished(false);
        resetRequest();
        return;
    }
    if (!m_active || !m_session)
        return;
    m_session->cancel();
}

void PolkitAgent::resetRequest()
{
    m_actionId.clear();
    m_message.clear();
    m_iconName.clear();
    m_identityLabel.clear();
    m_details.clear();
    m_prompt.clear();
    m_promptEcho = false;
    m_info.clear();
    m_errorText.clear();
    m_cookie.clear();
    emit changed();
}