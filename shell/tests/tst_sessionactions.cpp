// SPDX-License-Identifier: GPL-3.0-only
// Session action tests (T313): Log Out calls logind's
// `Manager.TerminateSession(s)` with the shell's session id, the menu action
// and the Super+Shift+Q action resolve to the same `log-out` id, and the
// fixture path never reaches the host bus.
//
// Run under a private `dbus-run-session` (see CMakeLists.txt). The fake
// `org.freedesktop.login1` service uses the real wire method; the host logind
// is never contacted.
#include "sessionactions.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusContext>
#include <QFile>
#include <QProcess>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>

namespace {

const QString kLogindService = QStringLiteral("org.freedesktop.login1");
const QString kLogindPath = QStringLiteral("/org/freedesktop/login1");

class FakeLogind : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.login1.Manager")

public:
    int callCount() const { return m_calls; }
    QString lastSessionId() const { return m_lastSessionId; }

public slots:
    void TerminateSession(const QString &sessionId)
    {
        ++m_calls;
        m_lastSessionId = sessionId;
    }

private:
    int m_calls = 0;
    QString m_lastSessionId;
};

bool registerLogind(QDBusConnection &bus, FakeLogind &service)
{
    if (!bus.registerService(kLogindService))
        return false;
    return bus.registerObject(kLogindPath, &service, QDBusConnection::ExportAllSlots);
}

QString busAddress()
{
    return QString::fromLocal8Bit(qgetenv("DBUS_SESSION_BUS_ADDRESS"));
}

QString writeFakeSystemctl(const QString &directory)
{
    const QString path = directory + QStringLiteral("/systemctl");
    const QString log = directory + QStringLiteral("/args");
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return QString();
    file.write(QStringLiteral("#!/bin/sh\nprintf '%s' \"$*\" > '%1'\n")
                   .arg(log).toUtf8());
    file.close();
    QFile::setPermissions(path, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                    | QFileDevice::ExeOwner | QFileDevice::ReadGroup
                                    | QFileDevice::ExeGroup | QFileDevice::ReadOther
                                    | QFileDevice::ExeOther);
    return path;
}

} // namespace

class TestSessionActions : public QObject
{
    Q_OBJECT

private slots:
    void cleanup()
    {
        qunsetenv("DRAGONFRUIT_NO_LOGOUT");
        qunsetenv("XDG_SESSION_ID");
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (bus.isConnected()) {
            bus.unregisterObject(kLogindPath);
            bus.unregisterService(kLogindService);
        }
    }

    void logOutCallsTerminateSessionWithTheSessionId()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        SessionActions actions;
        actions.setBusAddress(busAddress());
        actions.setSessionId(QStringLiteral("c7"));
        QSignalSpy dispatched(&actions, &SessionActions::dispatched);

        QVERIFY(actions.logindAvailable());
        QVERIFY(actions.requestLogout());
        QTRY_COMPARE(service.callCount(), 1);
        QCOMPARE(service.lastSessionId(), QStringLiteral("c7"));
        QCOMPARE(actions.lastMethod(), QStringLiteral("TerminateSession"));
        QCOMPARE(actions.lastSessionId(), QStringLiteral("c7"));
        QVERIFY(!actions.lastWasFixture());
        QCOMPARE(dispatched.count(), 1);
        QCOMPARE(dispatched.at(0).at(0).toString(),
                 QStringLiteral("TerminateSession"));
        QCOMPARE(dispatched.at(0).at(1).toString(), QStringLiteral("c7"));

        bus.unregisterObject(kLogindPath);
        bus.unregisterService(kLogindService);
    }

    void logOutReadsTheSessionIdFromTheEnvironment()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        qputenv("XDG_SESSION_ID", "42");
        SessionActions actions;
        actions.setBusAddress(busAddress());

        QVERIFY(actions.requestLogout());
        QTRY_COMPARE(service.callCount(), 1);
        QCOMPARE(service.lastSessionId(), QStringLiteral("42"));
        QCOMPARE(actions.lastSessionId(), QStringLiteral("42"));

        bus.unregisterObject(kLogindPath);
        bus.unregisterService(kLogindService);
    }

    void missingSessionIdDoesNotCallLogind()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        qunsetenv("XDG_SESSION_ID");
        SessionActions actions;
        actions.setBusAddress(busAddress());
        QSignalSpy failed(&actions, &SessionActions::failed);

        QVERIFY(!actions.requestLogout());
        QTest::qWait(100);
        QCOMPARE(service.callCount(), 0);
        QCOMPARE(failed.count(), 1);

        bus.unregisterObject(kLogindPath);
        bus.unregisterService(kLogindService);
    }

    void theFixtureRecordsTheCallWithoutTouchingTheBus()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        qputenv("DRAGONFRUIT_NO_LOGOUT", "1");
        SessionActions actions;
        actions.setBusAddress(busAddress());
        actions.setSessionId(QStringLiteral("c8"));
        QSignalSpy dispatched(&actions, &SessionActions::dispatched);

        QVERIFY(actions.requestLogout());
        QTest::qWait(100);
        QCOMPARE(service.callCount(), 0);
        QVERIFY(actions.lastWasFixture());
        QCOMPARE(actions.lastMethod(), QStringLiteral("TerminateSession"));
        QCOMPARE(actions.lastSessionId(), QStringLiteral("c8"));
        QCOMPARE(dispatched.count(), 1);

        bus.unregisterObject(kLogindPath);
        bus.unregisterService(kLogindService);
    }

    void absentLogindFallsBackToStoppingTheSessionTarget()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        if (bus.interface()->isServiceRegistered(kLogindService))
            QSKIP("the private bus unexpectedly owns the logind name");

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString systemctl = writeFakeSystemctl(directory.path());
        QVERIFY(!systemctl.isEmpty());
        const QString log = directory.filePath(QStringLiteral("args"));

        SessionActions actions;
        actions.setBusAddress(busAddress());
        actions.setSessionId(QStringLiteral("c9"));
        actions.setSystemctlPath(systemctl);

        QVERIFY(!actions.logindAvailable());
        QVERIFY(actions.unavailableReason().isEmpty());
        QVERIFY(actions.requestLogout());
        QTRY_VERIFY(QFile::exists(log));
        QFile file(log);
        QVERIFY(file.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(file.readAll()),
                 QStringLiteral("--user stop dragonfruit-session.target"));
        QCOMPARE(actions.lastMethod(),
                 QStringLiteral("systemctl --user stop dragonfruit-session.target"));
        QCOMPARE(actions.lastSessionId(), QStringLiteral("c9"));
    }

    void unavailableReasonNamesTheMissingPaths()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        if (bus.interface()->isServiceRegistered(kLogindService))
            QSKIP("the private bus unexpectedly owns the logind name");

        SessionActions actions;
        actions.setBusAddress(busAddress());
        actions.setSessionId(QStringLiteral("c10"));
        actions.setSystemctlPath(QStringLiteral("/nonexistent/systemctl"));

        QVERIFY(!actions.unavailableReason().isEmpty());
        QVERIFY(actions.unavailableReason().contains(QStringLiteral("logind")));
        QVERIFY(!actions.requestLogout());
    }
};

QTEST_GUILESS_MAIN(TestSessionActions)
#include "tst_sessionactions.moc"
