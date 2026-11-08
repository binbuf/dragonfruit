// SPDX-License-Identifier: MIT
#include "SettingsAuthorizer.h"

#include <QCoreApplication>
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QFile>
#include <QMap>
#include <QTimer>

namespace {

const QString kAuthorityService = QStringLiteral("org.freedesktop.PolicyKit1");
const QString kAuthorityPath = QStringLiteral("/org/freedesktop/PolicyKit1/Authority");
const QString kAuthorityInterface = QStringLiteral("org.freedesktop.PolicyKit1.Authority");

// The `(sa{sv})` subject CheckAuthorization takes. Same shape the shell's
// polkit agent registers with; duplicated here so the Settings app stays a
// consumer of the authority rather than linking the shell's dockcore.
struct AuthorizerSubject {
    QString kind;
    QVariantMap attributes;
};

QDBusArgument &operator<<(QDBusArgument &argument, const AuthorizerSubject &subject)
{
    argument.beginStructure();
    argument << subject.kind << subject.attributes;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, AuthorizerSubject &subject)
{
    argument.beginStructure();
    argument >> subject.kind >> subject.attributes;
    argument.endStructure();
    return argument;
}

// `a{ss}` has no built-in Qt container mapping (QVariantMap is `a{sv}`), so
// the details map gets a tiny explicit codec.
struct StringMap {
    QMap<QString, QString> values;
};

QDBusArgument &operator<<(QDBusArgument &argument, const StringMap &map)
{
    argument.beginMap(QMetaType::QString, QMetaType::QString);
    for (auto it = map.values.constBegin(); it != map.values.constEnd(); ++it) {
        argument.beginMapEntry();
        argument << it.key() << it.value();
        argument.endMapEntry();
    }
    argument.endMap();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, StringMap &map)
{
    argument.beginMap();
    while (!argument.atEnd()) {
        argument.beginMapEntry();
        QString key;
        QString value;
        argument >> key >> value;
        argument.endMapEntry();
        map.values.insert(key, value);
    }
    argument.endMap();
    return argument;
}

QString sessionId()
{
    QString id = qEnvironmentVariable("DF_SETTINGS_POLKIT_SUBJECT_SESSION");
    if (id.isEmpty())
        id = qEnvironmentVariable("XDG_SESSION_ID");
    return id;
}

quint64 processStartTime(qint64 pid)
{
    QFile stat(QStringLiteral("/proc/%1/stat").arg(pid));
    if (!stat.open(QIODevice::ReadOnly))
        return 0;
    const QByteArray data = stat.readAll();
    const int close = data.lastIndexOf(')');
    if (close < 0)
        return 0;
    const QList<QByteArray> fields = data.mid(close + 1).trimmed().split(' ');
    // Field 22 (starttime) is index 19 after the comm field.
    if (fields.size() > 19)
        return fields.at(19).toULongLong();
    return 0;
}

AuthorizerSubject buildSubject()
{
    const QString id = sessionId();
    if (!id.isEmpty()) {
        AuthorizerSubject subject;
        subject.kind = QStringLiteral("unix-session");
        subject.attributes.insert(QStringLiteral("session-id"), id);
        return subject;
    }

    const QByteArray pidEnv = qgetenv("DF_SETTINGS_POLKIT_SUBJECT_PID");
    qint64 pid = pidEnv.isEmpty() ? QCoreApplication::applicationPid()
                                  : pidEnv.toLongLong();
    if (pid <= 0)
        pid = QCoreApplication::applicationPid();

    AuthorizerSubject subject;
    subject.kind = QStringLiteral("unix-process");
    subject.attributes.insert(QStringLiteral("pid"), static_cast<uint>(pid));
    subject.attributes.insert(QStringLiteral("start-time"), processStartTime(pid));
    return subject;
}

} // namespace

SettingsAuthorizer::SettingsAuthorizer(QObject *parent)
    : QObject(parent)
    , m_fixture(qEnvironmentVariable("DF_SETTINGS_AUTH_FIXTURE"))
    , m_busAddress(qEnvironmentVariable("DF_SETTINGS_POLKIT_BUS"))
    , m_service(qEnvironmentVariable("DF_SETTINGS_POLKIT_SERVICE"))
{
    if (m_service.isEmpty())
        m_service = kAuthorityService;
    qDBusRegisterMetaType<AuthorizerSubject>();
    qDBusRegisterMetaType<StringMap>();
}

