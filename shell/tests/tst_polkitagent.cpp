// SPDX-License-Identifier: MIT
// polkit agent tests (T-13.6): the helper conversation against fake helper
// scripts, the D-Bus agent against a fake authority on the private session bus
// that `dbus-run-session` provides, and — when the host actually runs polkit —
// a real `pkcheck` request that raises the agent. The bus halves skip when no
// bus or no system polkit is present, so the suite is safe on a bus-less host.
#include "polkitagent.h"
#include "polkitsession.h"

#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusVirtualObject>
#include <QDir>
#include <QFile>
#include <QProcess>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QTest>

#include <unistd.h>

namespace {

const QString kAuthorityService = QStringLiteral("org.freedesktop.PolicyKit1");
const QString kAuthorityPath = QStringLiteral("/org/freedesktop/PolicyKit1/Authority");
const QString kAuthorityIface = QStringLiteral("org.freedesktop.PolicyKit1.Authority");
const QString kAgentIface = QStringLiteral("org.freedesktop.PolicyKit1.AuthenticationAgent");
const QString kAgentPath = QStringLiteral("/org/dragonfruit/PolicyKit1/AuthenticationAgent");
const QString kTestAgentService = QStringLiteral("org.dragonfruit.PolkitTestAgent");

// The `a(sa{sv})` identity element, registered as a QtDBus type so a test can
// send a real `BeginAuthentication` call.
struct TestIdentity {
    QString kind;
    QVariantMap attributes;
};

QDBusArgument &operator<<(QDBusArgument &argument, const TestIdentity &identity)
{
    argument.beginStructure();
    argument << identity.kind << identity.attributes;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, TestIdentity &identity)
{
    argument.beginStructure();
    argument >> identity.kind >> identity.attributes;
    argument.endStructure();
    return argument;
}

} // namespace

Q_DECLARE_METATYPE(TestIdentity)
Q_DECLARE_METATYPE(QList<TestIdentity>)

namespace {

// A minimal polkit authority: it accepts the agent registration and records
// the subject, so the test can prove the agent registered what it should.
class FakeAuthority : public QDBusVirtualObject
{
public:
    QString objectPath;
    QString locale;
    QString subjectKind;
    QVariantMap subjectAttributes;
    int registerCount = 0;
    int unregisterCount = 0;
    bool accept = true;

    QString introspect(const QString &) const override { return QString(); }

    bool handleMessage(const QDBusMessage &message, const QDBusConnection &connection) override
    {
        if (message.member() == QLatin1String("RegisterAuthenticationAgent")) {
            const QList<QVariant> args = message.arguments();
            if (args.size() >= 3 && args.at(0).canConvert<QDBusArgument>()) {
                const QDBusArgument argument = args.at(0).value<QDBusArgument>();
                argument.beginStructure();
                argument >> subjectKind >> subjectAttributes;
                argument.endStructure();
            }
            if (args.size() >= 3) {
                locale = args.at(1).toString();
                objectPath = args.at(2).toString();
            }
            ++registerCount;
            if (accept)
                connection.send(message.createReply());
            else
                connection.send(message.createErrorReply(
                    QStringLiteral("org.freedesktop.PolicyKit1.Error.Failed"),
                    QStringLiteral("An authentication agent already exists for the given subject")));
            return true;
        }
        if (message.member() == QLatin1String("UnregisterAuthenticationAgent")) {
            ++unregisterCount;
            connection.send(message.createReply());
            return true;
        }
        return false;
    }
};

// A fake `polkit-agent-helper-1`: argv[1] is the username, stdin is the cookie
// then the response. It speaks the same escaped line protocol the host helper
// does, and accepts the password "secret".
QString writeFakeHelper(const QString &directory)
{
    const QString path = directory + QStringLiteral("/polkit-agent-helper-1");
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return QString();
    file.write("#!/bin/sh\n"
               "read cookie\n"
               "printf 'PAM_TEXT_INFO Signed in as %s\\n' \"$1\"\n"
               "printf 'PAM_PROMPT_ECHO_OFF Password: \\n'\n"
               "read response\n"
               "if [ \"$response\" = \"secret\" ]; then\n"
               "  printf 'SUCCESS\\n'\n"
               "else\n"
               "  printf 'PAM_ERROR_MSG Wrong password\\n'\n"
               "  printf 'FAILURE\\n'\n"
               "fi\n");
    file.close();
    QFile::setPermissions(path,
                          QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner
                              | QFileDevice::ReadGroup | QFileDevice::ExeGroup
                              | QFileDevice::ReadOther | QFileDevice::ExeOther);
    return path;
}

QString sessionBusAddress()
{
    return QString::fromLocal8Bit(qgetenv("DBUS_SESSION_BUS_ADDRESS"));
}

} // namespace