void SettingsAuthorizer::setFixture(const QString &mode)
{
    m_fixture = mode;
}

void SettingsAuthorizer::check(const QString &requestId, const QString &actionId,
                               const QString &message)
{
    Q_UNUSED(message);

    if (!m_fixture.isEmpty()) {
        // Resolve on the next event-loop turn, like a real prompt, so the pane
        // can show its waiting state first.
        QTimer::singleShot(0, this, [this, requestId, actionId]() {
            if (m_fixture == QLatin1String("allow")) {
                emit finished(requestId, true, QString());
            } else if (m_fixture == QLatin1String("cancel")) {
                emit finished(requestId, false,
                              tr("Authentication was cancelled."));
            } else if (m_fixture == QLatin1String("wrong-password")) {
                emit finished(requestId, false,
                              tr("Authentication failed: the password was incorrect."));
            } else if (m_fixture == QLatin1String("unavailable")) {
                emit finished(requestId, false,
                              tr("The system authentication service is not available."));
            } else {
                emit finished(requestId, false, tr("Authentication was denied."));
            }
        });
        return;
    }

    QDBusConnection bus = QDBusConnection::systemBus();
    QString connectionName;
    if (!m_busAddress.isEmpty()) {
        connectionName = QStringLiteral("dragonfruit_settings_auth_%1")
                             .arg(reinterpret_cast<quintptr>(this));
        bus = QDBusConnection::connectToBus(m_busAddress, connectionName);
    }
    if (!bus.isConnected()) {
        QTimer::singleShot(0, this, [this, requestId]() {
            emit finished(requestId, false,
                          tr("The system authentication service is not available."));
        });
        return;
    }

    const AuthorizerSubject subject = buildSubject();
    const StringMap details;
    QDBusMessage call = QDBusMessage::createMethodCall(
        m_service, kAuthorityPath, kAuthorityInterface,
        QStringLiteral("CheckAuthorization"));
    // (sa{sv}) subject, s action, a{ss} details, u flags, s cancellation id.
    // AllowUserInteraction (0x1) is what raises the shell's agent.
    call << QVariant::fromValue(subject) << actionId
         << QVariant::fromValue(details) << static_cast<uint>(1) << QString();

    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, requestId, connectionName]() {
                const QDBusMessage reply = watcher->reply();
                watcher->deleteLater();
                if (!connectionName.isEmpty())
                    QDBusConnection::disconnectFromBus(connectionName);

                if (reply.type() == QDBusMessage::ErrorMessage) {
                    const QString error = reply.errorMessage();
                    const QString lower = error.toLower();
                    if (lower.contains(QStringLiteral("cancel"))
                            || lower.contains(QStringLiteral("dismiss"))) {
                        emit finished(requestId, false,
                                      tr("Authentication was cancelled."));
                    } else {
                        emit finished(requestId, false,
                                      error.isEmpty()
                                          ? tr("Authentication was denied.")
                                          : error);
                    }
                    return;
                }

                const QVariantList args = reply.arguments();
                const bool authorized = args.value(0).toBool();
                if (authorized) {
                    emit finished(requestId, true, QString());
                    return;
                }

                // `polkit.dismissed` is the authority's "the user closed the
                // prompt" detail; without it a false result is a denial or a
                // failed password, which the pane reports verbatim. The
                // authority's `a{ss}` details arrive either already decoded
                // (the fake authority/tests) or as a QDBusArgument (the real
                // authority), so accept both forms.
                bool dismissed = false;
                if (args.size() >= 3) {
                    const QVariant detailsArg = args.at(2);
                    if (detailsArg.typeId() == QMetaType::QVariantMap) {
                        const QVariantMap details = detailsArg.toMap();
                        dismissed = details.value(QStringLiteral("polkit.dismissed"))
                                        .toString() == QLatin1String("true");
                    } else if (detailsArg.canConvert<QDBusArgument>()) {
                        QDBusArgument argument = detailsArg.value<QDBusArgument>();
                        StringMap details;
                        argument >> details;
                        dismissed = details.values
                                        .value(QStringLiteral("polkit.dismissed"))
                                    == QLatin1String("true");
                    }
                }
                emit finished(requestId, false,
                              dismissed
                                  ? tr("Authentication was cancelled.")
                                  : tr("Authentication was denied."));
            });
}