class TestPolkitAgent : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        qDBusRegisterMetaType<TestIdentity>();
        qDBusRegisterMetaType<QList<TestIdentity>>();
        qDBusRegisterMetaType<QMap<QString, QString>>();
        m_uid = static_cast<uint>(::getuid());
        m_username = PolkitAgent::usernameForUid(m_uid);
        QVERIFY(!m_username.isEmpty());
    }

    // --- the helper conversation -------------------------------------------

    void theHelperRelaysPromptsAndSucceeds()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString helper = writeFakeHelper(dir.path());
        QVERIFY(!helper.isEmpty());

        PolkitHelperSession session;
        session.setHelperPath(helper);
        session.setSocketPath(QStringLiteral("/nonexistent"));
        QSignalSpy promptSpy(&session, &PolkitHelperSession::prompt);
        QSignalSpy infoSpy(&session, &PolkitHelperSession::info);
        QSignalSpy completedSpy(&session, &PolkitHelperSession::completed);

        QVERIFY(session.start(QStringLiteral("alice"), QStringLiteral("cookie-1")));
        QVERIFY(session.isRunning());
        QTRY_COMPARE_WITH_TIMEOUT(promptSpy.count(), 1, 5000);
        QCOMPARE(promptSpy.at(0).at(0).toString().trimmed(), QStringLiteral("Password:"));
        QCOMPARE(promptSpy.at(0).at(1).toBool(), false);
        QVERIFY(infoSpy.count() >= 1);

        QVERIFY(session.respond(QStringLiteral("secret")));
        QTRY_COMPARE_WITH_TIMEOUT(completedSpy.count(), 1, 5000);
        QCOMPARE(completedSpy.at(0).at(0).toBool(), true);
        QVERIFY(!session.isRunning());
    }

    void aWrongResponseFails()
    {
        QTemporaryDir dir;
        const QString helper = writeFakeHelper(dir.path());
        QVERIFY(!helper.isEmpty());

        PolkitHelperSession session;
        session.setHelperPath(helper);
        session.setSocketPath(QStringLiteral("/nonexistent"));
        QSignalSpy errorSpy(&session, &PolkitHelperSession::errorMessage);
        QSignalSpy completedSpy(&session, &PolkitHelperSession::completed);

        QVERIFY(session.start(QStringLiteral("alice"), QStringLiteral("cookie-2")));
        QTRY_COMPARE_WITH_TIMEOUT(completedSpy.count(), 0, 300);
        session.respond(QStringLiteral("nope"));
        QTRY_COMPARE_WITH_TIMEOUT(completedSpy.count(), 1, 5000);
        QCOMPARE(completedSpy.at(0).at(0).toBool(), false);
        QTRY_VERIFY(errorSpy.count() >= 1);
        QCOMPARE(errorSpy.at(0).at(0).toString(), QStringLiteral("Wrong password"));
    }

    void cancelCompletesFalse()
    {
        QTemporaryDir dir;
        const QString helper = writeFakeHelper(dir.path());
        QVERIFY(!helper.isEmpty());

        PolkitHelperSession session;
        session.setHelperPath(helper);
        session.setSocketPath(QStringLiteral("/nonexistent"));
        QSignalSpy completedSpy(&session, &PolkitHelperSession::completed);
        QVERIFY(session.start(QStringLiteral("alice"), QStringLiteral("cookie-3")));
        session.cancel();
        QTRY_COMPARE_WITH_TIMEOUT(completedSpy.count(), 1, 5000);
        QCOMPARE(completedSpy.at(0).at(0).toBool(), false);
    }

    void aMissingHelperCannotStart()
    {
        PolkitHelperSession session;
        session.setHelperPath(QStringLiteral("/nonexistent/helper"));
        session.setSocketPath(QStringLiteral("/nonexistent"));
        QVERIFY(!session.start(QStringLiteral("alice"), QStringLiteral("cookie-4")));
        QVERIFY(!session.isRunning());
    }

    // --- the D-Bus agent against a fake authority --------------------------

    void startFailsWithoutAnAuthority()
    {
        if (sessionBusAddress().isEmpty())
            QSKIP("no session bus available (run under dbus-run-session)");

        PolkitAgent agent;
        agent.setBusAddress(sessionBusAddress());
        agent.setServiceName(kTestAgentService);
        QVERIFY(agent.start());
        QTRY_VERIFY(!agent.registrationError().isEmpty());
        QVERIFY(!agent.isRegistered());
    }

    void theAgentRegistersAndPresentsARequest()
    {
        if (sessionBusAddress().isEmpty())
            QSKIP("no session bus available (run under dbus-run-session)");

        const QString connectionName = QStringLiteral("dragonfruit_test_authority");
        QDBusConnection authorityBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
        QVERIFY(authorityBus.isConnected());
        FakeAuthority authority;
        QVERIFY(authorityBus.registerService(kAuthorityService));
        QVERIFY(authorityBus.registerVirtualObject(kAuthorityPath, &authority,
                                                   QDBusConnection::SubPath));

        QTemporaryDir dir;
        const QString helper = writeFakeHelper(dir.path());
        QVERIFY(!helper.isEmpty());

        PolkitAgent agent;
        agent.setBusAddress(sessionBusAddress());
        agent.setServiceName(kTestAgentService);
        agent.setSubjectProcess(QCoreApplication::applicationPid());
        agent.setHelperPath(helper);
        QVERIFY(agent.start());
        QTRY_VERIFY(agent.isRegistered());
        QVERIFY2(agent.registrationError().isEmpty(), qPrintable(agent.registrationError()));
        QCOMPARE(authority.registerCount, 1);
        QCOMPARE(authority.subjectKind, QStringLiteral("unix-process"));
        QVERIFY(authority.subjectAttributes.contains(QStringLiteral("pid")));
        QCOMPARE(authority.objectPath, kAgentPath);

        QSignalSpy startedSpy(&agent, &PolkitAgent::started);
        QSignalSpy finishedSpy(&agent, &PolkitAgent::finished);

        QDBusMessage begin = QDBusMessage::createMethodCall(
            kTestAgentService, kAgentPath, kAgentIface, QStringLiteral("BeginAuthentication"));
        begin << QStringLiteral("org.freedesktop.systemd1.manage-units")
              << QStringLiteral("Authentication is required to manage system services.")
              << QString() << QVariant::fromValue(QMap<QString, QString>{
                     { QStringLiteral("polkit.caller-pid"),
                       QString::number(QCoreApplication::applicationPid()) } })
              << QStringLiteral("cookie-abc")
              << QVariant::fromValue(QList<TestIdentity>{
                     TestIdentity{ QStringLiteral("unix-user"),
                                   { { QStringLiteral("uid"), m_uid } } } });
        QDBusConnection session = QDBusConnection::sessionBus();
        QVERIFY(session.interface()->isServiceRegistered(kTestAgentService));
        session.call(begin, QDBus::NoBlock);

        QTRY_COMPARE_WITH_TIMEOUT(startedSpy.count(), 1, 5000);
        QVERIFY(agent.isActive());
        QCOMPARE(agent.actionId(), QStringLiteral("org.freedesktop.systemd1.manage-units"));
        QCOMPARE(agent.identityLabel(), m_username);
        QVERIFY(agent.prompt().startsWith(QStringLiteral("Password")));
        QVERIFY(!agent.details().isEmpty());

        agent.respond(QStringLiteral("secret"));
        QTRY_COMPARE_WITH_TIMEOUT(finishedSpy.count(), 1, 5000);
        QCOMPARE(finishedSpy.at(0).at(0).toBool(), true);
        QVERIFY(!agent.isActive());

        // A second request cancelled through `CancelAuthentication`.
        QDBusMessage cancelBegin = QDBusMessage::createMethodCall(
            kTestAgentService, kAgentPath, kAgentIface, QStringLiteral("BeginAuthentication"));
        cancelBegin << QStringLiteral("org.freedesktop.systemd1.manage-units")
                    << QStringLiteral("Authentication is required.")
                    << QString() << QVariant::fromValue(QMap<QString, QString>())
                    << QStringLiteral("cookie-def")
                    << QVariant::fromValue(QList<TestIdentity>{
                           TestIdentity{ QStringLiteral("unix-user"),
                                         { { QStringLiteral("uid"), m_uid } } } });
        QDBusConnection::sessionBus().call(cancelBegin, QDBus::NoBlock);
        QTRY_COMPARE_WITH_TIMEOUT(startedSpy.count(), 2, 5000);
        QVERIFY(agent.isActive());
        agent.cancel();
        QTRY_COMPARE_WITH_TIMEOUT(finishedSpy.count(), 2, 5000);
        QCOMPARE(finishedSpy.at(1).at(0).toBool(), false);

        authorityBus.unregisterObject(kAuthorityPath);
        authorityBus.unregisterService(kAuthorityService);
        QDBusConnection::disconnectFromBus(connectionName);
    }

    // A registration the authority refuses (another agent already serves the
    // subject) is a degraded state, not a crash.
    void aRefusedRegistrationDegrades()
    {
        if (sessionBusAddress().isEmpty())
            QSKIP("no session bus available (run under dbus-run-session)");

        const QString connectionName = QStringLiteral("dragonfruit_test_authority_refuse");
        QDBusConnection authorityBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
        QVERIFY(authorityBus.isConnected());
        FakeAuthority authority;
        authority.accept = false;
        QVERIFY(authorityBus.registerService(kAuthorityService));
        QVERIFY(authorityBus.registerVirtualObject(kAuthorityPath, &authority,
                                                   QDBusConnection::SubPath));

        PolkitAgent agent;
        agent.setBusAddress(sessionBusAddress());
        agent.setServiceName(kTestAgentService);
        QVERIFY(agent.start());
        QTRY_VERIFY(!agent.registrationError().isEmpty());
        QVERIFY(!agent.isRegistered());
        QVERIFY(agent.registrationError().contains(QStringLiteral("already exists"),
                                                   Qt::CaseInsensitive));

        authorityBus.unregisterObject(kAuthorityPath);
        authorityBus.unregisterService(kAuthorityService);
        QDBusConnection::disconnectFromBus(connectionName);
    }

    void theFixtureResolvesLocally()
    {
        PolkitAgent agent;
        QSignalSpy startedSpy(&agent, &PolkitAgent::started);
        QSignalSpy finishedSpy(&agent, &PolkitAgent::finished);
        agent.presentLocal(QStringLiteral("org.example.action"),
                           QStringLiteral("Authentication is required."),
                           QStringLiteral("alice"), {});
        QCOMPARE(startedSpy.count(), 1);
        QVERIFY(agent.isActive());
        QCOMPARE(agent.identityLabel(), QStringLiteral("alice"));
        agent.respond(QStringLiteral("secret"));
        QCOMPARE(finishedSpy.count(), 1);
        QVERIFY(!agent.isActive());
    }

    // --- a real privileged request raises the agent (acceptance) -----------

    void aRealPrivilegedRequestRaisesTheAgent()
    {
        QDBusConnection system = QDBusConnection::systemBus();
        if (!system.isConnected())
            QSKIP("no system bus available");
        QDBusConnectionInterface *iface = system.interface();
        if (!iface || !iface->isServiceRegistered(kAuthorityService))
            QSKIP("the host polkit authority is not running");
        if (QStandardPaths::findExecutable(QStringLiteral("pkcheck")).isEmpty())
            QSKIP("pkcheck is not installed");
        if (PolkitHelperSession::resolveHelperPath().isEmpty())
            QSKIP("polkit-agent-helper-1 is not installed");
        if (QStandardPaths::findExecutable(QStringLiteral("sleep")).isEmpty())
            QSKIP("no subject process available");

        // The subject is a long-lived process we own; registering the agent for
        // it routes the check to us without touching any other session agent.
        QProcess subject;
        subject.start(QStringLiteral("sleep"), { QStringLiteral("60") });
        QVERIFY(subject.waitForStarted(5000));

        PolkitAgent agent;
        agent.setSubjectProcess(subject.processId());
        QVERIFY(agent.start());
        QTRY_VERIFY_WITH_TIMEOUT(agent.isRegistered(), 10000);

        QSignalSpy startedSpy(&agent, &PolkitAgent::started);
        QProcess check;
        check.start(QStringLiteral("pkcheck"),
                    { QStringLiteral("--action-id"),
                      QStringLiteral("org.freedesktop.systemd1.manage-units"),
                      QStringLiteral("--process"), QString::number(subject.processId()),
                      QStringLiteral("--allow-user-interaction") });
        QVERIFY(check.waitForStarted(5000));

        QTRY_COMPARE_WITH_TIMEOUT(startedSpy.count(), 1, 15000);
        QVERIFY(agent.isActive());
        QCOMPARE(agent.actionId(), QStringLiteral("org.freedesktop.systemd1.manage-units"));
        // The identity polkit offers is the session user.
        QCOMPARE(agent.identityLabel(), m_username);

        agent.cancel();
        check.waitForFinished(5000);
        subject.kill();
        subject.waitForFinished(2000);
    }

private:
    uint m_uid = 0;
    QString m_username;
};

QTEST_GUILESS_MAIN(TestPolkitAgent)
#include "tst_polkitagent.moc"